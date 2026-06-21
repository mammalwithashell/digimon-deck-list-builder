import { useCallback, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { DeckBuilderWorkbench } from '@/pages/DeckBuilderPage';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import './DeckEditOverlay.css';

interface DeckEditOverlayProps {
  isOpen: boolean;
  /** Deck to edit. The overlay renders nothing until both isOpen and a deckId. */
  deckId: string | null;
  /** Close without saving (the unsaved-changes guard lives here). */
  onClose: () => void;
  /** Called after a successful save with the persisted deck id. */
  onSaved: (deckId: string) => void;
}

/**
 * Renders the full deck builder ({@link DeckBuilderWorkbench}) inside an overlay
 * over the play flow (the deck-edit-before-queue capability). The unsaved-changes
 * guard is centralized here so it covers every close path — the builder's
 * CANCEL/DONE buttons, the Escape key, and a backdrop click — without prompting
 * twice. Saving routes through `onSaved` (the play flow refreshes + closes);
 * `isDirty` is read from the shared builder store the workbench writes to.
 */
export function DeckEditOverlay({ isOpen, deckId, onClose, onSaved }: DeckEditOverlayProps) {
  const isDirty = useDeckBuilderStore((state) => state.isDirty);

  const guardedClose = useCallback(() => {
    if (isDirty && !window.confirm('Discard unsaved changes to this deck?')) return;
    onClose();
  }, [isDirty, onClose]);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') guardedClose();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [isOpen, guardedClose]);

  if (!isOpen || !deckId) return null;

  return createPortal(
    <div className="deck-edit-overlay-backdrop" onClick={guardedClose}>
      <div
        className="deck-edit-overlay-frame"
        role="dialog"
        aria-modal="true"
        aria-label="Edit deck"
        onClick={(event) => event.stopPropagation()}
      >
        <DeckBuilderWorkbench
          embedded
          initialDeckId={deckId}
          onClose={guardedClose}
          onSaved={onSaved}
        />
      </div>
    </div>,
    document.body,
  );
}
