import { useRef, useEffect, useMemo, useState } from 'react';
import { useDroppable } from '@dnd-kit/core';
import { PermanentSlot } from './PermanentSlot';
import type { PermanentInfo } from '@/types/game';
import { MAX_BATTLE_AREA_SLOTS } from '@/utils/constants';
import { usePositionTransitions } from '@/hooks/usePositionTransitions';

interface DroppableSlotProps {
  slotIndex: number;
  isEmpty: boolean;
  isOpponent: boolean;
  /** This slot is a valid drop target for the currently dragged card */
  isValidDrop: boolean;
  /** This slot should glow to indicate it's a valid digivolve target */
  isDigivolveTarget: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}

function DroppableSlot({ slotIndex, isEmpty, isOpponent, isValidDrop, isDigivolveTarget, children, onClick }: DroppableSlotProps) {
  const dropType = isEmpty ? 'empty-field-slot' : 'occupied-field-slot';
  const owner = isOpponent ? 'opponent' : 'player';
  const { isOver, setNodeRef } = useDroppable({
    id: `field-slot-${owner}-${slotIndex}`,
    data: { type: dropType, slotIndex },
    disabled: isOpponent,
  });

  const showDropHighlight = isOver && isValidDrop;

  return (
    <div
      ref={setNodeRef}
      data-testid={`field-slot-${isOpponent ? 'p2' : 'p1'}-${slotIndex}`}
      data-slot-id={`${owner}-${slotIndex}`}
      className={`ib-drop-slot ${
        showDropHighlight
          ? isEmpty
            ? 'ib-drop-slot--play'
            : 'ib-drop-slot--digivolve'
          : isDigivolveTarget
            ? 'ib-drop-slot--digivolve-soft'
            : ''
      }`}
      onClick={onClick}
    >
      {children}
    </div>
  );
}

interface BattleAreaProps {
  permanents: PermanentInfo[];
  isOpponent: boolean;
  highlightedSlots?: Set<number>;
  targetedSlots?: Set<number>;
  onSlotClick?: (slotIndex: number) => void;
  onSlotHover?: (slotIndex: number | null) => void;
  /** Right-click (context-menu) on a filled slot opens the stack inspector. */
  onSlotInspect?: (slotIndex: number) => void;
  /** Field slots where dragged hand card can digivolve */
  dragValidDropSlots?: Set<number>;
  /** Whether a hand card is being dragged */
  isDraggingHandCard?: boolean;
  /** Whether the dragged hand card can be played (to empty slots) */
  canPlayDragged?: boolean;
}

export function BattleArea({
  permanents,
  isOpponent,
  highlightedSlots,
  targetedSlots,
  onSlotClick,
  onSlotHover,
  onSlotInspect,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
}: BattleAreaProps) {
  const slots = Array.from({ length: MAX_BATTLE_AREA_SLOTS }, (_, i) => i);

  // Track previous card IDs per slot to detect entries and exits
  const prevCardIds = useRef<(string | null)[]>(
    Array(MAX_BATTLE_AREA_SLOTS).fill(null)
  );
  const [animatingSlots, setAnimatingSlots] = useState<Set<number>>(new Set());
  const [, setExitingSlots] = useState<Map<number, PermanentInfo>>(new Map());

  // FLIP animation keys: stable identity per filled slot so we can
  // smoothly slide cards when engine indices shift left after a midfield
  // deletion. Composite of `(topCardId, turnPlayed)` is good enough in
  // practice — duplicates same-turn would lose animation pairing but
  // never visually teleport, which is acceptable.
  const flipKeys = useMemo(() => {
    return slots
      .map((i) => {
        const p = permanents[i];
        if (!p) return null;
        return `${isOpponent ? 'opp' : 'me'}:${p.topCardId ?? '?'}:${p.turnPlayed}`;
      })
      .filter((k): k is string => k !== null);
  }, [permanents, isOpponent, slots]);
  const { registerNode } = usePositionTransitions(flipKeys, {
    durationMs: 250,
  });

  useEffect(() => {
    const newAnimating = new Set<number>();
    const newExiting = new Map<number, PermanentInfo>();
    for (let i = 0; i < MAX_BATTLE_AREA_SLOTS; i++) {
      const currentId = permanents[i]?.topCardId ?? null;
      const prevId = prevCardIds.current[i];
      // New card appeared in a slot that was empty or had a different card
      if (currentId && currentId !== prevId) {
        newAnimating.add(i);
      }
      prevCardIds.current[i] = currentId;
    }
    if (newAnimating.size > 0) {
      setAnimatingSlots(newAnimating);
      const timer = setTimeout(() => setAnimatingSlots(new Set()), 400);
      return () => clearTimeout(timer);
    }
    if (newExiting.size > 0) {
      setExitingSlots(newExiting);
      const timer = setTimeout(() => setExitingSlots(new Map()), 300);
      return () => clearTimeout(timer);
    }
  }, [permanents]);

  return (
    <div className="ib-battle-area">
      {slots.map((i) => {
        const perm = permanents[i];
        const isEmpty = !perm;

        return (
          <DroppableSlot
            key={i}
            slotIndex={i}
            isEmpty={isEmpty}
            isOpponent={isOpponent}
            isValidDrop={
              !isOpponent && isDraggingHandCard
                ? isEmpty
                  ? canPlayDragged
                  : (dragValidDropSlots?.has(i) ?? false)
                : !isOpponent
            }
            isDigivolveTarget={
              !isOpponent && isDraggingHandCard && !isEmpty && (dragValidDropSlots?.has(i) ?? false)
            }
            // The wrapper owns the click for the WHOLE slot (so clicking
            // card or padding works). The inner PermanentSlot must NOT also
            // bind onClick — its click would bubble here and double-fire
            // onSlotClick, double-dispatching actions (e.g. a dropped DNA
            // material pick once actionPendingRef de-dupes the second fire).
            onClick={() => onSlotClick?.(i)}
          >
            {isEmpty ? (
              <div className="ib-battle-slot ib-battle-slot--empty">
                <span>SLOT {String(i + 1).padStart(2, '0')}</span>
              </div>
            ) : (
              <div
                ref={registerNode(
                  `${isOpponent ? 'opp' : 'me'}:${perm.topCardId ?? '?'}:${perm.turnPlayed}`,
                )}
                className={animatingSlots.has(i) ? 'animate-card-play-in' : ''}
              >
                <PermanentSlot
                  perm={perm}
                  slotIndex={i}
                  isOpponent={isOpponent}
                  highlighted={highlightedSlots?.has(i)}
                  targeted={targetedSlots?.has(i)}
                  // No onClick here — the DroppableSlot wrapper owns the
                  // click for the whole slot. Binding it here too would
                  // double-fire onSlotClick (bubbling) and drop the second
                  // DNA material pick. Hover handlers stay local.
                  onMouseEnter={() => onSlotHover?.(i)}
                  onMouseLeave={() => onSlotHover?.(null)}
                  // Right-click inspects this permanent. Bound here (not on the
                  // wrapper) so it only fires over the actual card, and
                  // stopPropagation avoids the wrapper's drop/click handlers.
                  onInspect={onSlotInspect ? () => onSlotInspect(i) : undefined}
                />
              </div>
            )}
          </DroppableSlot>
        );
      })}
    </div>
  );
}
