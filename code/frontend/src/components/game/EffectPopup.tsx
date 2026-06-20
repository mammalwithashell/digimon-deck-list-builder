import { useState, useEffect, useRef, useCallback } from 'react';
import { useGameStore } from '@/stores/gameStore';
import type { GameEvent } from '@/types/game';

const DISPLAY_DURATION_MS = 1800;
const STAGGER_DELAY_MS = 300;

interface QueuedEffect {
  event: GameEvent;
  id: number;
}

let nextId = 0;

/**
 * Floating popup that shows effect activation text near the top of the game board.
 * Queues multiple effects and staggers their display.
 */
export function EffectPopup() {
  const events = useGameStore((s) => s.events);
  const [visible, setVisible] = useState<QueuedEffect[]>([]);
  const lastProcessedSeq = useRef(-1);
  const queueRef = useRef<QueuedEffect[]>([]);
  const processingRef = useRef(false);

  const processQueue = useCallback(() => {
    if (processingRef.current || queueRef.current.length === 0) return;
    processingRef.current = true;

    const item = queueRef.current.shift()!;
    setVisible((prev) => [...prev, item]);

    // Auto-remove after duration
    setTimeout(() => {
      setVisible((prev) => prev.filter((v) => v.id !== item.id));
    }, DISPLAY_DURATION_MS);

    // Process next item after stagger delay
    setTimeout(() => {
      processingRef.current = false;
      processQueue();
    }, STAGGER_DELAY_MS);
  }, []);

  useEffect(() => {
    const newEffects = events.filter(
      (e) => e.type === 'effect_activate' && e.seq > lastProcessedSeq.current
    );
    if (newEffects.length === 0) return;

    lastProcessedSeq.current = newEffects[newEffects.length - 1]!.seq;

    // Only queue effects that have meaningful text
    for (const event of newEffects) {
      const text = (event.meta.effect_text as string) || (event.meta.effect_name as string);
      if (text) {
        queueRef.current.push({ event, id: nextId++ });
      }
    }

    processQueue();
  }, [events, processQueue]);

  if (visible.length === 0) return null;

  return (
    <div className="absolute top-16 left-1/2 -translate-x-1/2 z-40 flex flex-col items-center gap-1.5 pointer-events-none">
      {visible.map((item) => (
        <EffectBadge key={item.id} event={item.event} />
      ))}
    </div>
  );
}

function EffectBadge({ event }: { event: GameEvent }) {
  const cardName = (event.meta.card_name as string) || 'Unknown';
  const effectText = (event.meta.effect_text as string) || (event.meta.effect_name as string) || '';
  const isInherited = event.meta.is_inherited as boolean;
  const timing = event.meta.timing as string;

  // Truncate long effect text
  const displayText = effectText.length > 80 ? effectText.slice(0, 77) + '...' : effectText;

  return (
    <div className="animate-effect-popup bg-[var(--ib-graphite)] border border-amber-600/40 rounded-lg px-3 py-1.5 max-w-sm shadow-lg">
      <div className="flex items-center gap-2 text-[11px]">
        <span className="text-amber-400 font-semibold shrink-0">{cardName}</span>
        {isInherited && (
          <span className="text-purple-400 text-[9px] uppercase font-bold shrink-0">
            inherited
          </span>
        )}
        {timing && (
          <span className="text-[var(--ib-bone-dd)] text-[9px] shrink-0">{formatTiming(timing)}</span>
        )}
      </div>
      {displayText && (
        <div className="text-[var(--ib-bone-d)] text-[10px] leading-snug mt-0.5">{displayText}</div>
      )}
    </div>
  );
}

function formatTiming(timing: string): string {
  // Convert enum names to readable labels
  return timing
    .replace(/([A-Z])/g, ' $1')
    .trim()
    .replace(/^On /, '');
}
