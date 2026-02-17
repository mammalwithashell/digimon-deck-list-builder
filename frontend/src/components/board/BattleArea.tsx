import { useDroppable } from '@dnd-kit/core';
import { PermanentSlot } from './PermanentSlot';
import type { PermanentInfo } from '@/types/game';
import { MAX_BATTLE_AREA_SLOTS } from '@/utils/constants';

interface DroppableSlotProps {
  slotIndex: number;
  isEmpty: boolean;
  isValidTarget: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}

function DroppableSlot({ slotIndex, isEmpty, isValidTarget, children, onClick }: DroppableSlotProps) {
  const dropType = isEmpty ? 'empty-field-slot' : 'occupied-field-slot';
  const { isOver, setNodeRef } = useDroppable({
    id: `field-slot-${slotIndex}`,
    data: { type: dropType, slotIndex },
  });

  const showDropHighlight = isOver && isValidTarget;

  return (
    <div
      ref={setNodeRef}
      className={showDropHighlight ? 'ring-2 ring-green-400 rounded' : ''}
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
}

export function BattleArea({
  permanents,
  isOpponent,
  highlightedSlots,
  targetedSlots,
  onSlotClick,
  onSlotHover,
}: BattleAreaProps) {
  const slots = Array.from({ length: MAX_BATTLE_AREA_SLOTS }, (_, i) => i);

  return (
    <div className="flex gap-1 flex-wrap justify-center min-h-[120px]">
      {slots.map((i) => {
        const perm = permanents[i];
        const isEmpty = !perm;

        return (
          <DroppableSlot
            key={i}
            slotIndex={i}
            isEmpty={isEmpty}
            isValidTarget={!isOpponent}
            onClick={() => onSlotClick?.(i)}
          >
            {isEmpty ? (
              <div className="w-[80px] h-[112px] border border-dashed border-gray-700/30 rounded flex items-center justify-center">
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
