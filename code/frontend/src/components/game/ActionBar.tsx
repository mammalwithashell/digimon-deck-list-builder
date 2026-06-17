import { ACTION, PHASE_NAMES, EFFECTS_PER_SOURCE } from '@/utils/constants';
import { GamePhase, type DecodedAction } from '@/types/game';
import {
  isActivatableEffect,
  activatableEffectLabel,
  effectTooltip,
  effectNameCounts,
} from '@/utils/effectLabel';

interface ActionBarProps {
  phase: GamePhase;
  actionMask: number[];
  onAction: (actionId: number) => void;
  onSurrender?: () => void;
  isGameOver: boolean;
  /** Map<sourceSlot, Set<effectIdx>> from useActionMask — legacy mask-derived
   *  fallback used only when the engine-decoded list is unavailable. */
  canActivateEffect?: Map<number, Set<number>>;
  /** Engine-decoded legal actions for the current decision player. The action
   *  bar names activatable effects by source card + effect from this. */
  decodedActions?: DecodedAction[];
}

export function ActionBar({ phase, actionMask, onAction, onSurrender, isGameOver, canActivateEffect, decodedActions }: ActionBarProps) {
  if (isGameOver) return null;

  // When the engine-decoded list is available (non-empty for any active
  // decision), it is the source of truth for activatable effects — every
  // category (field/Digiburst/Training/trash/hand [Main]) decoded correctly,
  // named by source card + effect. The raw mask-derived `canActivateEffect`
  // is only a fallback for surfaces that don't fetch the decoded list (e.g.
  // WebSocket PvP/online).
  const hasDecoded = (decodedActions?.length ?? 0) > 0;
  const decodedEffects = (decodedActions ?? []).filter(isActivatableEffect);
  const effectNameDupes = effectNameCounts(decodedEffects);

  const canPass = actionMask[ACTION.PASS] === 1;
  const canHatch = actionMask[ACTION.HATCH] === 1;
  const canMove = actionMask[ACTION.MOVE] === 1;
  const canKeepHand = actionMask[ACTION.MULLIGAN_KEEP] === 1;
  const canMulligan = actionMask[ACTION.MULLIGAN_REDRAW] === 1;

  const phaseName = PHASE_NAMES[phase] ?? 'Unknown';

  const handleSurrender = () => {
    if (onSurrender && window.confirm('Are you sure you want to surrender?')) {
      onSurrender();
    }
  };

  return (
    <div data-testid="action-bar" className="flex items-center gap-2 px-3 py-2 bg-gray-800 border-t border-gray-700">
      <span className="text-xs text-gray-500">{phaseName}:</span>

      {phase === GamePhase.Mulligan && canKeepHand && (
        <button
          data-testid="action-keep-hand"
          onClick={() => onAction(ACTION.MULLIGAN_KEEP)}
          className="px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded"
        >
          Keep Hand
        </button>
      )}

      {phase === GamePhase.Mulligan && canMulligan && (
        <button
          data-testid="action-mulligan"
          onClick={() => onAction(ACTION.MULLIGAN_REDRAW)}
          className="px-3 py-1 bg-orange-600 hover:bg-orange-500 text-white text-sm rounded"
        >
          Mulligan
        </button>
      )}

      {canHatch && (
        <button
          data-testid="action-hatch"
          onClick={() => onAction(ACTION.HATCH)}
          className="px-3 py-1 bg-yellow-600 hover:bg-yellow-500 text-white text-sm rounded"
        >
          Hatch
        </button>
      )}

      {canMove && (
        <button
          data-testid="action-move"
          onClick={() => onAction(ACTION.MOVE)}
          className="px-3 py-1 bg-green-600 hover:bg-green-500 text-white text-sm rounded"
        >
          Move
        </button>
      )}

      {canPass && (
        <button
          data-testid="action-pass"
          onClick={() => onAction(ACTION.PASS)}
          className="px-3 py-1 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
        >
          {phase === 2 ? 'Skip Breeding' : phase >= 5 ? 'Decline' : 'Pass'}
        </button>
      )}

      {/* Activatable [Main] effects — engine-decoded (card + effect name). */}
      {hasDecoded
        ? decodedEffects.length > 0 && (
            <>
              <span className="w-px h-5 bg-gray-600" />
              {decodedEffects.map((a) => (
                <button
                  key={`eff-${a.actionId}`}
                  data-testid={`action-effect-${a.actionId}`}
                  title={effectTooltip(a)}
                  onClick={() => onAction(a.actionId)}
                  className="px-3 py-1 bg-cyan-700 hover:bg-cyan-600 text-white text-sm rounded"
                >
                  {activatableEffectLabel(
                    a,
                    (effectNameDupes.get(a.cardName ?? '') ?? 0) > 1,
                  )}
                </button>
              ))}
            </>
          )
        : canActivateEffect && canActivateEffect.size > 0 && (
            <>
              <span className="w-px h-5 bg-gray-600" />
              {[...canActivateEffect.entries()].map(([source, effects]) =>
                [...effects].map((effectIdx) => (
                  <button
                    key={`eff-${source}-${effectIdx}`}
                    data-testid={`action-effect-${source}-${effectIdx}`}
                    onClick={() => onAction(ACTION.EFFECT_START + source * EFFECTS_PER_SOURCE + effectIdx)}
                    className="px-3 py-1 bg-cyan-700 hover:bg-cyan-600 text-white text-sm rounded"
                  >
                    Effect {source}:{effectIdx}
                  </button>
                ))
              )}
            </>
          )}

      {/* Spacer + Surrender */}
      {onSurrender && (
        <>
          <div className="flex-1" />
          <button
            data-testid="action-surrender"
            onClick={handleSurrender}
            className="px-3 py-1 bg-red-900/60 hover:bg-red-800 text-red-300 text-xs rounded border border-red-700/50"
          >
            Surrender
          </button>
        </>
      )}
    </div>
  );
}
