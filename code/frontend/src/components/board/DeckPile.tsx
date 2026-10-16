interface DeckPileProps {
  count: number;
}

export function DeckPile({ count }: DeckPileProps) {
  return (
    <div className="ib-zone ib-zone--deck">
      <div className="ib-zone__label">Deck</div>
      <div className="ib-zone__body ib-zone__body--deck">
        <span className="ib-zone__count">{count}</span>
      </div>
    </div>
  );
}
