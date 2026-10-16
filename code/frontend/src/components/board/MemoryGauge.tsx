import { PHASE_NAMES } from '@/utils/constants';
import type { GamePhase } from '@/types/game';
import type { BoardStyle } from '@/stores/uiStore';

interface MemoryGaugeProps {
  /** Raw memory value from backend (positive = player 1's memory) */
  value: number;
  /** Which player is the local (bottom) player: 1 or 2 */
  localPlayer: number;
  /** Current game phase for the embedded badge */
  currentPhase: GamePhase;
  /** Preview cost — positive means memory moves toward opponent */
  previewCost?: number | null;
  /** `classic`: diamond segments with ticks between 1, 5 and 10.
   *  `arena`: a numbered coin for every step. */
  variant?: BoardStyle;
  /** End-turn handler; when absent the round button renders disabled. */
  onEndTurn?: () => void;
  /** Text on the round end-turn button. */
  endTurnLabel?: string;
}

/** Memory gauge between the two fields, flanked by the phase badge and a
 *  round End Turn button. Classic is the DCGO-style diamond track; arena is
 *  a coin track. Player's memory is always on the LEFT. */
export function MemoryGauge({
  value,
  localPlayer,
  currentPhase,
  previewCost,
  variant = 'classic',
  onEndTurn,
  endTurnLabel = 'End Turn',
}: MemoryGaugeProps) {
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
    <div className="ib-gauge-frame">
      <div className="ib-gauge-pill ib-gauge-pill--phase">
        <span>Phase</span>
        {badgeText || 'WAIT'}
      </div>

      <div className="ib-gauge-bar" aria-label={`Memory ${orientedValue}`}>
        {variant === 'classic' && <div className="ib-gauge-label">Memory</div>}
        <div className="ib-gauge-segments">
          {leftSegments.map((seg) => {
            const isActive = orientedValue >= seg;
            const isExact = orientedValue === seg;
            const isPreviewTarget = previewValue != null && previewValue === seg;
            const isPreviewTrail = previewValue != null && previewValue > orientedValue
              && seg > orientedValue && seg <= previewValue;
            return (
              <GaugeSegment
                key={`L${seg}`}
                number={seg}
                isActive={isActive}
                isExact={isExact}
                side="player"
                isPreviewTarget={isPreviewTarget}
                isPreviewTrail={isPreviewTrail}
                allNumbered={variant === 'arena'}
              />
            );
          })}

          <GaugeSegment
            number={0}
            isActive={orientedValue === 0}
            isExact={orientedValue === 0}
            side="zero"
            isPreviewTarget={previewValue === 0}
            allNumbered={variant === 'arena'}
          />

          {rightSegments.map((seg) => {
            const isActive = orientedValue <= -seg;
            const isExact = orientedValue === -seg;
            const isPreviewTarget = previewValue != null && previewValue === -seg;
            const isPreviewTrail = previewValue != null && previewValue < orientedValue
              && -seg < orientedValue && -seg >= previewValue;
            return (
              <GaugeSegment
                key={`R${seg}`}
                number={seg}
                isActive={isActive}
                isExact={isExact}
                side="opponent"
                isPreviewTarget={isPreviewTarget}
                isPreviewTrail={isPreviewTrail}
                allNumbered={variant === 'arena'}
              />
            );
          })}
        </div>
      </div>

      <button
        type="button"
        data-testid="board-end-turn"
        className="ib-end-turn"
        disabled={!onEndTurn}
        onClick={onEndTurn}
      >
        {endTurnLabel}
      </button>
    </div>
  );
}

interface GaugeSegmentProps {
  number: number;
  isActive: boolean;
  isExact: boolean;
  side: 'player' | 'opponent' | 'zero';
  isPreviewTarget?: boolean;
  isPreviewTrail?: boolean;
  /** Arena coins show every number; classic shows ticks between 1, 5, 10. */
  allNumbered?: boolean;
}

function GaugeSegment({ number, isActive, isExact, side, isPreviewTarget, isPreviewTrail, allNumbered }: GaugeSegmentProps) {
  const showNumber = allNumbered
    || number === 0 || number % 5 === 0 || number === 1 || isExact || isPreviewTarget;

  return (
    <div
      className={[
        'ib-gauge-seg',
        `ib-gauge-seg--${side}`,
        isActive ? 'is-active' : '',
        isExact ? 'is-focal' : '',
        isPreviewTarget ? 'is-ghost' : '',
        isPreviewTrail ? 'is-trail' : '',
        showNumber ? 'is-numbered' : 'is-tick',
      ].filter(Boolean).join(' ')}
    >
      {showNumber ? number : ''}
    </div>
  );
}
