import './decisions.css';
import { useEffect, useState } from 'react';
import { SelectionPanel } from '@/components/game/SelectionPanel';
import { TrashSelectModal } from '@/components/game/TrashSelectModal';
import { BoardDecisionRail } from './BoardDecisionRail';
import { TriggerChooserModal } from './TriggerChooserModal';
import { OrderCardsModal, resolvePermutationCards } from './OrderCardsModal';
import { GamePhase, type PendingSelection, type PermanentInfo } from '@/types/game';
import { ACTION, PHASE_NAMES, SELECTION } from '@/utils/constants';
import { useResolutionPlanStore } from '@/stores/resolutionPlanStore';
import { usePeekStore } from '@/stores/peekStore';
import { isBoardDrivenSelection } from '@/utils/selectionSurface';
import { isSourceSelectAction, sourceSelectionHosts } from '@/utils/sourceSelection';

export interface DecisionRouterProps {
  currentPhase: GamePhase;
  pendingSelection: PendingSelection | null;
  actionMask: number[];
  firstPlayer: number;
  handIds: string[];
  securityCount: number;
  opponentSecurityCount: number;
  battleArea: PermanentInfo[];
  opponentBattleArea: PermanentInfo[];
  trashIds: string[];
  opponentTrashIds: string[];
  revealedCards?: { cardId: string; owner: number }[];
  localPlayer: number;
  isGameOver: boolean;
  isBusy?: boolean;
  /** Step-2 host slot selected on the board during multi-host SOURCE_SELECT. */
  selectedSourceHostSlot?: number | null;
  onSelectSourceHost?: (slot: number | null) => void;
  /** When true, SelectHand opens the centered Grid View instead of the inline hand rail. */
  handGridOverride?: boolean;
  onHandGridOverride?: (showGrid: boolean) => void;
  onAction: (actionId: number) => void;
  onInspectCard?: (cardId: string) => void;
}

/**
 * Unified Aegis-inspired decision router:
 * - Mulligan / Board target / Combat interrupt / Keyword / Single-effect optional -> Docked Side Rail
 * - Direct-in-hand `SelectHand` -> Docked Side Rail (clickable fanned hand + optional "Grid View")
 * - Multi-host `SOURCE_SELECT` -> Two-step SourceHostPicker (Step 1: board host rail -> Step 2: focused stack modal)
 * - `SelectPermutation` -> OrderCardsModal (1, 2, 3 numbering + sequential auto-dispatch)
 * - Multi-trigger `SelectEffectChoice` -> TriggerChooserModal (with client-side resolution plan queue)
 * - Trash picks -> TrashSelectModal
 * - Remaining Galleries / Security / Single-host Source / cardChoice / CountChoice -> SelectionPanel
 */
