import { useEffect } from 'react';
import { useCardImage } from '@/hooks/useCardImage';

interface CardDetailOverlayProps {
  cardId: string | null;
  onClose: () => void;
}

/**
 * DCGO-style CardDetail — fullscreen click-catcher showing one large card
 * image right-of-center over the board (mirrors DCGO's
 * CardDetail.OpenCardDetail zoom-in). Closes on any click, right-click,
 * or Escape.
 */
export function CardDetailOverlay({ cardId, onClose }: CardDetailOverlayProps) {
  const { src, hasError } = useCardImage(cardId);

  useEffect(() => {
    if (!cardId) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [cardId, onClose]);

  if (!cardId) return null;

  return (
    <div
      data-testid="card-detail-overlay"
      className="absolute inset-0 z-40 cursor-pointer"
      onClick={onClose}
      onContextMenu={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <div className="absolute top-1/2 left-[68%] -translate-x-1/2 -translate-y-1/2 animate-card-detail-open pointer-events-none">
        {src && !hasError ? (
          <img
            data-testid="card-detail-image"
            src={src}
            alt={cardId}
            draggable={false}
            className="h-[75vh] max-h-[680px] w-auto rounded-xl shadow-2xl select-none"
          />
        ) : (
          <div className="h-[75vh] max-h-[680px] aspect-[430/600] rounded-xl bg-gray-800 border border-gray-600 flex items-center justify-center text-gray-400 text-sm">
            {cardId}
          </div>
        )}
      </div>
    </div>
  );
}
