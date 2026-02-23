import { useDraggable, useDroppable } from '@dnd-kit/core';
import { PermanentSlot } from './PermanentSlot';
import type { PermanentInfo } from '@/types/game';
import type { DragData } from '@/hooks/useDropZone';

interface BreedingAreaProps {
  permanent: PermanentInfo | null;
  canMove?: boolean;
  canDigivolveDrop?: boolean;
  highlighted?: boolean;
  dropId?: string;
  onClick?: () => void;
}

export function BreedingArea({
  permanent,
  canMove = false,
  canDigivolveDrop = false,
  highlighted = false,
  dropId = 'breeding-slot',
  onClick,
}: BreedingAreaProps) {
  const dragData: DragData = { type: 'breeding-perm' };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: 'breeding-perm',
    data: dragData,
    disabled: !canMove,
  });
  const { isOver, setNodeRef: setDropNodeRef } = useDroppable({
    id: dropId,
    data: { type: 'breeding-slot', slotIndex: 12 },
    disabled: !canDigivolveDrop,
  });

  const showDropHighlight = isOver && canDigivolveDrop;

  return (
    <div className="flex flex-col items-center gap-1">
      {permanent ? (
        <div
          ref={(node) => {
            setNodeRef(node);
            setDropNodeRef(node);
          }}
          {...listeners}
          {...attributes}
          className={`${canMove ? 'ring-1 ring-green-400/50 rounded cursor-grab' : ''} ${showDropHighlight || highlighted ? 'ring-2 ring-purple-400 cursor-pointer' : ''} ${isDragging ? 'opacity-30' : ''}`}
          onClick={onClick}
        >
          <PermanentSlot
            perm={permanent}
            slotIndex={-1}
            isOpponent={false}
          />
        </div>
      ) : (
        <div
          ref={setDropNodeRef}
          className={`w-[80px] h-[112px] border border-dashed rounded flex items-center justify-center ${
            showDropHighlight ? 'border-green-400 ring-2 ring-green-400/80' : 'border-gray-700/50'
          }`}
        >
          <span className="text-[9px] text-gray-600">Breeding</span>
        </div>
      )}
      <span className="text-[9px] text-gray-500">Breeding</span>
    </div>
  );
}
