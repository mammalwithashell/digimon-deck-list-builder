import { PHASE_NAMES } from '@/utils/constants';
import type { GamePhase } from '@/types/game';

interface PhaseIndicatorProps {
  phase: GamePhase;
  turnCount: number;
  currentPlayer: number;
  isGameOver: boolean;
  winner: number | null;
  playerLabels: Record<number, string>;
}

export function PhaseIndicator({
  phase,
  turnCount,
  currentPlayer,
  isGameOver,
  winner,
  playerLabels,
}: PhaseIndicatorProps) {
  const getLabel = (playerId: number) => playerLabels[playerId] ?? `Player ${playerId}`;

  if (isGameOver) {
    return (
      <div data-testid="phase-indicator" className="flex items-center gap-3 px-3 py-1.5 bg-[var(--ib-graphite)] border border-[var(--ib-warn)] rounded">
        <span className="text-sm font-bold text-[var(--ib-warn)]">
          Game Over — {getLabel(winner!)} wins!
        </span>
      </div>
    );
  }

  return (
    <div data-testid="phase-indicator" className="flex items-center gap-3 px-3 py-1.5 bg-[var(--ib-graphite)] border border-[var(--ib-line)] rounded">
      <span data-testid="turn-count" className="text-xs text-[var(--ib-bone-dd)]">Turn {turnCount}</span>
      <span className="text-xs text-[var(--ib-bone-d)]">
        {getLabel(currentPlayer)}'s turn
      </span>
      <span data-testid="current-phase" className="text-xs font-medium text-[var(--ib-opp)]">
        {PHASE_NAMES[phase] ?? `Phase ${phase}`}
      </span>
    </div>
  );
}
