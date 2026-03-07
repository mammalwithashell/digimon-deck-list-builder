import { useEffect, useState, useRef } from 'react';
import { Card } from '@/components/shared/Card';
import { useGameStore } from '@/stores/gameStore';
import { COLOR_HEX, COLOR_NAMES } from '@/utils/constants';
import type { GameEvent } from '@/types/game';

interface DigivolveDisplay {
  cardId: string;
  cardName: string;
  colorIndex: number;
}

export function DigivolveBanner() {
  const events = useGameStore((s) => s.events);
  const [display, setDisplay] = useState<DigivolveDisplay | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const lastSeqRef = useRef(-1);

  useEffect(() => {
    // Find the most recent digivolve event we haven't shown yet
    const digivolveEvent = [...events]
      .reverse()
      .find((e: GameEvent) => e.type === 'digivolve' && e.seq > lastSeqRef.current);

    if (!digivolveEvent) return;
    lastSeqRef.current = digivolveEvent.seq;

    const cardId = digivolveEvent.source_card_id ?? '';
    const cardName = (digivolveEvent.meta?.card_name as string) ?? '';
    // Determine color from the player state after digivolve
    const player1 = useGameStore.getState().player1;
    const player2 = useGameStore.getState().player2;
    const slot = digivolveEvent.source_slot;
    let colorIndex = 0;
    if (slot != null) {
      const perm = (digivolveEvent.player === 1 ? player1 : player2)?.battleArea[slot];
      if (perm?.colors?.[0] != null) {
        colorIndex = perm.colors[0];
      }
    }

    setDisplay({ cardId, cardName, colorIndex });

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setDisplay(null), 1400);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [events]);

  if (!display) return null;

  const colorName = COLOR_NAMES[display.colorIndex] ?? 'Yellow';
  const glowColor = COLOR_HEX[colorName] ?? '#eab308';

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center pointer-events-none"
      onClick={() => setDisplay(null)}
    >
      <div
        className="flex flex-col items-center gap-3 px-12 py-6 rounded-xl
                    bg-gradient-to-r from-gray-900/95 via-gray-800/95 to-gray-900/95
                    border border-white/10 shadow-2xl animate-digivolve-banner"
        style={{
          '--glow-color': `${glowColor}80`,
          boxShadow: `0 0 40px 8px ${glowColor}40`,
        } as React.CSSProperties}
      >
        <span
          className="text-2xl font-black tracking-[0.2em] uppercase"
          style={{ color: glowColor }}
        >
          Digivolve!
        </span>
        <div className="animate-digivolve-card-drop">
          <Card cardId={display.cardId} size="lg" />
        </div>
        {display.cardName && (
          <span className="text-sm font-bold text-white/80 tracking-wide">
            {display.cardName}
          </span>
        )}
      </div>
    </div>
  );
}
