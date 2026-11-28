import { useState } from 'react';
import { ThemeSwitch, StylePresetSwitch } from '@/design/components/ThemeSwitch';
import { SleevePickerModal } from '@/components/settings/SleevePicker';
import { useUiStore, type BattleLanes, type BotSpeed, type Motion } from '@/stores/uiStore';
import type { PlayerState } from '@/types/game';

export interface ArenaHudProps {
  turnCount: number;
  player1: PlayerState;
  player2: PlayerState;
  playerLabels: Record<number, string>;
  isGameOver: boolean;
  gameSeed?: string | null;
  onUndo?: () => void;
  onSurrender?: () => void;
  onSkipPresentation?: () => void;
  isPresenting?: boolean;
  showSpeedControl?: boolean;
}

const SPEED_OPTIONS: { value: BotSpeed; label: string }[] = [
  { value: 'normal', label: 'Normal' },
  { value: 'fast', label: 'Fast' },
  { value: 'instant', label: 'Instant' },
];

const MOTION_OPTIONS: { value: Motion; label: string }[] = [
  { value: 'full', label: 'Full' },
  { value: 'reduced', label: 'Reduced' },
  { value: 'off', label: 'Off' },
];

const LANE_OPTIONS: { value: BattleLanes; label: string; testId: string }[] = [
  { value: 'two', label: '2 Rows (Front/Back)', testId: 'arena-lanes-two' },
  { value: 'one', label: '1 Row', testId: 'arena-lanes-one' },
];

/**
 * Compact Aegis/Hearthstone corner HUD replacing the 4 stacked top/bottom bars:
 * - Top-left: Opponent identity + Egg / Hand / Deck / Trash counters
 * - Top-right: Turn pill, Settings gear popover (Theme, Style, Speed, Motion, Seed, Undo), Surrender
 * - Bottom-left: Your identity + quick Mulligan/Hatch/Undo hooks
 * - Bottom-right: Skip-animation pill (while presenting) + Your Egg / Hand / Deck / Trash counters
 */
