import { Card } from '@/components/shared/Card';

interface RevealedCardsZoneProps {
  cards: { cardId: string; owner: number }[];
  validIndices?: Set<number>;
  onCardClick?: (index: number) => void;
  /** Label for the zone. Driven by context (e.g. the active selection prompt)
   *  so a deck SEARCH is not mislabeled "Security". */
  title?: string;
  /** Optional second line under the title. */
  subtitle?: string;
}

export function RevealedCardsZone({
  cards,
  validIndices,
  onCardClick,
  title = 'Revealed',
  subtitle,
}: RevealedCardsZoneProps) {
  if (cards.length === 0) return null;

  const hasClickable = validIndices && validIndices.size > 0;

  return (
    <div
      className="flex items-center gap-3 px-4 py-2 rounded-[var(--radius-md)] border shadow-[var(--shadow-card)] backdrop-blur-sm"
      style={{
        background: 'color-mix(in srgb, var(--surface) 92%, transparent)',
        borderColor: hasClickable ? 'var(--warn)' : 'var(--line-1)',
      }}
    >
      <div className="flex flex-col items-center mr-1 max-w-[136px]">
        <span
          className="text-[10px] font-bold uppercase tracking-wider text-center leading-tight"
          style={{ color: hasClickable ? 'var(--warn)' : 'var(--ink-1)', fontFamily: 'var(--font-mono)' }}
        >
          {title}
        </span>
        {subtitle && (
          <span className="text-[10px] text-[var(--ink-2)] text-center">{subtitle}</span>
        )}
      </div>
      <div className="w-px h-10 bg-[var(--line-0)]" />
      <div className="flex gap-2">
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
