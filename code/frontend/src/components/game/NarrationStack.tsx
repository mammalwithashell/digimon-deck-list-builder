import { Card } from '@/components/shared/Card';
import { usePresentationStore } from '@/stores/presentationStore';

/**
 * Scrollable non-blocking recent-effect toast column (Aegis NarrationStack
 * pattern): keeps up to 6 recent plays/effects readable for ~5s without
 * blocking clicks or slowing down the chain.
 */
export function NarrationStack() {
  const toasts = usePresentationStore((s) => s.toasts);
  if (toasts.length === 0) return null;

  return (
    <aside
      data-testid="narration-stack"
      className="hs-narration-stack"
      aria-live="polite"
      aria-label="Recent match effects"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`hs-narration-toast hs-narration-toast--${toast.actor}`}
        >
          {toast.cardId && <Card cardId={toast.cardId} size="sm" />}
          <div className="min-w-0">
            <div className="hs-narration-toast__title">{toast.title}</div>
            <div className="hs-narration-toast__body">{toast.body}</div>
          </div>
        </div>
      ))}
    </aside>
  );
}
