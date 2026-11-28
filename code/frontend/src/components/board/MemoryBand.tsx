import { GamePhase } from '@/types/game';
import { TurnControlOrb } from './TurnControlOrb';

export interface MemoryBandProps {
  /** Raw memory value from backend (positive = player 1's memory). */
  value: number;
  localPlayer: number;
  currentPhase: GamePhase;
  currentPlayer: number;
  actionMask: number[];
  isGameOver: boolean;
  isBusy?: boolean;
  /** Memory cost preview (positive = costs memory toward opponent). */
  previewCost?: number | null;
  /** Optional short floating delta badge (e.g., "+2", "-3") from presentation queue. */
  memoryDelta?: number | null;
  onAction: (actionId: number) => void;
}

const LEFT_SEGMENTS = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
const RIGHT_SEGMENTS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/**
 * Centered single-row circular memory gauge + Hearthstone-style End Phase orb.
 */
export function MemoryBand({
  value,
  localPlayer,
  currentPhase,
  currentPlayer,
  actionMask,
  isGameOver,
  isBusy = false,
  previewCost,
  memoryDelta,
  onAction,
}: MemoryBandProps) {
  const orientedValue = Math.max(-10, Math.min(10, localPlayer === 1 ? value : -value));
  const previewValue =
    previewCost != null ? Math.max(-10, Math.min(10, orientedValue - previewCost)) : null;

  return (
    <div className="hs-memory-band" aria-label={`Memory ${orientedValue}`}>
      <div className="hs-memory-track">
        {LEFT_SEGMENTS.map((seg) => {
          const isExact = orientedValue === seg;
          const isFilled = orientedValue >= seg;
          const isGhost = previewValue === seg;
          const isTrail =
            previewValue != null &&
            ((previewValue < orientedValue && seg < orientedValue && seg > previewValue) ||
              (previewValue > orientedValue && seg > orientedValue && seg < previewValue));
          return (
            <span
              key={`L${seg}`}
              className={`hs-mem-bead hs-mem-bead--player${isFilled ? ' is-filled' : ''}${
                isExact ? ' is-active' : ''
              }${isGhost ? ' is-ghost' : ''}${isTrail ? ' is-trail' : ''}`}
            >
              {seg}
            </span>
          );
        })}

        <span
          className={`hs-mem-bead hs-mem-bead--zero${orientedValue === 0 ? ' is-active' : ''}${
            previewValue === 0 ? ' is-ghost' : ''
          }`}
        >
          0
        </span>

        {RIGHT_SEGMENTS.map((seg) => {
          const signed = -seg;
          const isExact = orientedValue === signed;
          const isFilled = orientedValue <= signed;
          const isGhost = previewValue === signed;
          const isTrail =
            previewValue != null &&
            ((previewValue < orientedValue && signed < orientedValue && signed > previewValue) ||
              (previewValue > orientedValue && signed > orientedValue && signed < previewValue));
          return (
            <span
              key={`R${seg}`}
              className={`hs-mem-bead hs-mem-bead--opp${isFilled ? ' is-filled' : ''}${
                isExact ? ' is-active' : ''
              }${isGhost ? ' is-ghost' : ''}${isTrail ? ' is-trail' : ''}`}
            >
              {seg}
            </span>
          );
        })}

        {memoryDelta != null && memoryDelta !== 0 && (
          <span
            className={`hs-mem-delta ${memoryDelta > 0 ? 'is-gain' : 'is-loss'}`}
            aria-live="polite"
          >
            {memoryDelta > 0 ? `+${memoryDelta}` : memoryDelta}
          </span>
        )}
      </div>

      <TurnControlOrb
        phase={currentPhase}
        currentPlayer={currentPlayer}
        actionMask={actionMask}
        isGameOver={isGameOver}
        isBusy={isBusy}
        onAction={onAction}
      />
    </div>
  );
}
