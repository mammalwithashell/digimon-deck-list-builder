import { Card } from '@/components/shared/Card';

interface RevealedCardsZoneProps {
  cards: { cardId: string; owner: number }[];
  validIndices?: Set<number>;
  onCardClick?: (index: number) => void;
}

export function RevealedCardsZone({
  cards,
  validIndices,
  onCardClick,
}: RevealedCardsZoneProps) {
  if (cards.length === 0) return null;

  return (
    <div className="flex items-center gap-1 p-2 bg-gray-800/30 rounded">
      <span className="text-[10px] text-gray-500 mr-1">Revealed:</span>
      {cards.map((card, i) => (
        <Card
          key={`${card.cardId}-${i}`}
          cardId={card.cardId}
          size="sm"
          highlighted={validIndices?.has(i)}
          onClick={() => onCardClick?.(i)}
        />
      ))}
    </div>
  );
}
