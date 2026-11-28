import type { DecodedAction } from '@/types/game';
import { activatableEffectLabel, effectTooltip } from '@/utils/effectLabel';

export interface CardActionChoice {
  handIndex: number;
  cardId?: string;
  canPlay: boolean;
  canDigivolve: boolean;
  canDnaDigivolve: boolean;
}

export interface CardActionPopoverProps {
  /** Hand multi-route choice (Play / Digivolve / DNA Digivolve). */
  actionChoice: CardActionChoice | null;
  /** Activatable [Main] effects available right now (docked bottom-left beside player badge). */
  activatableEffects?: DecodedAction[];
  /** True when user is selecting a digivolve target on the field. */
  isChoosingDigivolveTarget?: boolean;
  onPlay: () => void;
  onDigivolve: () => void;
  onDna: () => void;
  onCancelChoice: () => void;
  onCancelDigivolve: () => void;
  onActivateEffect: (actionId: number) => void;
}

/**
 * Splits contextual hand actions (centered above hand) from activatable [Main]
 * effects (docked cleanly on the bottom-left beside the player name badge and
 * to the left of the fanned hand).
 */
export function CardActionPopover({
  actionChoice,
  activatableEffects = [],
  isChoosingDigivolveTarget = false,
  onPlay,
  onDigivolve,
  onDna,
  onCancelChoice,
  onCancelDigivolve,
  onActivateEffect,
}: CardActionPopoverProps) {
  const showCenter = Boolean(actionChoice) || isChoosingDigivolveTarget;
  const showMainEffects = !showCenter && activatableEffects.length > 0;
  if (!showCenter && !showMainEffects) return null;

  return (
    <>
      {showCenter && (
        <div className="hs-floating-actions" data-testid="floating-card-actions">
          {actionChoice && (
            <div className="hs-action-pill-bar" role="dialog" aria-label="Choose card action">
              <span className="hs-action-pill-bar__label">Choose action</span>
              {actionChoice.canPlay && (
                <button type="button" onClick={onPlay} className="hs-pill-btn hs-pill-btn--primary">
                  Play
                </button>
              )}
              {actionChoice.canDigivolve && (
                <button
                  type="button"
                  onClick={onDigivolve}
                  className="hs-pill-btn hs-pill-btn--accent"
                >
                  Digivolve
                </button>
              )}
              {actionChoice.canDnaDigivolve && (
                <button
                  type="button"
                  onClick={onDna}
                  className="hs-pill-btn hs-pill-btn--special"
                >
                  DNA Digivolve
                </button>
              )}
              <button
                type="button"
                onClick={onCancelChoice}
                className="hs-pill-btn hs-pill-btn--ghost"
              >
                Cancel
              </button>
            </div>
          )}

          {isChoosingDigivolveTarget && (
            <div className="hs-action-pill-bar hs-action-pill-bar--notice">
              <span className="hs-action-pill-bar__label">
                Select a Digimon on your field or breeding area to digivolve
              </span>
              <button
                type="button"
                onClick={onCancelDigivolve}
                className="hs-pill-btn hs-pill-btn--ghost"
              >
                Cancel
              </button>
            </div>
          )}
        </div>
      )}

      {showMainEffects && (
        <aside
          className="hs-main-effects-dock"
          data-testid="main-effects-dock"
          aria-label="Activatable Main Effects"
        >
          <div className="hs-main-effects-dock__head">[Main] Effects</div>
          <div className="hs-main-effects-dock__list">
            {activatableEffects.map((act) => {
              const idTag = act.cardId ? ` (${act.cardId})` : '';
              return (
                <button
                  key={`eff-${act.actionId}`}
                  type="button"
                  data-testid={`action-effect-${act.actionId}`}
                  title={`${act.cardName ?? ''}${idTag} — ${effectTooltip(act)}`}
                  onClick={() => onActivateEffect(act.actionId)}
                  className="hs-main-effects-dock__btn"
                >
                  {activatableEffectLabel(act, false)}
                </button>
              );
            })}
          </div>
        </aside>
      )}
    </>
  );
}
