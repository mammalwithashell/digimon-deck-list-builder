import { ACTION, PHASE_NAMES } from '@/utils/constants';
import type { GamePhase } from '@/types/game';

interface ActionBarProps {
  phase: GamePhase;
  actionMask: number[];
  onAction: (actionId: number) => void;
  isGameOver: boolean;
}

export function ActionBar({ phase, actionMask, onAction, isGameOver }: ActionBarProps) {
  if (isGameOver) return null;

  const canPass = actionMask[ACTION.PASS] === 1;
  const canHatch = actionMask[ACTION.HATCH] === 1;
  const canMove = actionMask[ACTION.MOVE] === 1;

  const phaseName = PHASE_NAMES[phase] ?? 'Unknown';

  return (
    <div className="flex items-center gap-2 px-3 py-2 bg-gray-800 border-t border-gray-700">
      <span className="text-xs text-gray-500">{phaseName}:</span>

      {canHatch && (
        <button
          onClick={() => onAction(ACTION.HATCH)}
          className="px-3 py-1 bg-yellow-600 hover:bg-yellow-500 text-white text-sm rounded"
        >
          Hatch
        </button>
      )}

      {canMove && (
        <button
          onClick={() => onAction(ACTION.MOVE)}
          className="px-3 py-1 bg-green-600 hover:bg-green-500 text-white text-sm rounded"
        >
          Move
        </button>
      )}

      {canPass && (
        <button
          onClick={() => onAction(ACTION.PASS)}
          className="px-3 py-1 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
        >
          {phase === 2 ? 'Skip Breeding' : phase >= 5 ? 'Decline' : 'Pass'}
        </button>
      )}
    </div>
  );
}
