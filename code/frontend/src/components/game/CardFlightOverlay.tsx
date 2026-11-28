import '@/components/game/decisions/decisions.css';
import { Card } from '@/components/shared/Card';
import { usePresentationStore } from '@/stores/presentationStore';

/**
 * Lightweight directional card-flight indicator driven by `presentationStore.flight`
 * for draw, mill, trash, deletion, bounce-to-hand, deck-return, and recovery events.
 * Strictly pointer-events-none and zero-blocking.
 */
export function CardFlightOverlay() {
  const flight = usePresentationStore((s) => s.flight);
  const activeBeat = usePresentationStore((s) => s.activeBeat);

  if (!flight) return null;

  return (
    <div
      key={activeBeat?.id ?? `${flight.actor}-${flight.toZone}-${flight.cardId ?? 'back'}`}
      data-testid="card-flight-overlay"
      aria-hidden="true"
      className={`hs-card-flight hs-card-flight--${flight.actor} hs-card-flight--to-${flight.toZone}`}
    >
      <Card
        cardId={flight.cardId ?? ''}
        cardName={flight.cardName ?? undefined}
        faceDown={!flight.cardId}
        size="sm"
      />
      <span className="hs-card-flight__pill">{flight.label}</span>
    </div>
  );
}
