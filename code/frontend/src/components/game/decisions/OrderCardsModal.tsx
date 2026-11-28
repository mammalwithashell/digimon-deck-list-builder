import './decisions.css';
import { useEffect, useMemo, useState } from 'react';
import { Card } from '@/components/shared/Card';
import { SELECTION } from '@/utils/constants';
import { isSourceSelectAction, sourceSelectionCards } from '@/utils/sourceSelection';
import { isTrashAction } from '@/utils/trashSelection';
import { usePeekStore } from '@/stores/peekStore';
import { useResolutionPlanStore } from '@/stores/resolutionPlanStore';
import type { PendingSelection, PermanentInfo } from '@/types/game';

export interface PermutationCardEntry {
  actionId: number;
  cardId: string;
  label?: string;
}

export interface PermutationContext {
  localPlayer: number;
  handIds: string[];
  trashIds: string[];
  opponentTrashIds: string[];
  battleArea: PermanentInfo[];
  opponentBattleArea: PermanentInfo[];
  revealedCards: { cardId: string; owner: number }[];
}

/**
 * Resolve every valid non-decline action ID in a `SelectPermutation` prompt
 * to its displayable card entry (revealed cards, battle-area stacks, hand, or trash).
 */
export function resolvePermutationCards(
  pending: PendingSelection,
  ctx: PermutationContext,
): PermutationCardEntry[] | null {
  const opponentZone =
    pending.zoneOwner != null && pending.zoneOwner !== ctx.localPlayer;
  const trash = opponentZone ? ctx.opponentTrashIds : ctx.trashIds;
  const area = opponentZone ? ctx.opponentBattleArea : ctx.battleArea;
  const entries: PermutationCardEntry[] = [];

  for (const id of pending.validIndices) {
    if (id === SELECTION.DECLINE) continue;
    if (id >= SELECTION.REVEALED_START && id <= SELECTION.REVEALED_END) {
      const rev = ctx.revealedCards[id - SELECTION.REVEALED_START];
      if (!rev?.cardId) return null;
      entries.push({ actionId: id, cardId: rev.cardId, label: 'Revealed' });
    } else if (id >= SELECTION.HAND_START && id <= SELECTION.HAND_END) {
      if (opponentZone) return null;
      const cardId = ctx.handIds[id - SELECTION.HAND_START];
      if (!cardId) return null;
      entries.push({ actionId: id, cardId, label: 'Hand' });
    } else if (isTrashAction(id)) {
      const cardId = trash[id - SELECTION.TRASH_START];
      if (!cardId) return null;
      entries.push({ actionId: id, cardId, label: 'Trash' });
    } else if (isSourceSelectAction(id)) {
      const [tile] = sourceSelectionCards([id], area);
      if (!tile) return null;
      entries.push({ actionId: id, cardId: tile.cardId, label: 'Digivolution' });
    } else {
      return null;
    }
  }
  return entries.length > 0 ? entries : null;
}

export interface OrderCardsModalProps {
  pendingSelection: PendingSelection;
  entries: PermutationCardEntry[];
  canDecline: boolean;
  onAction: (actionId: number) => void;
  onInspectCard?: (cardId: string) => void;
}

/**
 * Aegis-inspired card ordering modal for `GamePhase.SelectPermutation`
 * (e.g. returning revealed/stacked cards to the top or bottom of the deck).
 *
 * Players number cards 1..N via click or left/right nudges and click
 * "Confirm Order", which submits the first pick immediately and queues the
 * remaining picks in `resolutionPlanStore` so the 1-step engine advances smoothly.
 */
