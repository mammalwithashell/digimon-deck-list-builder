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

  const hasClickable = validIndices && validIndices.size > 0;

  return (
    <div
      className={`flex items-center gap-2 px-4 py-2 rounded-lg border ${
        hasClickable
          ? 'bg-amber-900/30 border-amber-500/40 shadow-[0_0_12px_rgba(245,158,11,0.15)]'
          : 'bg-gray-800/50 border-gray-600/40'
      }`}
    >
      <div className="flex flex-col items-center mr-1">
        <span className="text-[10px] font-bold uppercase tracking-wider text-amber-400">
          Security
        </span>
        <span className="text-[9px] text-gray-400">Check</span>
      </div>
      <div className="w-px h-10 bg-gray-600/50" />
      <div className="flex gap-1.5">
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
    </div>
  );
}
