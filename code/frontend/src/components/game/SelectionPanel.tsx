import './decisions/decisions.css';
import type { CountChoice } from '@/types/countChoice';
import { Card } from '@/components/shared/Card';
import { formatEffect } from '@/utils/formatCardText';
import { GamePhase, type PendingSelection, type PermanentInfo } from '@/types/game';
import { SELECTION } from '@/utils/constants';
import {
  isSourceSelectAction,
  sourceSelectionCards,
  sourceSelectionHosts,
} from '@/utils/sourceSelection';
import { usePeekStore } from '@/stores/peekStore';
import { cardZoneSelectionTiles, isCardZoneSelection } from '@/utils/cardZoneSelection';

interface SelectionPanelProps {
  currentPhase: GamePhase;
  pendingSelection: PendingSelection | null;
  actionMask: number[];
  /** The local player's hand card IDs */
  handIds: string[];
  /** Number of cards in the local player's security stack. Security is
   *  face-down, so selection is positional (by count), not by card id. */
  securityCount: number;
  /** Number of cards in the opponent's security stack. Security selections can
   *  target the OPPONENT's security (e.g. BT24-018 Styracomon "trash 1 of your
   *  opponent's security") — `pendingSelection.zoneOwner` says which side the
   *  action ids index into. */
  opponentSecurityCount: number;
  /** The local player's battle-area permanents (for source-card picks). */
  battleArea: PermanentInfo[];
  /** The opponent's battle-area permanents — source picks whose `zoneOwner`
   *  is the opponent (BT26-060 ordering their returned stacked cards). */
  opponentBattleArea?: PermanentInfo[];
  /** Trash card ids, for mixed-zone picks (hand + trash). */
  trashIds?: string[];
  opponentTrashIds?: string[];
  /** Currently revealed cards from deck search / reveal effects. */
  revealedCards?: { cardId: string; owner: number }[];
  /** Optional Step-2 host filter for two-step multi-host SOURCE_SELECT. */
  hostFilterSlot?: number | null;
  /** Callback to return to Step 1 (choose another host Digimon on board). */
  onChangeHost?: () => void;
  /** Optional callback to switch back to direct-in-hand picking when opened from Grid View. */
  onPickInHand?: () => void;
  onAction: (actionId: number) => void;
  localPlayer: number;
  /** Right-click a card in the panel to open the enlarged card detail. */
  onInspectCard?: (cardId: string) => void;
}

/** Phases where this panel should auto-open. Trash selections (single AND
 *  capped multi) are owned by `TrashSelectModal`, not this panel. */
const PANEL_PHASES = new Set<GamePhase>([
  GamePhase.SelectHand,
  GamePhase.SelectReveal,
  GamePhase.SelectSecurity,
  GamePhase.SelectEffectChoice,
]);

interface CardEntry {
  countChoice?: CountChoice | null;
  cardId: string;
  cardName?: string;
  actionId: number;
  isValid: boolean;
  label?: string;
  /** Render a face-down card back (used for hidden security positions). */
  faceDown?: boolean;
}

/**
 * Unified DecisionGalleryModal (exported as `SelectionPanel` for compatibility)
 * handling owned `cardChoice` pages, Security picks, Source-card stack picks,
 * mixed-zone union/budgeted picks, count choices, and Hand Grid View fallback.
 */