export function OrderCardsModal({
  pendingSelection,
  entries,
  canDecline,
  onAction,
  onInspectCard,
}: OrderCardsModalProps) {
  const peeking = usePeekStore((s) => s.peeking);
  const togglePeek = usePeekStore((s) => s.toggle);
  const startPermutationOrder = useResolutionPlanStore((s) => s.startPermutationOrder);

  const entryKey = useMemo(
    () => entries.map((e) => e.actionId).join(','),
    [entries],
  );
  const [orderedIds, setOrderedIds] = useState<number[]>([]);

  useEffect(() => {
    setOrderedIds([]);
  }, [entryKey]);

  const promptLower = (pendingSelection.prompt || '').toLowerCase();
  const destinationEyebrow = promptLower.includes('bottom')
    ? 'Return Remaining Cards · Bottom of Deck'
    : promptLower.includes('top')
      ? 'Return Cards · Top of Deck'
      : 'Order Remaining Cards';
  const directionHint = promptLower.includes('bottom')
    ? '1 = Placed first onto the bottom of your deck'
    : promptLower.includes('top')
      ? '1 = Placed first onto the top of your deck'
      : '1 = First card placed in order';

  const toggleCard = (actionId: number) => {
    setOrderedIds((prev) =>
      prev.includes(actionId)
        ? prev.filter((id) => id !== actionId)
        : [...prev, actionId],
    );
  };

  const moveOrder = (actionId: number, delta: -1 | 1) => {
    setOrderedIds((prev) => {
      const idx = prev.indexOf(actionId);
      if (idx === -1) return prev;
      const target = idx + delta;
      if (target < 0 || target >= prev.length) return prev;
      const next = [...prev];
      [next[idx], next[target]] = [next[target]!, next[idx]!];
      return next;
    });
  };

  const fillRemaining = () => {
    setOrderedIds((prev) => {
      const existing = new Set(prev);
      const missing = entries.map((e) => e.actionId).filter((id) => !existing.has(id));
      return [...prev, ...missing];
    });
  };

  const finalOrder = useMemo(() => {
    if (orderedIds.length === entries.length) return orderedIds;
    const picked = new Set(orderedIds);
    return [
      ...orderedIds,
      ...entries.map((e) => e.actionId).filter((id) => !picked.has(id)),
    ];
  }, [entries, orderedIds]);

  const handleConfirm = () => {
    if (finalOrder.length === 0) return;
    const [first, ...rest] = finalOrder;
    if (rest.length > 0) {
      startPermutationOrder(rest, pendingSelection);
    }
    onAction(first!);
  };

  const allAssigned = orderedIds.length === entries.length;

  return (
    <>
      <div
        data-testid="order-cards-modal"
        className={`hs-modal-backdrop ${peeking ? 'is-peeking' : ''}`}
      >
        <div className="hs-modal-sheet max-w-[920px]">
          <div className="hs-modal-sheet__header">
            <div>
              <div className="hs-decision-rail__eyebrow">{destinationEyebrow}</div>
              <h2 className="hs-modal-sheet__title">
                {pendingSelection.prompt || 'Choose card order'}
              </h2>
              <div className="hs-modal-sheet__subtitle">
                {directionHint} · Click cards in order ({orderedIds.length}/{entries.length} numbered)
              </div>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={fillRemaining}
                disabled={allAssigned}
                className="ds-btn ds-btn--ghost"
              >
                Auto-Fill
              </button>
              <button
                type="button"
                onClick={() => setOrderedIds([])}
                disabled={orderedIds.length === 0}
                className="ds-btn ds-btn--ghost"
              >
                Clear
              </button>
            </div>
          </div>

          <div className="hs-modal-sheet__body">
            <div
              className={`hs-card-gallery ${entries.length > 6 ? 'hs-card-gallery--scroll' : ''}`}
            >
              {entries.map((entry) => {
                const orderIdx = orderedIds.indexOf(entry.actionId);
                const isOrdered = orderIdx !== -1;
                return (
                  <div
                    key={entry.actionId}
                    data-testid={`order-card-${entry.actionId}`}
                    className={`hs-order-card ${isOrdered ? 'is-ordered' : ''}`}
                  >
                    <button
                      type="button"
                      onClick={() => toggleCard(entry.actionId)}
                      className="hs-order-card__btn"
                      onContextMenu={(e) => {
                        e.preventDefault();
                        onInspectCard?.(entry.cardId);
                      }}
                    >
                      <span className="hs-order-card__badge">
                        {isOrdered ? orderIdx + 1 : '·'}
                      </span>
                      <Card
                        cardId={entry.cardId}
                        size="lg"
                        highlighted={isOrdered}
                      />
                    </button>

                    <div className="hs-order-card__controls">
                      {isOrdered ? (
                        <>
                          <button
                            type="button"
                            className="hs-order-nudge"
                            aria-label="Move earlier"
                            disabled={orderIdx === 0}
                            onClick={() => moveOrder(entry.actionId, -1)}
                          >
                            ←
                          </button>
                          <span className="hs-zone-pill">#{orderIdx + 1}</span>
                          <button
                            type="button"
                            className="hs-order-nudge"
                            aria-label="Move later"
                            disabled={orderIdx === orderedIds.length - 1}
                            onClick={() => moveOrder(entry.actionId, 1)}
                          >
                            →
                          </button>
                        </>
                      ) : (
                        <span className="hs-zone-pill">{entry.label ?? 'Card'}</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="hs-modal-sheet__footer">
            <div className="flex items-center gap-2">
              <button
                type="button"
                data-testid="order-view-board"
                onClick={togglePeek}
                className="ds-btn ds-btn--ghost"
              >
                View Board
              </button>
              {canDecline && (
                <button
                  type="button"
                  data-testid="selection-decline"
                  onClick={() => onAction(SELECTION.DECLINE)}
                  className="ds-btn ds-btn--ghost"
                >
                  Decline
                </button>
              )}
            </div>
            <button
              type="button"
              data-testid="order-confirm"
              onClick={handleConfirm}
              className="ds-btn ds-btn--primary"
            >
              {allAssigned
                ? `Confirm Order (${entries.length})`
                : orderedIds.length > 0
                  ? `Confirm Order (auto-fill ${entries.length - orderedIds.length})`
                  : 'Confirm Shown Order'}
            </button>
          </div>
        </div>
      </div>

      {peeking && (
        <button type="button" onClick={togglePeek} className="hs-peek-return">
          Return to Card Order
        </button>
      )}
    </>
  );
}
