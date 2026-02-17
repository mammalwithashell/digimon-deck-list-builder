import { useDraggable } from '@dnd-kit/core';
import { PermanentSlot } from './PermanentSlot';
import type { PermanentInfo } from '@/types/game';
import type { DragData } from '@/hooks/useDropZone';

interface BreedingAreaProps {
  permanent: PermanentInfo | null;
  canMove?: boolean;
  onClick?: () => void;
}

export function BreedingArea({ permanent, canMove = false, onClick }: BreedingAreaProps) {
  const dragData: DragData = { type: 'breeding-perm' };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: 'breeding-perm',
    data: dragData,
    disabled: !canMove,
  });

  return (
    <div className="flex flex-col items-center gap-1">
      {permanent ? (
        <div
          ref={setNodeRef}
          {...listeners}
          {...attributes}
          className={`${canMove ? 'ring-1 ring-green-400/50 rounded cursor-grab' : ''} ${isDragging ? 'opacity-30' : ''}`}
          onClick={onClick}
        >
          <PermanentSlot
            perm={permanent}
            slotIndex={-1}
            isOpponent={false}
          />
        </div>
      ) : (
        <div className="w-[80px] h-[112px] border border-dashed border-gray-700/50 rounded flex items-center justify-center">
          <span className="text-[9px] text-gray-600">Breeding</span>
        </div>
      )}
      <span className="text-[9px] text-gray-500">Breeding</span>
    </div>
  );
}
