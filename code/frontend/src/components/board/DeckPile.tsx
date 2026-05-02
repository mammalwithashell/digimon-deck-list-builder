interface DeckPileProps {
  count: number;
}

export function DeckPile({ count }: DeckPileProps) {
  return (
    <div className="ib-zone">
      <div className="ib-zone__label">Deck</div>
      <div className="ib-zone__body ib-zone__body--deck">
        <span>{count}</span>
      </div>
    </div>
  );
}
