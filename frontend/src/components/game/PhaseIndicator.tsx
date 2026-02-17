import { PHASE_NAMES } from '@/utils/constants';
import type { GamePhase } from '@/types/game';

interface PhaseIndicatorProps {
  phase: GamePhase;
  turnCount: number;
  currentPlayer: number;
  isGameOver: boolean;
  winner: number | null;
}

export function PhaseIndicator({
  phase,
  turnCount,
  currentPlayer,
  isGameOver,
  winner,
}: PhaseIndicatorProps) {
  if (isGameOver) {
    return (
      <div className="flex items-center gap-3 px-3 py-1.5 bg-yellow-900/30 border border-yellow-700/50 rounded">
        <span className="text-sm font-bold text-yellow-300">
          Game Over — Player {winner} wins!
        </span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3 px-3 py-1.5 bg-gray-800/50 rounded">
      <span className="text-xs text-gray-500">Turn {turnCount}</span>
      <span className="text-xs text-gray-400">
        Player {currentPlayer}'s turn
      </span>
      <span className="text-xs font-medium text-blue-400">
        {PHASE_NAMES[phase] ?? `Phase ${phase}`}
      </span>
    </div>
  );
}
