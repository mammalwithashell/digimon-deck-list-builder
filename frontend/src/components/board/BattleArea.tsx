import { useDroppable } from '@dnd-kit/core';
import { PermanentSlot } from './PermanentSlot';
import type { PermanentInfo } from '@/types/game';
import { MAX_BATTLE_AREA_SLOTS } from '@/utils/constants';

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
  const { isOver, setNodeRef } = useDroppable({
    id: `field-slot-${slotIndex}`,
    data: { type: dropType, slotIndex },
  });

  const showDropHighlight = isOver && isValidDrop;

  return (
    <div
      ref={setNodeRef}
      data-testid={`field-slot-${isOpponent ? 'p2' : 'p1'}-${slotIndex}`}
      data-slot-id={`${isOpponent ? 'opponent' : 'player'}-${slotIndex}`}
      className={`rounded ${
        showDropHighlight
          ? isEmpty
            ? 'ring-2 ring-green-400'
            : 'ring-2 ring-purple-400'
          : isDigivolveTarget
            ? 'ring-2 ring-purple-400/60'
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
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
}: BattleAreaProps) {
  const slots = Array.from({ length: MAX_BATTLE_AREA_SLOTS }, (_, i) => i);

  return (
    <div className="grid grid-cols-6 gap-1 justify-center min-h-[276px] w-fit mx-auto">
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
            onClick={() => onSlotClick?.(i)}
          >
            {isEmpty ? (
              <div className="w-[96px] h-[134px] border border-dashed border-gray-700/30 rounded flex items-center justify-center">
                <span className="text-[9px] text-gray-700">{i}</span>
              </div>
            ) : (
              <PermanentSlot
                perm={perm}
                slotIndex={i}
                isOpponent={isOpponent}
                highlighted={highlightedSlots?.has(i)}
                targeted={targetedSlots?.has(i)}
                onClick={() => onSlotClick?.(i)}
                onMouseEnter={() => onSlotHover?.(i)}
                onMouseLeave={() => onSlotHover?.(null)}
              />
            )}
          </DroppableSlot>
        );
      })}
    </div>
  );
}
