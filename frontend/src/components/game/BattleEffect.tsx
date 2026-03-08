import { useEffect, useState, useRef } from 'react';
import { useGameStore } from '@/stores/gameStore';
import type { GameEvent } from '@/types/game';

interface BattleDisplay {
  loserSlot: number;
  loserPlayer: number;
  result: string;
}

export function BattleEffect() {
  const events = useGameStore((s) => s.events);
  const [display, setDisplay] = useState<BattleDisplay | null>(null);
  const lastSeqRef = useRef(-1);
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    const battleEvent = [...events]
      .reverse()
      .find((e: GameEvent) => e.type === 'battle_result' && e.seq > lastSeqRef.current);

    if (!battleEvent) return;
    lastSeqRef.current = battleEvent.seq;

    const result = (battleEvent.meta?.result as string) ?? 'tie';
    if (result === 'tie') return; // No animation on tie

    // Determine loser slot
    let loserSlot: number;
    let loserPlayer: number;
    if (result === 'attacker_wins') {
      // Defender loses — target_slot
      loserSlot = battleEvent.target_slot ?? -1;
      // Defender is the opponent (non-turn player)
      loserPlayer = battleEvent.player === 1 ? 2 : 1;
    } else {
      // Attacker loses — source_slot
      loserSlot = battleEvent.source_slot ?? -1;
      loserPlayer = battleEvent.player;
    }

    if (loserSlot < 0) return;

    setDisplay({ loserSlot, loserPlayer, result });

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setDisplay(null), 500);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [events]);

  useEffect(() => {
    if (!display) return;

    // Find the DOM element for the loser's slot and apply shake
    const side = display.loserPlayer === 1 ? 'player' : 'opponent';
    const slotEl = document.querySelector(
      `[data-slot-id="${side}-${display.loserSlot}"]`
    );
    if (slotEl) {
      slotEl.classList.add('animate-battle-shake');
      const cleanup = setTimeout(() => slotEl.classList.remove('animate-battle-shake'), 500);
      return () => {
        clearTimeout(cleanup);
        slotEl.classList.remove('animate-battle-shake');
      };
    }
  }, [display]);

  if (!display) return null;

  // Position the slash overlay over the loser's slot using DOM lookup
  const side = display.loserPlayer === 1 ? 'player' : 'opponent';
  const slotEl = document.querySelector(
    `[data-slot-id="${side}-${display.loserSlot}"]`
  );

  if (!slotEl) return null;

  const rect = slotEl.getBoundingClientRect();

  return (
    <div
      className="fixed z-30 pointer-events-none"
      style={{
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
      }}
    >
      {/* Diagonal slash line */}
      <div
        className="absolute inset-0 animate-battle-slash"
        style={{
          background: 'linear-gradient(135deg, transparent 35%, rgba(255, 100, 100, 0.8) 48%, rgba(255, 255, 255, 0.9) 50%, rgba(255, 100, 100, 0.8) 52%, transparent 65%)',
        }}
      />
    </div>
  );
}
