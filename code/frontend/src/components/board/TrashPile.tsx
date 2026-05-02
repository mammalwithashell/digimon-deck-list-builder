interface TrashPileProps {
  count: number;
  onClick?: () => void;
}

export function TrashPile({ count, onClick }: TrashPileProps) {
  return (
    <div className="ib-zone ib-zone--clickable">
      <div className="ib-zone__label">Trash</div>
      <div
        className="ib-zone__body ib-zone__body--trash"
        onClick={onClick}
      >
        <span>{count}</span>
      </div>
    </div>
  );
}
