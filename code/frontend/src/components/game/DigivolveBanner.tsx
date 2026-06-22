import { useEffect, useState, useRef } from 'react';
import { Card } from '@/components/shared/Card';
import { useGameStore } from '@/stores/gameStore';
import { useMotion } from '@/stores/uiStore';
import { COLOR_HEX, COLOR_NAMES } from '@/utils/constants';
import type { GameEvent } from '@/types/game';

interface DigivolveDisplay {
  cardId: string;
  cardName: string;
  colorIndex: number;
}

export function DigivolveBanner() {
  const events = useGameStore((s) => s.events);
  const motion = useMotion();
  const [display, setDisplay] = useState<DigivolveDisplay | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const lastSeqRef = useRef(-1);
  // Read the current motion level inside the (events-only) effect without
  // re-running it when motion changes mid-show (which would clear the dismiss
  // timer). The rendered variant still tracks `motion` directly.
  const motionRef = useRef(motion);
  motionRef.current = motion;

  useEffect(() => {
    // Find the most recent digivolve event we haven't shown yet (unchanged
    // trigger + dedupe + lifecycle — only the visual is upgraded).
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
    // Full cut-in needs a touch longer to breathe; off is brief.
    const duration =
      motionRef.current === 'full' ? 1700 : motionRef.current === 'reduced' ? 1400 : 1000;
    timerRef.current = setTimeout(() => setDisplay(null), duration);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [events]);

  if (!display) return null;

  const colorName = COLOR_NAMES[display.colorIndex] ?? 'Yellow';
  const glowColor = COLOR_HEX[colorName] ?? '#eab308';

  const overlayProps = {
    className:
      'fixed inset-0 z-40 flex items-center justify-center pointer-events-none',
    onClick: () => setDisplay(null),
  };

  // Off: a static reveal — no animated cut-in, just the result briefly.
  if (motion === 'off') {
    return (
      <div {...overlayProps}>
        <div
          className="flex flex-col items-center gap-2 px-10 py-5 rounded-xl bg-gray-900/95 border border-white/10"
          style={{ boxShadow: `0 0 24px ${glowColor}40` }}
        >
          <span
            className="text-xl font-black tracking-[0.2em] uppercase"
            style={{ color: glowColor }}
          >
            Digivolve!
          </span>
          <Card cardId={display.cardId} size="lg" />
          {display.cardName && (
            <span className="text-sm font-bold text-white/80 tracking-wide">
              {display.cardName}
            </span>
          )}
        </div>
      </div>
    );
  }

  // Reduced: the simpler legacy banner (still identifies what digivolved).
  if (motion === 'reduced') {
    return (
      <div {...overlayProps}>
        <div
          className="flex flex-col items-center gap-3 px-12 py-6 rounded-xl
                      bg-gradient-to-r from-gray-900/95 via-gray-800/95 to-gray-900/95
                      border border-white/10 shadow-2xl animate-digivolve-banner"
          style={
            {
              '--glow-color': `${glowColor}80`,
              boxShadow: `0 0 40px 8px ${glowColor}40`,
            } as React.CSSProperties
          }
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

  // Full: the multi-phase cut-in (wireframe → orbit ring → flash reveal).
  return (
    <div {...overlayProps}>
      <div className="dgv-cutin" style={{ '--cut-color': glowColor } as React.CSSProperties}>
        <div className="dgv-cutin__stage">
          <div className="dgv-cutin__ring" />
          <div className="dgv-cutin__art">
            <Card cardId={display.cardId} size="lg" />
          </div>
          <div className="dgv-cutin__wire" />
          <div className="dgv-cutin__flash" />
          <div className="dgv-cutin__beam" />
        </div>
        <div className="dgv-cutin__label">Digivolve</div>
        {display.cardName && (
          <div className="dgv-cutin__name">{display.cardName}</div>
        )}
      </div>
    </div>
  );
}
