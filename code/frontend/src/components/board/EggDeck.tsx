interface EggDeckProps {
  count: number;
  canHatch?: boolean;
  onClick?: () => void;
}

export function EggDeck({ count, canHatch = false, onClick }: EggDeckProps) {
  return (
    <div className={`ib-zone ib-zone--egg ${canHatch ? 'ib-zone--ready' : ''}`}>
      <div className="ib-zone__label">Eggs</div>
      <div
        className="ib-zone__body ib-zone__body--egg"
        onClick={canHatch ? onClick : undefined}
      >
        <span>{count}</span>
      </div>
    </div>
  );
}
