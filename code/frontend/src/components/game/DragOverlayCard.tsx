import { createPortal } from 'react-dom';
import { DragOverlay } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';
import { useUiStore } from '@/stores/uiStore';

interface DragOverlayCardProps {
  cardId: string | null;
  isOverValid: boolean;
}

/**
 * Drag-preview overlay that follows the cursor.
 *
 * Why this is portaled to `document.body`
 * ---------------------------------------
 * Without the portal, the overlay renders inside `<CanvasScaler>`'s
 * `transform: scale(s)` subtree. dnd-kit positions the overlay using
 * `position: fixed; top/left = getBoundingClientRect of activator` —
 * those values are in window pixels, but `position: fixed` inside a
 * transformed ancestor is anchored to the *transformed coordinate
 * system*, so the overlay ends up offset from the cursor by roughly
 * `(1 - s) * delta` (or worse, because `transform-origin: center
 * center` rotates the offset around the canvas midpoint). Result: the
 * card image floats far away from the actual cursor.
 *
 * Portaling escapes the transform entirely — the overlay is now a
 * direct child of `<body>`, so window-pixel coordinates mean what
 * they say. We then rescale the *content* by `canvasScale` (with
 * top-left origin) so the visual card size matches the on-board card
 * size, which itself is scaled down by the canvas. Without that, the
 * drag preview would render at native (canvas-pixel) size while the
 * board renders shrunken — a visual mismatch.
 *
 * Because the overlay lives at body-level, dnd-kit's delta
 * (window-pixel pointer movement) interprets correctly without any
 * compensating modifier on `DndContext`. An earlier attempt added a
 * "divide by canvasScale" modifier; that worked when the overlay was
 * inside the canvas but over-corrects after portaling (overlay moves
 * 1.5× the cursor at 0.667 scale). The modifier was removed.
 */
export function DragOverlayCard({ cardId, isOverValid }: DragOverlayCardProps) {
  const canvasScale = useUiStore((s) => s.canvasScale);

  if (typeof document === 'undefined') return null;

  const overlay = !cardId ? (
    <DragOverlay />
  ) : (
    <DragOverlay>
      <div
        className={`${isOverValid ? 'opacity-80' : 'opacity-40'} pointer-events-none`}
        style={{
          // Match the on-board card's *visual* size: the board is
          // scaled by `canvasScale` via CanvasScaler, so the preview
          // must scale by the same factor. `top left` origin keeps the
          // overlay's top-left corner aligned with the cursor's grab
          // point, which is what dnd-kit positions against.
          transform: `scale(${canvasScale})`,
          transformOrigin: 'top left',
        }}
      >
        <Card cardId={cardId} size="md" />
      </div>
    </DragOverlay>
  );

  return createPortal(overlay, document.body);
}
