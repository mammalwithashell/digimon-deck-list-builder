import { Card } from '@/components/shared/Card';
import { usePresentationStore } from '@/stores/presentationStore';

/**
 * Brief centered card reveal when the opponent plays, digivolves, or triggers
 * a security check so the human can actually see the card before it settles.
 */
export function OpponentShowcaseOverlay() {
  const showcase = usePresentationStore((s) => s.showcase);
  if (!showcase) return null;

  return (
    <div
      data-testid="opponent-showcase"
      className="hs-opp-showcase"
      role="status"
      aria-live="polite"
    >
      <span className="hs-opp-showcase__label">{showcase.caption}</span>
      <Card cardId={showcase.cardId} cardName={showcase.cardName} size="lg" />
      <span className="text-xs font-semibold text-[var(--ink-1)]">{showcase.cardName}</span>
    </div>
  );
}
