import { useDraggable, useDroppable } from '@dnd-kit/core';
import { PermanentSlot } from './PermanentSlot';
import type { PermanentInfo } from '@/types/game';
import type { DragData } from '@/hooks/useDropZone';
import { BREEDING_SLOT } from '@/utils/constants';

interface BreedingAreaProps {
  permanent: PermanentInfo | null;
  canMove?: boolean;
  canDigivolveDrop?: boolean;
  highlighted?: boolean;
  dropId?: string;
  onClick?: () => void;
  /** Right-click (context-menu) opens the stack inspector for this permanent. */
  onInspect?: () => void;
}

export function BreedingArea({
  permanent,
  canMove = false,
  canDigivolveDrop = false,
  highlighted = false,
  dropId = 'breeding-slot',
  onClick,
  onInspect,
}: BreedingAreaProps) {
  const dragData: DragData = { type: 'breeding-perm' };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `${dropId}-perm`,
    data: dragData,
    disabled: !canMove,
  });
  const { isOver, setNodeRef: setDropNodeRef } = useDroppable({
    id: dropId,
    data: { type: 'breeding-slot', slotIndex: BREEDING_SLOT },
    disabled: !canDigivolveDrop,
  });

  const showDropHighlight = isOver && canDigivolveDrop;

  return (
    <div className="ib-raise-zone">
      <div className="ib-raise-zone__label">Raising</div>
      {permanent ? (
        <div
          ref={(node) => {
            setNodeRef(node);
            setDropNodeRef(node);
          }}
          {...listeners}
          {...attributes}
          className={`ib-raise-zone__perm ${canMove ? 'ib-raise-zone__perm--ready' : ''} ${showDropHighlight || highlighted ? 'ib-raise-zone__perm--target' : ''} ${isDragging ? 'opacity-30' : ''}`}
          onClick={onClick}
        >
          <PermanentSlot
            perm={permanent}
            slotIndex={-1}
            isOpponent={false}
            onInspect={onInspect}
          />
        </div>
      ) : (
        <div
          ref={setDropNodeRef}
          className={`ib-raise-zone__empty ${showDropHighlight ? 'ib-raise-zone__empty--target' : ''}`}
        >
          <span>Empty</span>
        </div>
      )}
    </div>
  );
}
