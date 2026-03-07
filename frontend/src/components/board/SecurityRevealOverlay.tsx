import { useState, useEffect, useRef } from 'react';
import { useGameStore } from '@/stores/gameStore';
import { Card } from '@/components/shared/Card';
import type { GameEvent } from '@/types/game';

const DISPLAY_DURATION_MS = 2000;

/**
 * Full-screen overlay that shows a security card when it's revealed.
 * Auto-dismisses after 2s or on click.
 */
export function SecurityRevealOverlay() {
  const events = useGameStore((s) => s.events);
  const [revealEvent, setRevealEvent] = useState<GameEvent | null>(null);
  const lastProcessedSeq = useRef(-1);

  useEffect(() => {
    // Find the latest security_reveal event we haven't shown yet
    const securityReveals = events.filter(
      (e) => e.type === 'security_reveal' && e.seq > lastProcessedSeq.current
    );
    if (securityReveals.length > 0) {
      const latest = securityReveals[securityReveals.length - 1]!;
      lastProcessedSeq.current = latest.seq;
      setRevealEvent(latest);
    }
  }, [events]);

  useEffect(() => {
    if (!revealEvent) return;
    const timer = setTimeout(() => setRevealEvent(null), DISPLAY_DURATION_MS);
    return () => clearTimeout(timer);
  }, [revealEvent]);

  if (!revealEvent) return null;

  const cardId = (revealEvent.meta.revealed_card_id as string) || revealEvent.source_card_id;
  const cardName = (revealEvent.meta.revealed_card_name as string) || 'Unknown';
  const securityRemaining = revealEvent.meta.security_remaining as number;

  if (!cardId) return null;

  return (
    <div
      className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 animate-fade-in cursor-pointer"
      onClick={() => setRevealEvent(null)}
    >
      <div className="flex flex-col items-center gap-3 animate-security-reveal">
        <div className="text-amber-400 text-sm font-bold tracking-wider uppercase">
          Security Check!
        </div>
        <Card cardId={cardId} cardName={cardName} size="xl" />
        <div className="text-gray-300 text-xs">
          {cardName}
        </div>
        <div className="text-gray-500 text-[10px]">
          {securityRemaining} security remaining — click to dismiss
        </div>
      </div>
    </div>
  );
}
