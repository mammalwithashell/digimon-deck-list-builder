import { useEffect } from 'react';
import { useUiStore } from '@/stores/uiStore';
import { GraphicsSettingsPanel } from './GraphicsSettingsPanel';

export const GRAPHICS_MODAL_ID = 'graphics';

/**
 * Centered Graphics-settings modal (add-desktop-ui-polish). Opened from the
 * nav-rail Graphics item and the title-bar View menu via
 * `useUiStore.openModal('graphics')`, replacing the old standalone
 * /settings/graphics route. Closes on backdrop click, the close button, or Esc.
 *
 * Mounted at the App root OUTSIDE CanvasScaler so its fixed positioning is in
 * real viewport pixels (not the scaled canvas coordinate space).
 */
export function GraphicsSettingsModal() {
  const open = useUiStore((s) => s.activeModal === GRAPHICS_MODAL_ID);
  const closeModal = useUiStore((s) => s.closeModal);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeModal();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open, closeModal]);

  if (!open) return null;

  return (
    <div
      className="ds-modal-backdrop"
      role="presentation"
      onClick={closeModal}
      data-testid="graphics-modal-backdrop"
    >
      <div
        className="ds-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Graphics settings"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          type="button"
          className="ds-modal__close"
          aria-label="Close"
          title="Close"
          onClick={closeModal}
        >
          ✕
        </button>
        <GraphicsSettingsPanel />
      </div>
    </div>
  );
}
