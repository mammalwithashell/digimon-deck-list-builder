import { useEffect, useRef, useState } from 'react';
import { Card } from '@/components/shared/Card';
import { SELECTION } from '@/utils/constants';
import type { PendingSelection } from '@/types/game';
import {
  parseCountCappedKind,
  trashSelectionMode,
  type CountCappedKind,
} from '@/utils/trashSelection';

interface TrashSelectModalProps {
  pendingSelection: PendingSelection | null;
  actionMask: number[];
  /** The local player's trash card IDs (zone order). */
  ownTrashIds: string[];
  /** The opponent's trash card IDs (public zone). */
  opponentTrashIds: string[];
  localPlayer: number;
  /** Dispatch an engine action. Returns a promise that resolves once the
   *  resulting state has been applied (the deferred multi-select drain
   *  awaits each pick). */
  onAction: (actionId: number) => Promise<void> | void;
  /** Right-click a card to open the enlarged detail. */
  onInspectCard?: (cardId: string) => void;
}

interface TrashCard {
  cardId: string;
  actionId: number;
  /** Legal in the current engine mask (a fresh, un-picked candidate). */
  legalNow: boolean;
}

/**
 * Interactive trash-selection modal — the real selector for every
 * `SelectionKind::Trash` (single) and trash-range `CountCappedMultiSelect`
 * (capped multi) pick, against either player's trash.
 *
 * Card at zone index `i` maps to action `SELECTION.TRASH_START + i` (the
 * range mirrors the engine's `TRASH_EFFECT_START`). Selectability is gated on
 * the engine action mask — never a hardcoded assumption — so a future
 * action-space shift can't silently soft-lock trash selection (see the
 * `add-trash-select-modal` design, D5).
 */
