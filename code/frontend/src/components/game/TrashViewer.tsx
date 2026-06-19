import { Modal } from '@/components/shared/Modal';
import { Card } from '@/components/shared/Card';

interface TrashViewerProps {
  isOpen: boolean;
  onClose: () => void;
  trashIds: string[];
  ownerLabel: string;
  /** Right-click a trash card to inspect it at full size. */
  onInspect?: (cardId: string) => void;
}

export function TrashViewer({ isOpen, onClose, trashIds, ownerLabel, onInspect }: TrashViewerProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`${ownerLabel}'s Trash (${trashIds.length})`}>
      {trashIds.length === 0 ? (
        <div className="text-center text-[var(--ib-bone-dd)] py-8">No cards in trash</div>
      ) : (
        <div className="grid grid-cols-5 gap-2">
          {trashIds.map((cardId, i) => (
            <Card
              key={`${cardId}-${i}`}
              cardId={cardId}
              size="md"
              onContextMenu={
                onInspect
                  ? (e) => {
                      e.preventDefault();
                      onInspect(cardId);
                    }
                  : undefined
              }
            />
          ))}
        </div>
      )}
    </Modal>
  );
}

