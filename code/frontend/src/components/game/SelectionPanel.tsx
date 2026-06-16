import { Card } from '@/components/shared/Card';
import { GamePhase, type PendingSelection, type PermanentInfo } from '@/types/game';
import { SELECTION } from '@/utils/constants';
import { isSourceSelectAction, sourceSelectionCards } from '@/utils/sourceSelection';

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
  onAction: (actionId: number) => void;
  localPlayer: number;
  /** Right-click a card in the panel to open the enlarged card detail. */
  onInspectCard?: (cardId: string) => void;
}

/** Phases where this panel should auto-open. Trash selections (single AND
 *  capped multi) are owned by `TrashSelectModal`, not this panel. */
const PANEL_PHASES = new Set<GamePhase>([
  GamePhase.SelectHand,
  GamePhase.SelectSecurity,
  GamePhase.SelectEffectChoice,
]);

interface CardEntry {
  cardId: string;
  actionId: number;
  isValid: boolean;
  label?: string;
  /** Render a face-down card back (used for hidden security positions). */
  faceDown?: boolean;
}

export function SelectionPanel({
  currentPhase,
  pendingSelection,
  actionMask,
  handIds,
  securityCount,
  opponentSecurityCount,
  battleArea,
  onAction,
  localPlayer,
  onInspectCard,
}: SelectionPanelProps) {
  // Only show for specific selection phases where the local player is selecting
  if (!pendingSelection) return null;
  if (pendingSelection.selectingPlayer !== localPlayer) return null;

  // Keyword prompt is handled by KeywordPromptDialog instead
  if (pendingSelection.keywordPrompt) return null;

  // Source-card (digivolution material) picks are identified by their
  // SOURCE_SELECT-range action ids, NOT a dedicated phase: `select_material`
  // runs in SelectMaterial (shared with the board-driven DNA pick) and
  // SelectSource. Gate the modal on the id range so it opens for source picks
  // but stays out of the DNA flow (which is board-clicked by raw field index).
  const isSourceSelect = pendingSelection.validIndices.some(isSourceSelectAction);
  if (!PANEL_PHASES.has(currentPhase) && !isSourceSelect) return null;

  const isEffectChoice = currentPhase === GamePhase.SelectEffectChoice;
  let title = pendingSelection.prompt || (isEffectChoice
    ? 'Multiple effects are triggered. Choose which effect to process.'
    : 'Select a card');
  let cards: CardEntry[] = [];

  if (currentPhase === GamePhase.SelectHand) {
    cards = handIds.map((cardId, i) => ({
      cardId,
      actionId: SELECTION.HAND_START + i,
      isValid: actionMask[SELECTION.HAND_START + i] === 1,
    }));
  } else if (currentPhase === GamePhase.SelectSecurity) {
    // Security is face-down, so we render positional placeholders (one per
    // card in the targeted stack) rather than card faces. The engine encodes
    // own-security selections as action ids 40-49 and opponent-security as
    // 50-59 (disambiguated by `zoneOwner`). Picking the wrong base/stack made
    // opponent-security prompts (e.g. Styracomon) show an empty, unselectable
    // modal.
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
    // Engine uses HAND_EFFECT_START (30+) for effect-choice action IDs,
    // NOT the frontend's `EFFECT_CHOICE_START` (1000+). The old fallback
    // scanned the wrong range and rendered nothing. Prefer the enriched
    // `effectChoices` from the engine (it carries the actual action_id);
    // fall back to scanning `pendingSelection.validIndices` if no labels.
    if (pendingSelection.effectChoices && pendingSelection.effectChoices.length > 0) {
      cards = pendingSelection.effectChoices.map((choice) => {
        const actionId =
          choice.actionId ?? SELECTION.EFFECT_CHOICE_START + choice.index;
        return {
          cardId: choice.cardId,
          actionId,
          isValid: actionMask[actionId] === 1,
          label: choice.label,
        };
      });
    } else {
      cards = pendingSelection.validIndices.map((actionId, i) => ({
        cardId: `effect-${i}`,
        actionId,
        isValid: actionMask[actionId] === 1,
        label: `Effect ${i + 1}`,
      }));
    }
  } else if (isSourceSelect) {
    // Pick a specific digivolution card from under one of your Digimon
    // (`select_material`). Each SOURCE_SELECT id resolves to the non-top
    // source card the engine would act on.
    cards = sourceSelectionCards(pendingSelection.validIndices, battleArea).map((t) => ({
      cardId: t.cardId,
      actionId: t.actionId,
      isValid: actionMask[t.actionId] === 1,
    }));
  }

  const canDecline = actionMask[SELECTION.DECLINE] === 1;
  const validCount = cards.filter((c) => c.isValid).length;

  // Right-click a card to open the enlarged detail (DCGO parity). Synthetic
  // ids (`effect-N` fallbacks with no resolved source card) have no image.
  const inspectOnRightClick = (cardId: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    if (!cardId.startsWith('effect-')) onInspectCard?.(cardId);
  };

  if (cards.length === 0 && !canDecline) return null;

  return (
    <div
      data-testid="selection-panel"
      className="fixed inset-0 z-40 flex items-center justify-center"
    >
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" />

      {/* Modal */}
      <div className="relative bg-slate-800/95 rounded-xl shadow-2xl max-w-4xl w-full mx-4 max-h-[85vh] flex flex-col border border-slate-600/50">
        {/* Header with gradient */}
        <div className="bg-gradient-to-b from-slate-700/80 to-transparent px-5 py-4 border-b border-slate-600/50">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-base font-semibold text-gray-100">{title}</h2>
              {!isEffectChoice && validCount > 0 && (
                <span className="text-xs text-slate-400 mt-0.5">
                  {validCount} of {cards.length} selectable
                </span>
              )}
            </div>
            {canDecline && (
              <button
                data-testid="selection-decline"
                onClick={() => onAction(SELECTION.DECLINE)}
                className="px-4 py-2 bg-slate-600 hover:bg-slate-500 text-white text-sm font-medium rounded-lg transition-colors"
              >
                Decline
              </button>
            )}
          </div>
        </div>

        {/* Content area */}
        <div className="overflow-y-auto p-5">
          {isEffectChoice ? (
            /* Effect choice: show cards with labels */
            <div className="flex flex-wrap gap-4 justify-center">
              {cards.map((entry) => (
                <button
                  key={entry.actionId}
                  onClick={() => entry.isValid ? onAction(entry.actionId) : undefined}
                  className={`flex flex-col items-center gap-2 p-3 rounded-lg border-2 transition-all ${
                    entry.isValid
                      ? 'border-cyan-500/50 hover:border-cyan-400 hover:shadow-[0_0_16px_rgba(34,211,238,0.3)] cursor-pointer bg-slate-700/50 hover:bg-slate-600/50'
                      : 'border-slate-700 opacity-40 cursor-not-allowed'
                  }`}
                >
                  <Card
                    cardId={entry.cardId}
                    size="md"
                    onContextMenu={inspectOnRightClick(entry.cardId)}
                  />
                  {entry.label && (
                    <span className="text-xs text-slate-200 text-center max-w-[120px] leading-tight">
                      {entry.label}
                    </span>
                  )}
                </button>
              ))}
            </div>
          ) : (
            /* Card selection: grid layout */
            <div className="grid grid-cols-5 gap-3 justify-items-center">
              {cards.map((entry) => (
                <div
                  key={`${entry.cardId}-${entry.actionId}`}
                  className={`transition-all ${
                    entry.isValid
                      ? 'cursor-pointer hover:scale-105'
                      : 'cursor-not-allowed'
                  }`}
                >
                  <Card
                    cardId={entry.cardId}
                    size="md"
                    faceDown={entry.faceDown}
                    highlighted={entry.isValid}
                    dimmed={!entry.isValid}
                    onClick={entry.isValid ? () => onAction(entry.actionId) : undefined}
                    onContextMenu={
                      entry.faceDown ? undefined : inspectOnRightClick(entry.cardId)
                    }
                  />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