export function ArenaHud({
  turnCount,
  player1,
  player2,
  playerLabels,
  isGameOver,
  gameSeed,
  onUndo,
  onSurrender,
  onSkipPresentation,
  isPresenting = false,
  showSpeedControl = true,
}: ArenaHudProps) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [sleevesOpen, setSleevesOpen] = useState(false);
  const [seedCopied, setSeedCopied] = useState(false);
  const botSpeed = useUiStore((s) => s.botSpeed);
  const setBotSpeed = useUiStore((s) => s.setBotSpeed);
  const motion = useUiStore((s) => s.motion);
  const setMotion = useUiStore((s) => s.setMotion);
  const battleLanes = useUiStore((s) => s.battleLanes);
  const setBattleLanes = useUiStore((s) => s.setBattleLanes);

  const handleCopySeed = () => {
    if (!gameSeed) return;
    void navigator.clipboard?.writeText(gameSeed);
    setSeedCopied(true);
    setTimeout(() => setSeedCopied(false), 1500);
  };

  const handleSurrender = () => {
    if (onSurrender && window.confirm('Are you sure you want to surrender?')) {
      onSurrender();
    }
  };

  return (
    <>
      {/* Top-left: Opponent identity & compact zone counters */}
      <div className="hs-hud hs-hud--top-left">
        <div className="hs-player-pill hs-player-pill--opp">
          <span className="hs-player-pill__avatar">
            {(playerLabels[2] ?? 'O').slice(0, 1).toUpperCase()}
          </span>
          <span className="hs-player-pill__name">{playerLabels[2] ?? 'Opponent'}</span>
        </div>
        <div className="hs-counter-group" aria-label="Opponent zone counts">
          <span className="hs-counter-chip" title="Opponent Digi-Eggs">
            <span className="hs-counter-chip__k">EGG</span>
            <strong>{player2.eggDeckCount}</strong>
          </span>
          <span className="hs-counter-chip" title="Opponent Hand">
            <span className="hs-counter-chip__k">HND</span>
            <strong>{player2.handCount}</strong>
          </span>
          <span className="hs-counter-chip" title="Opponent Deck">
            <span className="hs-counter-chip__k">DCK</span>
            <strong>{player2.deckCount}</strong>
          </span>
          <span className="hs-counter-chip" title="Opponent Trash">
            <span className="hs-counter-chip__k">TRS</span>
            <strong>{player2.trashIds.length}</strong>
          </span>
        </div>
      </div>

      {/* Top-right: Turn chip + Settings popover + Undo + Surrender */}
      <div className="hs-hud hs-hud--top-right">
        <span className="hs-turn-badge">Turn {turnCount}</span>
        {!isGameOver && onUndo && (
          <button
            type="button"
            data-testid="undo-step-button"
            onClick={onUndo}
            className="hs-icon-btn"
            title="Undo last action (debug)"
          >
            ↩ Undo
          </button>
        )}
        <div className="relative">
          <button
            type="button"
            data-testid="arena-settings-toggle"
            className={`hs-icon-btn${settingsOpen ? ' is-open' : ''}`}
            onClick={() => setSettingsOpen((v) => !v)}
            aria-expanded={settingsOpen}
            title="Arena look, theme & speed"
          >
            ⚙ Look & Speed
          </button>
          {settingsOpen && (
            <div className="hs-settings-popover" data-testid="arena-settings-popover">
              <div className="hs-settings-popover__row">
                <span className="hs-settings-popover__label">Mode</span>
                <ThemeSwitch />
              </div>
              <div className="hs-settings-popover__col">
                <span className="hs-settings-popover__label">Style Preset</span>
                <StylePresetSwitch />
              </div>
              <div className="hs-settings-popover__col">
                <span className="hs-settings-popover__label">Battle Lanes</span>
                <div className="hs-seg-group" role="group" aria-label="Battle Lanes">
                  {LANE_OPTIONS.map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      data-testid={opt.testId}
                      aria-pressed={battleLanes === opt.value}
                      onClick={() => setBattleLanes(opt.value)}
                      className={`hs-seg-btn${battleLanes === opt.value ? ' is-active' : ''}`}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              <div className="hs-settings-popover__row">
                <span className="hs-settings-popover__label">Card Sleeves</span>
                <button
                  type="button"
                  data-testid="arena-sleeves-button"
                  onClick={() => {
                    setSettingsOpen(false);
                    setSleevesOpen(true);
                  }}
                  className="hs-seg-btn"
                >
                  Choose Sleeves…
                </button>
              </div>
              {showSpeedControl && (
                <div className="hs-settings-popover__col">
                  <span className="hs-settings-popover__label">Pacing Speed</span>
                  <div className="hs-seg-group" role="group" aria-label="Animation speed">
                    {SPEED_OPTIONS.map((opt) => {
                      const active =
                        botSpeed === opt.value || (opt.value === 'normal' && botSpeed === 'slow');
                      return (
                        <button
                          key={opt.value}
                          type="button"
                          data-testid={`arena-speed-${opt.value}`}
                          aria-pressed={active}
                          onClick={() => setBotSpeed(opt.value)}
                          className={`hs-seg-btn${active ? ' is-active' : ''}`}
                        >
                          {opt.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
              <div className="hs-settings-popover__col">
                <span className="hs-settings-popover__label">Motion</span>
                <div className="hs-seg-group" role="group" aria-label="Motion">
                  {MOTION_OPTIONS.map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      aria-pressed={motion === opt.value}
                      onClick={() => setMotion(opt.value)}
                      className={`hs-seg-btn${motion === opt.value ? ' is-active' : ''}`}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {gameSeed && (
                <div className="hs-settings-popover__row">
                  <span className="hs-settings-popover__label">Seed</span>
                  <button
                    type="button"
                    aria-label="Copy seed"
                    onClick={handleCopySeed}
                    className="hs-seg-btn"
                  >
                    {seedCopied ? 'Copied!' : `${gameSeed.slice(0, 10)}… Copy`}
                  </button>
                </div>
              )}
              {!isGameOver && onSurrender && (
                <div className="pt-2 mt-1 border-t border-[var(--line-0)]">
                  <button
                    type="button"
                    data-testid="action-surrender"
                    onClick={() => {
                      setSettingsOpen(false);
                      handleSurrender();
                    }}
                    className="hs-icon-btn hs-icon-btn--danger w-full justify-center"
                    title="Surrender match"
                  >
                    🏳 Surrender Match
                  </button>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Bottom-left: Your identity */}
      <div className="hs-hud hs-hud--bottom-left">
        <div className="hs-player-pill hs-player-pill--you">
          <span className="hs-player-pill__avatar">
            {(playerLabels[1] ?? 'Y').slice(0, 1).toUpperCase()}
          </span>
          <span className="hs-player-pill__name">{playerLabels[1] ?? 'You'}</span>
        </div>
      </div>

      {/* Bottom-right: Skip animation pill + Your compact zone counters */}
      <div className="hs-hud hs-hud--bottom-right">
        {isPresenting && onSkipPresentation && (
          <button
            type="button"
            data-testid="skip-presentation-btn"
            onClick={onSkipPresentation}
            className="hs-icon-btn hs-icon-btn--accent"
            title="Skip to latest state (Space)"
          >
            » Skip
          </button>
        )}
        <div className="hs-counter-group" aria-label="Your zone counts">
          <span className="hs-counter-chip" title="Your Digi-Eggs">
            <span className="hs-counter-chip__k">EGG</span>
            <strong>{player1.eggDeckCount}</strong>
          </span>
          <span
            className="hs-counter-chip"
            aria-label={`${player1.handCount} cards in your hand`}
            title="Your Hand"
          >
            <span className="hs-counter-chip__k">HND</span>
            <strong>{player1.handCount}</strong>
          </span>
          <span className="hs-counter-chip" title="Your Deck">
            <span className="hs-counter-chip__k">DCK</span>
            <strong>{player1.deckCount}</strong>
          </span>
          <span className="hs-counter-chip" title="Your Trash">
            <span className="hs-counter-chip__k">TRS</span>
            <strong>{player1.trashIds.length}</strong>
          </span>
        </div>
      </div>

      <SleevePickerModal
        isOpen={sleevesOpen}
        onClose={() => setSleevesOpen(false)}
      />
    </>
  );
}