export function TrashSelectModal({
  pendingSelection,
  actionMask,
  ownTrashIds,
  opponentTrashIds,
  localPlayer,
  onAction,
  onInspectCard,
}: TrashSelectModalProps) {
  // Local, ordered selection for deferred-toggle multi-select. Action ids.
  const [picked, setPicked] = useState<number[]>([]);
  // While draining the deferred picks to the engine, freeze the grid.
  const [committing, setCommitting] = useState(false);
  // Identity of the currently-open selection, to reset local state only when a
  // genuinely new selection opens (not on the intermediate re-prompts the
  // deferred drain produces).
  const identityRef = useRef<string | null>(null);

  const mode = trashSelectionMode(pendingSelection);
  const isLocal =
    pendingSelection != null &&
    pendingSelection.selectingPlayer === localPlayer &&
    !pendingSelection.keywordPrompt;
  const open = mode != null && isLocal;

  const identity = open
    ? `${pendingSelection!.prompt}::${pendingSelection!.selectingPlayer}`
    : null;

  useEffect(() => {
    if (committing) return;
    if (identity !== identityRef.current) {
      identityRef.current = identity;
      setPicked([]);
    }
  }, [identity, committing]);

  if (!open || !pendingSelection) return null;

  const cc: CountCappedKind | null =
    mode === 'multi' ? parseCountCappedKind(pendingSelection.kind) : null;
  // Deferred toggle only when the candidate set is stable between picks.
  const deferred = mode === 'multi' && cc != null && !cc.distinct;

  const ownerIds =
    pendingSelection.zoneOwner != null && pendingSelection.zoneOwner !== localPlayer
      ? opponentTrashIds
      : ownTrashIds;
  const ownerLabel =
    pendingSelection.zoneOwner != null && pendingSelection.zoneOwner !== localPlayer
      ? "Opponent's Trash"
      : 'Your Trash';

  const cards: TrashCard[] = ownerIds.map((cardId, i) => {
    const actionId = SELECTION.TRASH_START + i;
    return { cardId, actionId, legalNow: actionMask[actionId] === 1 };
  });

  const canDecline = actionMask[SELECTION.DECLINE] === 1;

  const inspectOnRightClick = (cardId: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    onInspectCard?.(cardId);
  };

  // ── Click handling ─────────────────────────────────────────────────────
  const onCardClick = (card: TrashCard) => {
    if (committing) return;
    if (mode === 'single') {
      if (card.legalNow) void onAction(card.actionId);
      return;
    }
    if (deferred) {
      // Toggle within the local set (true deselect), capped at `max`.
      setPicked((prev) => {
        if (prev.includes(card.actionId)) {
          return prev.filter((id) => id !== card.actionId);
        }
        if (!card.legalNow) return prev;
        if (cc && prev.length >= cc.max) return prev;
        return [...prev, card.actionId];
      });
      return;
    }
    // Immediate-commit (distinct): each click dispatches one pick.
    if (card.legalNow) void onAction(card.actionId);
  };

  const onDone = async () => {
    if (committing || !cc) return;
    if (deferred) {
      setCommitting(true);
      try {
        for (const id of picked) {
          await onAction(id);
        }
        // The final pick auto-commits when it reaches `max`; otherwise stop.
        if (picked.length < cc.max && canDecline) {
          await onAction(SELECTION.DECLINE);
        } else if (picked.length === 0 && canDecline) {
          // Zero-pick finish (optional-zero) — PASS to take none.
          await onAction(SELECTION.DECLINE);
        }
      } finally {
        setCommitting(false);
      }
      return;
    }
    // Immediate-commit mode: Done is the stop/PASS.
    if (canDecline) void onAction(SELECTION.DECLINE);
  };

  // ── Derived UI state ───────────────────────────────────────────────────
  const isSelected = (card: TrashCard): boolean => {
    if (deferred) return picked.includes(card.actionId);
    // Immediate-commit: a card that was a candidate but is no longer legal has
    // been committed (the engine removed it from the mask).
    if (mode === 'multi') return !card.legalNow && isTrashCandidateConsumed(card, pendingSelection);
    return false;
  };

  const selectedCount = deferred ? picked.length : (cc?.picked ?? 0);
  const doneEnabled = deferred
    ? cc != null && picked.length >= cc.min
    : canDecline;

  const title = pendingSelection.prompt || 'Select a card from the trash';
  const legalSelectable = cards.filter((c) => c.legalNow).length;

  return (
    <div
      data-testid="trash-select-modal"
      className="fixed inset-0 z-40 flex items-center justify-center"
    >
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" />

      <div className="relative bg-[var(--ib-graphite)] rounded-xl shadow-2xl max-w-4xl w-full mx-4 max-h-[85vh] flex flex-col border border-[var(--ib-line)]">
        {/* Header */}
        <div className="bg-gradient-to-b from-[var(--ib-panel-2)] to-transparent px-5 py-4 border-b border-[var(--ib-line)]">
          <div className="flex items-center justify-between gap-4">
            <div>
              <div className="text-[11px] uppercase tracking-wider text-[var(--ib-bone-dd)] font-semibold">
                {ownerLabel}
              </div>
              <h2 className="text-base font-semibold text-[var(--ib-bone)]">{title}</h2>
              {mode === 'multi' && cc ? (
                <span data-testid="trash-select-counter" className="text-xs text-[var(--ib-bone-dd)] mt-0.5">
                  {selectedCount} / {cc.max} selected
                  {cc.min > 0 && selectedCount < cc.min ? ` — select at least ${cc.min}` : ''}
                </span>
              ) : (
                legalSelectable > 0 && (
                  <span className="text-xs text-[var(--ib-bone-dd)] mt-0.5">
                    {legalSelectable} of {cards.length} selectable
                  </span>
                )
              )}
            </div>
            <div className="flex items-center gap-2">
              {mode === 'multi' && (
                <button
                  data-testid="trash-select-done"
                  disabled={!doneEnabled || committing}
                  onClick={onDone}
                  className={`px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
                    doneEnabled && !committing
                      ? 'bg-purple-600 hover:bg-purple-500 text-white'
                      : 'bg-[var(--ib-panel)] text-[var(--ib-bone-dd)] cursor-not-allowed'
                  }`}
                >
                  {committing ? 'Resolving…' : 'Done'}
                </button>
              )}
              {mode === 'single' && canDecline && (
                <button
                  data-testid="trash-select-decline"
                  onClick={() => onAction(SELECTION.DECLINE)}
                  className="px-4 py-2 bg-[var(--ib-panel-2)] hover:opacity-90 text-[var(--ib-bone)] text-sm font-medium rounded-lg border border-[var(--ib-line)] transition-colors"
                >
                  Decline
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Card grid */}
        <div className="overflow-y-auto p-5">
          {cards.length === 0 ? (
            <div className="text-center text-[var(--ib-bone-dd)] py-8">No cards in trash</div>
          ) : (
            <div className="grid grid-cols-5 gap-3 justify-items-center">
              {cards.map((card) => {
                const selected = isSelected(card);
                const atCap = deferred && cc != null && picked.length >= cc.max && !selected;
                // A card is interactable when: single & legal; deferred & (legal or
                // already selected) & under cap; immediate & legal. Frozen while
                // committing.
                const interactable =
                  !committing &&
                  (mode === 'single'
                    ? card.legalNow
                    : deferred
                      ? (card.legalNow || selected) && !atCap
                      : card.legalNow);
                const dimmed = !interactable && !selected;
                return (
                  <div
                    key={`${card.cardId}-${card.actionId}`}
                    data-testid={`trash-card-${card.actionId}`}
                    data-selected={selected ? 'true' : 'false'}
                    className={`relative transition-all ${
                      interactable ? 'cursor-pointer hover:scale-105' : 'cursor-not-allowed'
                    }`}
                  >
                    <Card
                      cardId={card.cardId}
                      size="md"
                      highlighted={selected || (interactable && mode !== 'single')}
                      dimmed={dimmed}
                      onClick={interactable || selected ? () => onCardClick(card) : undefined}
                      onContextMenu={inspectOnRightClick(card.cardId)}
                    />
                    {selected && (
                      <div className="absolute top-1 right-1 w-5 h-5 rounded-full bg-purple-500 text-white text-xs font-bold flex items-center justify-center shadow">
                        {deferred ? picked.indexOf(card.actionId) + 1 : '✓'}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * In immediate-commit (distinct) mode a card that is no longer legal has been
 * committed this selection. We only treat trash-range cards that way (the mask
 * also drops cards for other reasons, but within a trash selection the only
 * reason a trash card leaves the mask is that it was picked).
 */
function isTrashCandidateConsumed(
  card: TrashCard,
  pending: PendingSelection,
): boolean {
  // Heuristic guard: only meaningful for multi-select trash, where the engine
  // shrinks `validIndices` as picks land.
  return !pending.validIndices.includes(card.actionId);
}
