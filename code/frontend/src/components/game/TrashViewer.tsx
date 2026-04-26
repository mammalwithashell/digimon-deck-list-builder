import { Modal } from '@/components/shared/Modal';
import { Card } from '@/components/shared/Card';

interface TrashViewerProps {
  isOpen: boolean;
  onClose: () => void;
  trashIds: string[];
  ownerLabel: string;
}

export function TrashViewer({ isOpen, onClose, trashIds, ownerLabel }: TrashViewerProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`${ownerLabel}'s Trash (${trashIds.length})`}>
      {trashIds.length === 0 ? (
        <div className="text-center text-gray-400 py-8">No cards in trash</div>
      ) : (
        <div className="grid grid-cols-5 gap-2">
          {trashIds.map((cardId, i) => (
            <Card
              key={`${cardId}-${i}`}
              cardId={cardId}
              size="md"
            />
          ))}
        </div>
      )}
    </Modal>
  );
}

