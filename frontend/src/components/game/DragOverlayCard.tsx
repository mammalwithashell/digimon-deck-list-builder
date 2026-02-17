import { DragOverlay } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';

interface DragOverlayCardProps {
  cardId: string | null;
  isOverValid: boolean;
}

export function DragOverlayCard({ cardId, isOverValid }: DragOverlayCardProps) {
  if (!cardId) return <DragOverlay />;

  return (
    <DragOverlay>
      <div className={`${isOverValid ? 'opacity-80' : 'opacity-40'} pointer-events-none`}>
        <Card cardId={cardId} size="md" />
      </div>
    </DragOverlay>
  );
}
