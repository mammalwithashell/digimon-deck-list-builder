import { PHASE_NAMES } from '@/utils/constants';
import type { GamePhase } from '@/types/game';

interface MemoryGaugeProps {
  /** Raw memory value from backend (positive = player 1's memory) */
  value: number;
  /** Which player is the local (bottom) player: 1 or 2 */
  localPlayer: number;
  /** Current game phase for the embedded badge */
  currentPhase: GamePhase;
  /** Preview cost — positive means memory moves toward opponent */
  previewCost?: number | null;
}

/** DCGO-style diamond memory gauge. Player's memory is always on the LEFT. */
export function MemoryGauge({ value, localPlayer, currentPhase, previewCost }: MemoryGaugeProps) {
  // Normalize: positive = local player has memory (shown on left)
  const orientedValue = localPlayer === 1 ? value : -value;

  // Preview: subtract cost from oriented value (cost moves memory toward opponent = negative)
  const previewValue = previewCost != null ? orientedValue - previewCost : null;

  // Segments: 10..1 (player side, left) | 0 | 1..10 (opponent side, right)
  const leftSegments = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
  const rightSegments = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

  const phaseName = PHASE_NAMES[currentPhase] ?? '';
  // Show major gameplay phases on the badge
  const showPhaseBadge = currentPhase <= 4 || currentPhase === 17;
  const badgeText = showPhaseBadge ? phaseName.toUpperCase() : '';

  return (
    <div className="flex items-center justify-center gap-0 py-3 px-2 select-none">
      {/* Player side (left) — orange/red */}
      {leftSegments.map((seg) => {
        const isActive = orientedValue >= seg;
        const isExact = orientedValue === seg;
        const isPreviewTarget = previewValue != null && previewValue === seg;
        // Ghost trail: segments between current and preview that will become active/inactive
        const isPreviewTrail = previewValue != null && previewValue > orientedValue
          && seg > orientedValue && seg <= previewValue;
        return (
          <Diamond
            key={`L${seg}`}
            number={seg}
            isActive={isActive}
            isExact={isExact}
            side="player"
            isPreviewTarget={isPreviewTarget}
            isPreviewTrail={isPreviewTrail}
          />
        );
      })}

      {/* Center zero + phase badge */}
      <div className="flex flex-col items-center mx-1 relative">
        <div
          className={`w-7 h-7 rotate-45 flex items-center justify-center rounded-sm border-2 ${
            orientedValue === 0
              ? 'bg-yellow-400 border-yellow-300 shadow-[0_0_12px_rgba(250,204,21,0.6)]'
              : previewValue === 0
                ? 'bg-yellow-400/40 border-yellow-300/50 animate-pulse'
                : 'bg-gray-600 border-gray-500'
          }`}
        >
          <span
            className={`-rotate-45 text-[10px] font-black ${
              orientedValue === 0 ? 'text-gray-900' : previewValue === 0 ? 'text-yellow-300' : 'text-gray-300'
            }`}
          >
            0
          </span>
        </div>
        {badgeText && (
          <div className="absolute -bottom-5 whitespace-nowrap px-2 py-0.5 bg-gray-900/90 border border-gray-600 rounded text-[9px] font-bold text-amber-300 tracking-wider">
            {badgeText}
          </div>
        )}
      </div>

      {/* Opponent side (right) — blue */}
      {rightSegments.map((seg) => {
        const isActive = orientedValue <= -seg;
        const isExact = orientedValue === -seg;
        const isPreviewTarget = previewValue != null && previewValue === -seg;
        // Ghost trail: segments between current and preview that will become active
        const isPreviewTrail = previewValue != null && previewValue < orientedValue
          && -seg < orientedValue && -seg >= previewValue;
        return (
          <Diamond
            key={`R${seg}`}
            number={seg}
            isActive={isActive}
            isExact={isExact}
            side="opponent"
            isPreviewTarget={isPreviewTarget}
            isPreviewTrail={isPreviewTrail}
          />
        );
      })}
    </div>
  );
}

interface DiamondProps {
  number: number;
  isActive: boolean;
  isExact: boolean;
  side: 'player' | 'opponent';
  isPreviewTarget?: boolean;
  isPreviewTrail?: boolean;
}

function Diamond({ number, isActive, isExact, side, isPreviewTarget, isPreviewTrail }: DiamondProps) {
  const playerColors = {
    active: 'bg-orange-500 border-orange-400',
    inactive: 'bg-orange-900/30 border-orange-800/40',
  };
  const opponentColors = {
    active: 'bg-blue-500 border-blue-400',
    inactive: 'bg-blue-900/30 border-blue-800/40',
  };

  const colors = side === 'player' ? playerColors : opponentColors;
  const colorClass = isActive ? colors.active : colors.inactive;
  const glowClass = isExact
    ? 'shadow-[0_0_12px_rgba(250,204,21,0.6)] ring-1 ring-yellow-300'
    : '';
  const previewClass = isPreviewTarget
    ? 'ring-2 ring-amber-400 shadow-[0_0_14px_rgba(251,191,36,0.7)] animate-pulse'
    : isPreviewTrail
      ? 'ring-1 ring-amber-400/50'
      : '';
  const showNumber = number % 5 === 0 || number === 1 || isExact || isPreviewTarget;

  return (
    <div className="flex flex-col items-center mx-[-2px]">
      <div
        className={`w-5 h-5 rotate-45 flex items-center justify-center rounded-[2px] border ${colorClass} ${glowClass} ${previewClass} transition-colors duration-200`}
      >
        {showNumber && (
          <span
            className={`-rotate-45 text-[8px] font-bold ${
              isPreviewTarget ? 'text-amber-300' : isActive ? 'text-white' : 'text-gray-500'
            }`}
          >
            {number}
          </span>
        )}
      </div>
    </div>
  );
}