export function DecisionRouter({
  currentPhase,
  pendingSelection,
  actionMask,
  firstPlayer,
  handIds,
  securityCount,
  opponentSecurityCount,
  battleArea,
  opponentBattleArea,
  trashIds,
  opponentTrashIds,
  revealedCards = [],
  localPlayer,
  isGameOver,
  isBusy = false,
  selectedSourceHostSlot = null,
  onSelectSourceHost,
  handGridOverride = false,
  onHandGridOverride,
  onAction,
  onInspectCard,
}: DecisionRouterProps) {
  const plan = useResolutionPlanStore((s) => s.plan);
  const permutationQueue = useResolutionPlanStore((s) => s.permutationQueue);
  const consumeNextAction = useResolutionPlanStore((s) => s.consumeNextAction);
  const consumeNextPermutation = useResolutionPlanStore((s) => s.consumeNextPermutation);
  const clearPlan = useResolutionPlanStore((s) => s.clearPlan);
  const clearPermutationOrder = useResolutionPlanStore((s) => s.clearPermutationOrder);
  const [showAllSources, setShowAllSources] = useState(false);

  const selectionIdentity = pendingSelection
    ? `${pendingSelection.phase}::${pendingSelection.prompt}::${pendingSelection.validIndices.join(',')}`
    : null;

  useEffect(() => {
    setShowAllSources(false);
    usePeekStore.getState().setPeeking(false);
  }, [selectionIdentity]);

  // Auto-dispatch queued resolution-plan steps once animations settle
  useEffect(() => {
    if (isGameOver || isBusy || !plan) return;
    if (!pendingSelection || pendingSelection.selectingPlayer !== localPlayer) return;
    const nextAction = consumeNextAction(pendingSelection, actionMask, SELECTION.DECLINE);
    if (nextAction !== null) {
      const timer = setTimeout(() => onAction(nextAction), 140);
      return () => clearTimeout(timer);
    }
  }, [actionMask, consumeNextAction, isBusy, isGameOver, localPlayer, onAction, pendingSelection, plan]);

  // Auto-dispatch queued SelectPermutation card order steps sequentially
  useEffect(() => {
    if (isGameOver || isBusy || permutationQueue.length === 0) return;
    if (!pendingSelection || pendingSelection.selectingPlayer !== localPlayer) return;
    const nextAction = consumeNextPermutation(pendingSelection, actionMask);
    if (nextAction !== null) {
      const timer = setTimeout(() => onAction(nextAction), 110);
      return () => clearTimeout(timer);
    }
  }, [actionMask, consumeNextPermutation, isBusy, isGameOver, localPlayer, onAction, pendingSelection, permutationQueue]);

  if (isGameOver) return null;

  // Opening Mulligan rail
  if (currentPhase === GamePhase.Mulligan && !isBusy) {
    const canKeep = actionMask[ACTION.MULLIGAN_KEEP] === 1;
    const canRedraw = actionMask[ACTION.MULLIGAN_REDRAW] === 1;
    if (canKeep || canRedraw) {
      return (
        <BoardDecisionRail
          eyebrow="Opening Hand"
          title={firstPlayer === 0 ? 'You go first — Keep or Mulligan?' : 'You go second — Keep or Mulligan?'}
          clauseText="You may redraw your opening 5-card hand once before security is placed."
        >
          {canKeep && (
            <button
              type="button"
              data-testid="action-keep-hand"
              onClick={() => onAction(ACTION.MULLIGAN_KEEP)}
              className="ds-btn ds-btn--primary flex-1"
            >
              Keep Hand
            </button>
          )}
          {canRedraw && (
            <button
              type="button"
              data-testid="action-mulligan"
              onClick={() => onAction(ACTION.MULLIGAN_REDRAW)}
              className="ds-btn flex-1"
            >
              Mulligan
            </button>
          )}
        </BoardDecisionRail>
      );
    }
  }

  // Opponent is selecting or animating
  if (pendingSelection && pendingSelection.selectingPlayer !== localPlayer) {
    return (
      <div
        data-testid="prompt-bar"
        className="fixed top-14 left-1/2 -translate-x-1/2 z-30 flex items-center gap-2 px-4 py-1.5 rounded-full border border-[var(--line-1)] bg-[var(--surface)] shadow-md text-xs font-semibold text-[var(--ink-1)]"
      >
        <span className="inline-block w-2 h-2 rounded-full bg-[var(--opp)] animate-pulse" />
        Waiting for opponent…
      </div>
    );
  }

  if (!pendingSelection || isBusy) {
    if (plan) {
      return (
        <button
          type="button"
          onClick={clearPlan}
          className="hs-peek-return"
          data-testid="cancel-resolution-plan"
        >
          Cancel Auto-Order ({plan.queue.length} queued)
        </button>
      );
    }
    if (permutationQueue.length > 0) {
      return (
        <button
          type="button"
          onClick={clearPermutationOrder}
          className="hs-peek-return"
          data-testid="cancel-permutation-queue"
        >
          Ordering Cards… ({permutationQueue.length} left)
        </button>
      );
    }
    return null;
  }

  const canDecline = actionMask[SELECTION.DECLINE] === 1;

  // 1. Multi-trigger simultaneous resolution order -> TriggerChooserModal (Aegis resolution plan)
  // Mutually exclusive `EffectChoice` prompts ("choose 1 of the following effects")
  // must NOT use the trigger-ordering modal — they fall through to SelectionPanel's
  // dedicated pick-one choice UI.
  const isTriggerOrder =
    pendingSelection.kind === 'TriggerOrder' ||
    (pendingSelection.kind == null &&
      /which triggered effect|resolution order|pending\)/i.test(pendingSelection.prompt || ''));
  const validEffectChoices = (pendingSelection.effectChoices ?? []).filter(
    (c) => c.actionId != null && actionMask[c.actionId] === 1 && !c.countChoice,
  );
  if (
    currentPhase === GamePhase.SelectEffectChoice &&
    isTriggerOrder &&
    !pendingSelection.cardChoice &&
    validEffectChoices.length >= 2
  ) {
    return (
      <TriggerChooserModal
        title={pendingSelection.prompt || 'Choose effect resolution order'}
        choices={pendingSelection.effectChoices ?? []}
        actionMask={actionMask}
        pendingSelection={pendingSelection}
        waitingTriggers={pendingSelection.waitingTriggers}
        canDecline={canDecline}
        declineActionId={SELECTION.DECLINE}
        onAction={onAction}
        onInspectCard={onInspectCard}
      />
    );
  }

  // 2. SelectPermutation -> OrderCardsModal (numbered 1, 2, 3 deck/stack ordering)
  if (
    currentPhase === GamePhase.SelectPermutation &&
    !pendingSelection.cardChoice &&
    !pendingSelection.keywordPrompt
  ) {
    const permEntries = resolvePermutationCards(pendingSelection, {
      localPlayer,
      handIds,
      trashIds,
      opponentTrashIds,
      battleArea,
      opponentBattleArea,
      revealedCards,
    });
    if (permEntries && permEntries.length >= 2) {
      return (
        <OrderCardsModal
          pendingSelection={pendingSelection}
          entries={permEntries}
          canDecline={canDecline}
          onAction={onAction}
          onInspectCard={onInspectCard}
        />
      );
    }
  }

  // 3. Optional Keyword / Single-effect Yes-No / Combat Interrupt -> Docked Side Rail
  const kw = pendingSelection.keywordPrompt;
  const nonDeclineValid = pendingSelection.validIndices.filter(
    (id) => id !== SELECTION.DECLINE && actionMask[id] === 1,
  );
  const isSingleOptionalEffect =
    currentPhase === GamePhase.SelectEffectChoice &&
    !pendingSelection.cardChoice &&
    canDecline &&
    nonDeclineValid.length === 1;
  const isCombatInterrupt =
    currentPhase === GamePhase.BlockTiming ||
    currentPhase === GamePhase.CounterTiming ||
    currentPhase === GamePhase.AllianceTiming;

  if (kw || isSingleOptionalEffect || isCombatInterrupt) {
    const useId = nonDeclineValid.length === 1 ? nonDeclineValid[0]! : null;
    const kwLabel =
      kw?.keyword ??
      (currentPhase === GamePhase.BlockTiming
        ? 'Blocker'
        : currentPhase === GamePhase.CounterTiming
          ? 'Counter'
          : currentPhase === GamePhase.AllianceTiming
            ? 'Alliance'
            : 'Effect');
    const sourceId =
      kw?.cardId ??
      pendingSelection.sourceCardId ??
      pendingSelection.effectChoices?.[0]?.cardId ??
      null;
    const clause =
      pendingSelection.effectChoices?.[0]?.label ??
      pendingSelection.prompt ??
      `Choose whether to activate ${kwLabel}.`;

    return (
      <BoardDecisionRail
        eyebrow={isCombatInterrupt ? `${kwLabel} Window` : 'Optional Effect'}
        title={
          useId !== null
            ? `Use ${kwLabel}?`
            : pendingSelection.prompt || `Select a ${kwLabel} target on the board`
        }
        sourceCardId={sourceId}
        clauseText={clause}
        tone={isCombatInterrupt ? 'combat' : 'default'}
        onInspectCard={onInspectCard}
      >
        {useId !== null && (
          <button
            type="button"
            data-testid="rail-use-effect"
            onClick={() => onAction(useId)}
            className="ds-btn ds-btn--primary flex-1"
          >
            Use {kwLabel}
          </button>
        )}
        {canDecline && (
          <button
            type="button"
            data-testid="rail-decline"
            onClick={() => onAction(SELECTION.DECLINE)}
            className="ds-btn ds-btn--ghost flex-1"
          >
            Decline
          </button>
        )}
      </BoardDecisionRail>
    );
  }

  // 4. Direct-in-Hand SelectHand Rail (board & fanned hand remain directly clickable!)
  const isDirectHandSelect =
    currentPhase === GamePhase.SelectHand &&
    !pendingSelection.cardChoice &&
    !handGridOverride &&
    handIds.length > 0;

  if (isDirectHandSelect) {
    const validHandCount = pendingSelection.validIndices.filter(
      (id) => id >= SELECTION.HAND_START && id <= SELECTION.HAND_END,
    ).length;
    return (
      <BoardDecisionRail
        eyebrow="Select From Hand"
        title={pendingSelection.prompt || 'Choose a card from your hand'}
        sourceCardId={pendingSelection.sourceCardId}
        clauseText={
          pendingSelection.sourceCardName
            ? `Source: ${
                pendingSelection.sourceCardId &&
                pendingSelection.sourceCardName !== pendingSelection.sourceCardId
                  ? `${pendingSelection.sourceCardName} (${pendingSelection.sourceCardId})`
                  : pendingSelection.sourceCardName
              } — click a highlighted card directly in your hand.`
            : 'Click a highlighted card directly in your hand below.'
        }
        metaText={`${validHandCount} valid card${validHandCount === 1 ? '' : 's'} highlighted in hand`}
        tone="target"
        onInspectCard={onInspectCard}
      >
        {onHandGridOverride && (
          <button
            type="button"
            data-testid="hand-open-grid"
            onClick={() => onHandGridOverride(true)}
            className="ds-btn ds-btn--ghost flex-1"
          >
            Grid View
          </button>
        )}
        {canDecline && (
          <button
            type="button"
            data-testid="selection-decline"
            onClick={() => onAction(SELECTION.DECLINE)}
            className="ds-btn ds-btn--ghost flex-1"
          >
            Decline
          </button>
        )}
      </BoardDecisionRail>
    );
  }

  // 5. Multi-host SOURCE_SELECT -> Two-step SourceHostPicker
  const isPureSourceSelect =
    !pendingSelection.cardChoice &&
    pendingSelection.validIndices.some(isSourceSelectAction) &&
    currentPhase !== GamePhase.SelectPermutation;
  const sourceArea =
    pendingSelection.zoneOwner != null && pendingSelection.zoneOwner !== localPlayer
      ? opponentBattleArea
      : battleArea;
  const hostGroups = isPureSourceSelect
    ? sourceSelectionHosts(pendingSelection.validIndices, sourceArea)
    : [];
  const isMultiHostSource = hostGroups.length >= 2 && !showAllSources;

  if (isMultiHostSource && selectedSourceHostSlot === null) {
    return (
      <BoardDecisionRail
        eyebrow="Step 1 of 2 · Choose Digimon"
        title={pendingSelection.prompt || 'Choose a Digimon to inspect its digivolution cards'}
        sourceCardId={pendingSelection.sourceCardId}
        clauseText="Click one of the highlighted Digimon on the board to open its digivolution stack."
        metaText={`${hostGroups.length} Digimon have valid digivolution cards`}
        tone="target"
        onInspectCard={onInspectCard}
      >
        <button
          type="button"
          data-testid="source-show-all"
          onClick={() => setShowAllSources(true)}
          className="ds-btn ds-btn--ghost flex-1"
        >
          Show All Cards
        </button>
        {canDecline && (
          <button
            type="button"
            data-testid="selection-decline"
            onClick={() => onAction(SELECTION.DECLINE)}
            className="ds-btn ds-btn--ghost flex-1"
          >
            Decline
          </button>
        )}
      </BoardDecisionRail>
    );
  }

  // 6. Board-driven target / DNA / breeding selection -> Docked Side Rail
  if (isBoardDrivenSelection(pendingSelection, currentPhase)) {
    const phaseLabel = PHASE_NAMES[currentPhase] ?? 'Select Target';
    return (
      <BoardDecisionRail
        eyebrow={phaseLabel}
        title={pendingSelection.prompt || 'Select a target on the board'}
        sourceCardId={pendingSelection.sourceCardId}
        clauseText={
          pendingSelection.sourceCardName
            ? `Source: ${
                pendingSelection.sourceCardId &&
                pendingSelection.sourceCardName !== pendingSelection.sourceCardId
                  ? `${pendingSelection.sourceCardName} (${pendingSelection.sourceCardId})`
                  : pendingSelection.sourceCardName
              }`
            : 'Click a highlighted card directly on the board.'
        }
        metaText={
          pendingSelection.isOptional ? 'Optional — you may decline.' : 'Click a highlighted target.'
        }
        tone="target"
        onInspectCard={onInspectCard}
      >
        {canDecline && (
          <button
            type="button"
            data-testid="selection-decline"
            onClick={() => onAction(SELECTION.DECLINE)}
            className="ds-btn ds-btn--ghost flex-1"
          >
            Decline
          </button>
        )}
      </BoardDecisionRail>
    );
  }

  // 7. Trash selections & general gallery modals (Security / Source Step 2 / CardChoice / Union / Hand Grid)
  return (
    <>
      <SelectionPanel
        currentPhase={currentPhase}
        pendingSelection={pendingSelection}
        actionMask={actionMask}
        handIds={handIds}
        securityCount={securityCount}
        opponentSecurityCount={opponentSecurityCount}
        battleArea={battleArea}
        opponentBattleArea={opponentBattleArea}
        trashIds={trashIds}
        opponentTrashIds={opponentTrashIds}
        revealedCards={revealedCards}
        hostFilterSlot={isMultiHostSource ? selectedSourceHostSlot : null}
        onChangeHost={
          isMultiHostSource && selectedSourceHostSlot !== null && onSelectSourceHost
            ? () => onSelectSourceHost(null)
            : undefined
        }
        onPickInHand={
          currentPhase === GamePhase.SelectHand &&
          !pendingSelection.cardChoice &&
          handGridOverride &&
          onHandGridOverride
            ? () => onHandGridOverride(false)
            : undefined
        }
        onAction={onAction}
        localPlayer={localPlayer}
        onInspectCard={onInspectCard}
      />
      <TrashSelectModal
        pendingSelection={pendingSelection}
        actionMask={actionMask}
        ownTrashIds={trashIds}
        opponentTrashIds={opponentTrashIds}
        localPlayer={localPlayer}
        onAction={onAction}
        onInspectCard={onInspectCard}
      />
    </>
  );
}
