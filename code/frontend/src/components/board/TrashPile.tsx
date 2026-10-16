import { Card } from '@/components/shared/Card';

interface TrashPileProps {
  count: number;
  /** The most recently trashed card, shown face-up on top of the pile. */
  topCardId?: string | null;
  onClick?: () => void;
}

export function TrashPile({ count, topCardId, onClick }: TrashPileProps) {
  return (
    <div className="ib-zone ib-zone--clickable">
      <div className="ib-zone__label">Trash</div>
      <div
        className={`ib-zone__body ib-zone__body--trash ${topCardId ? 'has-cards' : ''}`}
        onClick={onClick}
      >
        {topCardId && <Card cardId={topCardId} size="sm" className="!w-[72px] !h-[100px]" />}
        <span className="ib-zone__count">{count}</span>
      </div>
    </div>
  );
}
