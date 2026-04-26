import { useEffect, useState, useRef } from 'react';
import { GamePhase } from '@/types/game';
import { PHASE_NAMES } from '@/utils/constants';

interface PhaseBannerProps {
  phase: GamePhase;
  isGameOver: boolean;
}

/** Phases worth showing a full-screen banner for */
const BANNER_PHASES = new Set<GamePhase>([
  GamePhase.Breeding,
  GamePhase.Main,
  GamePhase.BlockTiming,
  GamePhase.CounterTiming,
  GamePhase.AllianceTiming,
]);

const PHASE_COLORS: Partial<Record<GamePhase, string>> = {
  [GamePhase.Breeding]: 'from-green-600/90 to-green-800/90 text-green-100',
  [GamePhase.Main]: 'from-blue-600/90 to-blue-800/90 text-blue-100',
  [GamePhase.BlockTiming]: 'from-red-600/90 to-red-800/90 text-red-100',
  [GamePhase.CounterTiming]: 'from-orange-600/90 to-orange-800/90 text-orange-100',
  [GamePhase.AllianceTiming]: 'from-indigo-600/90 to-indigo-800/90 text-indigo-100',
};

export function PhaseBanner({ phase, isGameOver }: PhaseBannerProps) {
  const [visible, setVisible] = useState(false);
  const [displayPhase, setDisplayPhase] = useState<GamePhase>(phase);
  const prevPhase = useRef(phase);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (isGameOver) return;
    if (phase === prevPhase.current) return;
    prevPhase.current = phase;

    if (!BANNER_PHASES.has(phase)) return;

    setDisplayPhase(phase);
    setVisible(true);

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setVisible(false), 1200);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [phase, isGameOver]);

  if (!visible) return null;

  const colorClass = PHASE_COLORS[displayPhase] ?? 'from-gray-600/90 to-gray-800/90 text-gray-100';
  const phaseName = PHASE_NAMES[displayPhase] ?? 'Unknown';

  return (
    <div
      data-testid="phase-banner"
      className="fixed inset-0 z-30 flex items-center justify-center pointer-events-none"
    >
      <div
        className={`px-16 py-4 bg-gradient-to-r ${colorClass} rounded-lg shadow-2xl
          animate-[bannerSlide_1.2s_ease-out_forwards]`}
      >
        <span className="text-3xl font-extrabold tracking-widest uppercase">
          {phaseName}
        </span>
      </div>
    </div>
  );
}

