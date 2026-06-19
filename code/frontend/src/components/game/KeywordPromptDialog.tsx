import { Card } from '@/components/shared/Card';
import { GamePhase, type PendingSelection } from '@/types/game';
import { SELECTION } from '@/utils/constants';

interface KeywordPromptDialogProps {
  currentPhase: GamePhase;
  pendingSelection: PendingSelection | null;
  actionMask: number[];
  onAction: (actionId: number) => void;
  localPlayer: number;
}

/**
 * DCGO-style "Will you use [Keyword]?" dialog.
 * Shows when an optional keyword triggers (single effect + decline).
 */
export function KeywordPromptDialog({
  currentPhase,
  pendingSelection,
  actionMask,
  onAction,
  localPlayer,
}: KeywordPromptDialogProps) {
  if (!pendingSelection) return null;
  if (pendingSelection.selectingPlayer !== localPlayer) return null;

  // Show for keyword prompts (explicit metadata from backend)
  const kw = pendingSelection.keywordPrompt;

  // Also detect implicit keyword patterns:
  // SelectEffectChoice with exactly 1 valid effect + decline = keyword prompt
  const isImplicitKeyword = !kw
    && currentPhase === GamePhase.SelectEffectChoice
    && actionMask[SELECTION.DECLINE] === 1
    && pendingSelection.validIndices.filter((i) => i !== SELECTION.DECLINE).length === 1;

  // Also handle BlockTiming / CounterTiming as Use/Not use patterns
  const isBlockOrCounter = currentPhase === GamePhase.BlockTiming || currentPhase === GamePhase.CounterTiming;
  const hasDecline = actionMask[SELECTION.DECLINE] === 1;

  if (!kw && !isImplicitKeyword && !(isBlockOrCounter && hasDecline)) return null;

  // Determine the action ID for "Use"
  let useActionId: number | null = null;
  if (kw || isImplicitKeyword) {
    // Find the non-decline valid action
    for (const idx of pendingSelection.validIndices) {
      if (idx !== SELECTION.DECLINE && actionMask[idx] === 1) {
        useActionId = idx;
        break;
      }
    }
  } else if (isBlockOrCounter) {
    // For block/counter, the valid indices are the blocker slots (100-111)
    for (const idx of pendingSelection.validIndices) {
      if (idx !== SELECTION.DECLINE && actionMask[idx] === 1) {
        useActionId = idx;
        break;
      }
    }
    // If multiple blockers, let the main selection handle it
    const validBlockers = pendingSelection.validIndices.filter(
      (i) => i !== SELECTION.DECLINE && actionMask[i] === 1
    );
    if (validBlockers.length > 1) return null;
  }

  if (useActionId === null) return null;

  const keywordName = kw?.keyword
    ?? (isBlockOrCounter
      ? (currentPhase === GamePhase.BlockTiming ? 'Blocker' : 'Counter')
      : 'this effect');

  const cardId = kw?.cardId;
  const cardName = kw?.cardName;

  return (
    <div className="fixed bottom-24 left-4 z-40 w-[280px]">
      <div className="bg-[var(--ib-graphite)] border border-[var(--ib-line)] rounded-xl shadow-2xl backdrop-blur-sm overflow-hidden">
        {/* Header */}
        <div className="bg-gradient-to-r from-[var(--ib-panel-2)] to-transparent px-4 py-2.5 border-b border-[var(--ib-line)]">
          <h3 className="text-sm font-semibold text-[var(--ib-bone)]">
            Will you use <span className="text-[var(--ib-player)]">{keywordName}</span>?
          </h3>
          {cardName && (
            <span className="text-[11px] text-[var(--ib-bone-dd)]">{cardName}</span>
          )}
        </div>

        {/* Card preview */}
        {cardId && (
          <div className="flex justify-center p-3">
            <Card cardId={cardId} size="md" />
          </div>
        )}

        {/* Action buttons */}
        <div className="flex gap-2 p-3 pt-0">
          <button
            onClick={() => onAction(useActionId!)}
            className="flex-1 px-4 py-2.5 bg-amber-600 hover:bg-amber-500 text-white text-sm font-bold rounded-lg transition-colors shadow-lg"
          >
            Use
          </button>
          <button
            onClick={() => onAction(SELECTION.DECLINE)}
            className="flex-1 px-4 py-2.5 bg-blue-700 hover:bg-blue-600 text-white text-sm font-bold rounded-lg transition-colors shadow-lg"
          >
            Not use
          </button>
        </div>
      </div>
    </div>
  );
}
