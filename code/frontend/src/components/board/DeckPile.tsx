import { Card } from '@/components/shared/Card';

interface DeckPileProps {
  count: number;
  isOpponent?: boolean;
}

export function DeckPile({ count, isOpponent = false }: DeckPileProps) {
  return (
    <div className="ib-zone" data-testid="deck-pile">
      <div className="ib-zone__label">Deck</div>
      <div
        className="ib-zone__body ib-zone__body--deck relative overflow-hidden p-0"
        aria-label={`Deck: ${count} cards`}
        title={`Deck: ${count} cards`}
      >
        {count > 0 ? (
          <Card
            cardId="deck-back"
            size="sm"
            faceDown
            useDefaultSleeve={isOpponent}
            className="!block !w-full !h-full pointer-events-none"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-[10px] font-mono text-[var(--ink-2)] opacity-60">
            Empty
          </div>
        )}
        <div
          data-testid="deck-pile-count"
          className="absolute bottom-1 right-1 z-10 min-w-[20px] px-1.5 py-0.5 rounded-full bg-slate-950/85 border border-white/25 text-white text-[11px] font-bold leading-none text-center tabular-nums shadow-md pointer-events-none"
        >
          {count}
        </div>
      </div>
    </div>
  );
}
