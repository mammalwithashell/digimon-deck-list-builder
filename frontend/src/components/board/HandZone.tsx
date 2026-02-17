import { useDraggable } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';
import { useGameStore } from '@/stores/gameStore';
import type { DragData } from '@/hooks/useDropZone';

interface DraggableHandCardProps {
  cardId: string;
  index: number;
  isOpponent: boolean;
  highlighted: boolean;
  onClick: () => void;
}

function DraggableHandCard({ cardId, index, isOpponent, highlighted, onClick }: DraggableHandCardProps) {
  const setHoveredCard = useGameStore((s) => s.setHoveredCard);
  const dragData: DragData = { type: 'hand-card', handIndex: index, cardId };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `hand-card-${index}`,
    data: dragData,
    disabled: isOpponent,
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={`transition-transform hover:-translate-y-2 ${isDragging ? 'opacity-30' : ''}`}
      style={{ marginLeft: index > 0 ? '-10px' : 0, zIndex: isDragging ? 100 : index }}
    >
      <Card
        cardId={cardId}
        size="md"
        faceDown={isOpponent}
        highlighted={highlighted}
        onClick={onClick}
        onMouseEnter={() => {
          if (!isOpponent) setHoveredCard(cardId);
        }}
        onMouseLeave={() => setHoveredCard(null)}
      />
    </div>
  );
}

interface HandZoneProps {
  cardIds: string[];
  isOpponent: boolean;
  highlightedIndices?: Set<number>;
  onCardClick?: (handIndex: number) => void;
  onCardHover?: (cardId: string | null) => void;
}

export function HandZone({
  cardIds,
  isOpponent,
  highlightedIndices,
  onCardClick,
}: HandZoneProps) {
  return (
    <div className="flex justify-center gap-[-8px] py-1">
      {cardIds.map((cardId, i) => (
        <DraggableHandCard
          key={`${cardId}-${i}`}
          cardId={cardId}
          index={i}
          isOpponent={isOpponent}
          highlighted={highlightedIndices?.has(i) ?? false}
          onClick={() => onCardClick?.(i)}
        />
      ))}
      {cardIds.length === 0 && (
        <div className="text-xs text-gray-600 py-8">No cards in hand</div>
      )}
    </div>
  );
}
