import { useEffect, useRef } from 'react';
import { useGameStore } from '@/stores/gameStore';

const HIGHLIGHT_DURATION_MS = 1500;

/**
 * Watches for effect_activate events and sets the active effect slot
 * in the store. Automatically clears after HIGHLIGHT_DURATION_MS.
 */
export function useEffectHighlight() {
  const events = useGameStore((s) => s.events);
  const setActiveEffect = useGameStore((s) => s.setActiveEffect);
  const lastProcessedSeq = useRef(-1);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const effectEvents = events.filter(
      (e) => e.type === 'effect_activate' && e.seq > lastProcessedSeq.current
    );
    if (effectEvents.length === 0) return;

    const latest = effectEvents[effectEvents.length - 1]!;
    lastProcessedSeq.current = latest.seq;

    if (latest.source_slot != null) {
      setActiveEffect(latest.source_slot, latest.player);

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        setActiveEffect(null, null);
      }, HIGHLIGHT_DURATION_MS);
    }
  }, [events, setActiveEffect]);
}