export function SelectionPanel({
  currentPhase,
  pendingSelection,
  actionMask,
  handIds,
  securityCount,
  opponentSecurityCount,
  battleArea,
  opponentBattleArea = [],
  trashIds = [],
  opponentTrashIds = [],
  revealedCards = [],
  hostFilterSlot = null,
  onChangeHost,
  onPickInHand,
  onAction,
  localPlayer,
  onInspectCard,
}: SelectionPanelProps) {
  const peeking = usePeekStore((s) => s.peeking);
  const togglePeek = usePeekStore((s) => s.toggle);

  // Only show for specific selection phases where the local player is selecting
  if (!pendingSelection) return null;
  if (pendingSelection.selectingPlayer !== localPlayer) return null;

  // Keyword prompt is handled by docked rail / KeywordPromptDialog instead
  if (pendingSelection.keywordPrompt) return null;

  const isSourceSelect = pendingSelection.validIndices.some(isSourceSelectAction);
  const zoneCtx = {
    localPlayer,
    ownHandIds: handIds,
    ownTrashIds: trashIds,
    opponentTrashIds,
    ownBattleArea: battleArea,
    opponentBattleArea,
  };
  const isCardZone =
    !PANEL_PHASES.has(currentPhase) && isCardZoneSelection(pendingSelection, zoneCtx);
  const cardChoice = pendingSelection.cardChoice;
  if (!PANEL_PHASES.has(currentPhase) && !isSourceSelect && !isCardZone && !cardChoice) return null;

  const sourceArea =
    pendingSelection.zoneOwner != null && pendingSelection.zoneOwner !== localPlayer
      ? opponentBattleArea
      : battleArea;

  const isEffectChoice = currentPhase === GamePhase.SelectEffectChoice;
  const isRevealSelection = currentPhase === GamePhase.SelectReveal && !cardChoice;
  const validActionIds = new Set(pendingSelection.validIndices);
  const canDecline = actionMask[SELECTION.DECLINE] === 1;
  const promptLower = (pendingSelection.prompt || '').toLowerCase();
  let title =
    pendingSelection.prompt ||
    (isEffectChoice
      ? 'Choose 1 option to activate'
      : isRevealSelection
        ? 'Select a card from the revealed cards'
        : 'Select a card');
  let subtitleNote: string | null = null;
  let cards: CardEntry[] = [];

  if (cardChoice) {
    cards = cardChoice.rows
      .filter((row) => row.operation === 'commit_card' || row.operation === 'inspect_card')
      .map((row) => ({
        cardId: row.identity_visible ? row.card_id ?? '' : '',
        actionId: row.action_id,
        isValid: validActionIds.has(row.action_id) && actionMask[row.action_id] === 1,
        faceDown: !row.identity_visible,
        label: `${
          row.zone === 'battle_top'
            ? 'Battle card'
            : row.zone === 'security'
              ? 'Security'
              : row.zone === 'hand'
                ? 'Hand'
                : row.zone === 'trash'
                  ? 'Trash'
                  : row.zone === 'breeding_source'
                    ? 'Breeding source'
                    : 'Digivolution card'
        } ${row.position == null ? '' : (BigInt(row.position) + 1n).toString()}`,
      }));
  } else if (isRevealSelection) {
    cards = revealedCards.map((rev, i) => {
      const actionId = SELECTION.REVEALED_START + i;
      const isValid = validActionIds.has(actionId) || actionMask[actionId] === 1;
      return {
        cardId: rev.cardId,
        actionId,
        isValid,
        label: isValid ? 'Selectable' : 'Not Eligible',
      };
    });
  } else if (currentPhase === GamePhase.SelectHand) {
    cards = handIds.map((cardId, i) => ({
      cardId,
      actionId: SELECTION.HAND_START + i,
      isValid: validActionIds.has(SELECTION.HAND_START + i),
      label: 'Hand',
    }));
  } else if (currentPhase === GamePhase.SelectSecurity) {
    const targetingOpponent =
      pendingSelection.zoneOwner != null && pendingSelection.zoneOwner !== localPlayer;
    const base = targetingOpponent
      ? SELECTION.ENEMY_SECURITY_START
      : SELECTION.OWN_SECURITY_START;
    const count = targetingOpponent ? opponentSecurityCount : securityCount;
    cards = Array.from({ length: count }, (_, i) => ({
      cardId: '',
      actionId: base + i,
      isValid: actionMask[base + i] === 1,
      faceDown: true,
      label: `Security ${i + 1}`,
    }));
  } else if (isEffectChoice) {
    if (pendingSelection.effectChoices && pendingSelection.effectChoices.length > 0) {
      cards = pendingSelection.effectChoices.map((choice) => {
        const actionId = choice.actionId ?? SELECTION.EFFECT_CHOICE_START + choice.index;
        return {
          cardId: choice.cardId,
          cardName: choice.cardName,
          actionId,
          isValid: actionMask[actionId] === 1,
          label: choice.label,
          countChoice: choice.countChoice,
        };
      });
    } else {
      cards = pendingSelection.validIndices.map((actionId, i) => ({
        cardId: `effect-${i}`,
        actionId,
        isValid: actionMask[actionId] === 1,
        label: `Option ${i + 1}`,
      }));
    }
  } else if (isSourceSelect && hostFilterSlot != null) {
    const groups = sourceSelectionHosts(pendingSelection.validIndices, sourceArea);
    const chosenHost = groups.find((g) => g.fieldIndex === hostFilterSlot);
    if (chosenHost) {
      subtitleNote = chosenHost.hostCardName
        ? `Digivolution cards under ${chosenHost.hostCardName}`
        : `Digivolution cards in slot ${hostFilterSlot + 1}`;
      cards = chosenHost.tiles.map((t) => ({
        cardId: t.cardId,
        actionId: t.actionId,
        isValid: validActionIds.has(t.actionId) || actionMask[t.actionId] === 1,
        label: 'Digivolution',
      }));
    }
  } else if (isCardZone) {
    cards = (cardZoneSelectionTiles(pendingSelection, zoneCtx) ?? []).map((t) => ({
      cardId: t.cardId,
      actionId: t.actionId,
      isValid: validActionIds.has(t.actionId),
      label:
        t.zone === 'hand'
          ? 'Hand'
          : t.zone === 'trash'
            ? 'Trash'
            : 'Digivolution',
    }));
  } else if (isSourceSelect) {
    cards = sourceSelectionCards(pendingSelection.validIndices, sourceArea).map((t) => ({
      cardId: t.cardId,
      actionId: t.actionId,
      isValid: actionMask[t.actionId] === 1,
      label: 'Digivolution',
    }));
  }

  const validCount = cards.filter((c) => c.isValid).length;
  const hasCountChoice = cards.some((c) => Boolean(c.countChoice));
  const isExclusiveEffectChoice = isEffectChoice && !cardChoice && !hasCountChoice;
  const hasExplicitDeclineOption = cards.some((c) => c.actionId === SELECTION.DECLINE);

  const modalEyebrow = isRevealSelection
    ? promptLower.includes('bottom')
      ? 'Revealed Cards · Return Remaining to Bottom of Deck'
      : promptLower.includes('top')
        ? 'Revealed Cards · Return to Top of Deck'
        : 'Revealed From Deck · Select Card'
    : pendingSelection.kind === 'Material' && cardChoice
      ? 'Digi-Xros Material Selection'
      : isExclusiveEffectChoice
        ? pendingSelection.kind === 'PlayOrder'
          ? 'Match Setup · Play Order'
          : canDecline
            ? 'Choose One Effect (Optional)'
            : 'Choose One Effect'
        : null;

  if (isExclusiveEffectChoice && !subtitleNote) {
    subtitleNote = canDecline
      ? 'Pick one option below — only your chosen option will activate, or decline to skip'
      : 'Pick one option below — only your chosen option will activate';
  }

  const inspectOnRightClick = (cardId: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    if (cardId && !cardId.startsWith('effect-')) onInspectCard?.(cardId);
  };

  if (
    cards.length === 0 &&
    !canDecline &&
    !cardChoice?.rows.some(
      (row) =>
        row.operation === 'refresh_page' ||
        row.operation === 'previous_page' ||
        row.operation === 'next_page',
    )
  ) {
    return null;
  }

  const declineLabel =
    cardChoice?.inspection_only ||
    cardChoice?.rows.some((row) => row.operation === 'finish_impossible')
      ? 'Done'
      : 'Decline';

  return (
    <>
      <div
        data-testid="selection-panel"
        className={`hs-modal-backdrop ${peeking ? 'is-peeking' : ''}`}
      >
        <div className={`hs-modal-sheet ${isRevealSelection ? 'max-w-[920px]' : ''}`}>
          {/* Header */}
          <div className="hs-modal-sheet__header">
            <div>
              {modalEyebrow && <div className="hs-decision-rail__eyebrow">{modalEyebrow}</div>}
              <h2 className="hs-modal-sheet__title">{title}</h2>
              <div className="hs-modal-sheet__subtitle">
                {subtitleNote && <span>{subtitleNote} · </span>}
                {!isEffectChoice && validCount > 0 && (
                  <span>
                    {validCount} of {cards.length} selectable
                  </span>
                )}
                {pendingSelection.sourceCardName && (
                  <span> · Source: {pendingSelection.sourceCardName}</span>
                )}
              </div>
            </div>
            <div className="flex items-center gap-2">
              {onChangeHost && (
                <button
                  type="button"
                  data-testid="source-change-host"
                  onClick={onChangeHost}
                  className="ds-btn ds-btn--ghost"
                >
                  ← Change Digimon
                </button>
              )}
              {onPickInHand && (
                <button
                  type="button"
                  data-testid="hand-pick-inline"
                  onClick={onPickInHand}
                  className="ds-btn ds-btn--ghost"
                >
                  Pick in Hand
                </button>
              )}
              {canDecline && (
                <button
                  type="button"
                  data-testid="selection-decline"
                  onClick={() => onAction(SELECTION.DECLINE)}
                  className="ds-btn ds-btn--ghost"
                >
                  {declineLabel}
                </button>
              )}
            </div>
          </div>

          {/* Content area */}
          <div className="hs-modal-sheet__body">
            {cardChoice && (
              <div
                className="flex items-center justify-center gap-3 mb-4"
                aria-label="Card choice pages"
              >
                {cardChoice.rows
                  .filter(
                    (row) =>
                      row.operation === 'previous_page' ||
                      row.operation === 'next_page' ||
                      row.operation === 'refresh_page',
                  )
                  .map((row) => (
                    <button
                      key={row.action_id}
                      type="button"
                      className="ds-btn ds-btn--ghost"
                      disabled={
                        !validActionIds.has(row.action_id) ||
                        actionMask[row.action_id] !== 1
                      }
                      onClick={() => onAction(row.action_id)}
                    >
                      {row.operation === 'refresh_page'
                        ? 'Refresh choices'
                        : row.operation === 'previous_page'
                          ? 'Previous page'
                          : 'Next page'}
                    </button>
                  ))}
                <span className="hs-zone-pill">
                  Page {(BigInt(cardChoice.page_index) + 1n).toString()} of{' '}
                  {cardChoice.page_count}
                </span>
              </div>
            )}

            {isEffectChoice && !cardChoice ? (
              hasCountChoice ? (
                <div className="hs-card-gallery">
                  {cards.map((entry) => (
                    <button
                      key={entry.actionId}
                      type="button"
                      disabled={!entry.isValid}
                      onClick={() => (entry.isValid ? onAction(entry.actionId) : undefined)}
                      className="hs-card-gallery__item"
                    >
                      {entry.countChoice ? (
                        <span className="ds-btn ds-btn--primary">
                          {entry.countChoice.operation === 'commit_count'
                            ? `Count ${entry.countChoice.declared_count} (payable ${entry.countChoice.payable_count})`
                            : entry.countChoice.operation === 'previous_page'
                              ? 'Previous page'
                              : 'Next page'}
                        </span>
                      ) : (
                        <Card
                          cardId={entry.cardId}
                          size="md"
                          highlighted={entry.isValid}
                          dimmed={!entry.isValid}
                          onContextMenu={inspectOnRightClick(entry.cardId)}
                        />
                      )}
                      {entry.label && !entry.countChoice && (
                        <span className="hs-card-gallery__label">{entry.label}</span>
                      )}
                    </button>
                  ))}
                </div>
              ) : (
                <div className="hs-choice-grid" data-testid="exclusive-effect-choices">
                  {cards.map((entry, idx) => {
                    const hasRealCard =
                      Boolean(entry.cardId) && !entry.cardId.startsWith('effect-');
                    const isDeclineEntry = entry.actionId === SELECTION.DECLINE;
                    const displaySource =
                      hasRealCard &&
                      entry.cardName &&
                      entry.cardName !== entry.label &&
                      entry.cardName !== entry.cardId
                        ? `${entry.cardName} (${entry.cardId})`
                        : hasRealCard
                          ? entry.cardName || entry.cardId
                          : `Option ${idx + 1}`;
                    return (
                      <button
                        key={entry.actionId}
                        type="button"
                        disabled={!entry.isValid}
                        onClick={() => (entry.isValid ? onAction(entry.actionId) : undefined)}
                        className={`hs-choice-option${isDeclineEntry ? ' hs-choice-option--decline' : ''}`}
                        onContextMenu={
                          hasRealCard ? inspectOnRightClick(entry.cardId) : undefined
                        }
                      >
                        {hasRealCard && (
                          <div className="hs-choice-option__art">
                            <Card
                              cardId={entry.cardId}
                              size="sm"
                              highlighted={entry.isValid}
                              dimmed={!entry.isValid}
                              onContextMenu={inspectOnRightClick(entry.cardId)}
                            />
                          </div>
                        )}
                        <div className="hs-choice-option__body">
                          <div className="hs-choice-option__source">
                            {displaySource}
                          </div>
                          {entry.label && (
                            <div className="hs-choice-option__text card-effect-text">
                              {formatEffect(entry.label)}
                            </div>
                          )}
                        </div>
                        <span className="hs-choice-option__cta">
                          {isDeclineEntry ? 'Skip' : 'Choose'}
                        </span>
                      </button>
                    );
                  })}
                </div>
              )
            ) : (
              <div
                className={`hs-card-gallery ${cards.length > 6 ? 'hs-card-gallery--scroll' : ''}`}
              >
                {cards.map((entry) => (
                  <button
                    type="button"
                    key={`${entry.cardId}-${entry.actionId}`}
                    disabled={!entry.isValid}
                    onClick={() => entry.isValid && onAction(entry.actionId)}
                    className="hs-card-gallery__item"
                  >
                    <Card
                      cardId={entry.cardId}
                      size={isRevealSelection ? 'lg' : 'md'}
                      faceDown={entry.faceDown}
                      highlighted={entry.isValid}
                      dimmed={!entry.isValid}
                      onContextMenu={
                        entry.faceDown ? undefined : inspectOnRightClick(entry.cardId)
                      }
                    />
                    {entry.label && (
                      <span className="hs-zone-pill">{entry.label}</span>
                    )}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Footer with built-in View Board toggle */}
          <div className="hs-modal-sheet__footer">
            <div className="flex items-center gap-2">
              <button
                type="button"
                data-testid="selection-view-board"
                onClick={togglePeek}
                className="ds-btn ds-btn--ghost"
              >
                View Board
              </button>
              <span className="text-[11px] text-[var(--ink-2)]">
                Right-click any card to inspect
              </span>
            </div>
            {isExclusiveEffectChoice && canDecline && !hasExplicitDeclineOption && (
              <button
                type="button"
                data-testid="effect-choice-skip"
                onClick={() => onAction(SELECTION.DECLINE)}
                className="ds-btn ds-btn--ghost"
              >
                Do Not Use
              </button>
            )}
          </div>
        </div>
      </div>

      {peeking && (
        <button
          type="button"
          onClick={togglePeek}
          className="hs-peek-return"
        >
          Return to Selection
        </button>
      )}
    </>
  );
}
