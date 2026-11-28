import { GamePhase } from '@/types/game';
import { ACTION, PHASE_NAMES } from '@/utils/constants';

export interface TurnControlOrbProps {
  phase: GamePhase;
  currentPlayer: number;
  actionMask: number[];
  isGameOver: boolean;
  isBusy?: boolean;
  onAction: (actionId: number) => void;
}

/**
 * Hearthstone/Aegis-style circular End-Phase / Pass orb anchored on the right
 * edge of the midline MemoryBand. Keeps `data-testid="action-pass"` intact for
 * existing E2E and unit tests.
 */
export function TurnControlOrb({
  phase,
  currentPlayer,
  actionMask,
  isGameOver,
  isBusy = false,
  onAction,
}: TurnControlOrbProps) {
  if (isGameOver) return null;

  const canPass = !isBusy && actionMask[ACTION.PASS] === 1;
  const isYourTurn = currentPlayer === 1;
  const phaseLabel = (PHASE_NAMES[phase] ?? 'Wait').toUpperCase();

  let orbLabel = isYourTurn ? 'End Phase' : 'Opp Turn';
  if (canPass) {
    if (phase === GamePhase.Breeding) orbLabel = 'Skip Breed';
    else if (phase === GamePhase.Main) orbLabel = 'End Turn';
    else if (phase >= GamePhase.SelectTarget) orbLabel = 'Decline';
    else orbLabel = 'Pass';
  } else if (isBusy) {
    orbLabel = 'Resolving…';
  }

  return (
    <div className="hs-turn-control">
      <div className="hs-phase-pill" data-testid="phase-pill">
        <span className={`hs-phase-pill__dot ${isYourTurn ? 'is-you' : 'is-opp'}`} />
        <span>{phaseLabel}</span>
      </div>
      <button
        type="button"
        data-testid="action-pass"
        disabled={!canPass}
        onClick={() => canPass && onAction(ACTION.PASS)}
        className={`hs-turn-orb ${canPass ? 'is-ready' : 'is-idle'} ${
          isYourTurn ? 'is-you' : 'is-opp'
        }`}
        title={canPass ? orbLabel : 'Waiting for opponent or selection'}
      >
        <span className="hs-turn-orb__ring" aria-hidden="true" />
        <span className="hs-turn-orb__label">{orbLabel}</span>
      </button>
    </div>
  );
}
