import { useEffect } from 'react';
import { createPortal } from 'react-dom';
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
      if (e.key !== 'Escape') return;
      // Capture phase + stopPropagation: this is the top-most layer, so
      // Escape closes only the enlarged detail — not an underlying panel
      // (e.g. the stack inspector) that also listens for Escape.
      e.stopPropagation();
      onClose();
    };
    window.addEventListener('keydown', handleKey, true);
    return () => window.removeEventListener('keydown', handleKey, true);
  }, [cardId, onClose]);

  if (!cardId) return null;

  // Portal to <body> at z-[60] so the enlarged detail is the top-most layer
  // even above other portaled modals (e.g. the TrashViewer Modal at z-50,
  // which is also portaled to body — an inline `absolute z-50` would render
  // BEHIND it). Right-click zoom from the trash must appear OVER the trash.
  return createPortal(
    <div
      data-testid="card-detail-overlay"
      className="fixed inset-0 z-[60] cursor-pointer"
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
          <div className="h-[75vh] max-h-[680px] aspect-[430/600] rounded-xl bg-[var(--ib-graphite)] border border-[var(--ib-line)] flex items-center justify-center text-[var(--ib-bone-dd)] text-sm">
            {cardId}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
