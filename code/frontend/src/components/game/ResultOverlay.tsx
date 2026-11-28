import '@/components/game/decisions/decisions.css';

interface ResultOverlayProps {
  isGameOver: boolean;
  winner: number | null;
  localPlayer: number;
  playerLabels: Record<number, string>;
  surrenderedBy?: number | null;
  gameSeed?: string | null;
  turnCount?: number;
  ownSecurity?: number;
  opponentSecurity?: number;
  onReturnToMenu: () => void;
}

/**
 * Modernized Hearthstone/Aegis-styled post-match result card using unified
 * design tokens across all 6 theme presets.
 */
export function ResultOverlay({
  isGameOver,
  winner,
  localPlayer,
  playerLabels,
  surrenderedBy,
  gameSeed,
  turnCount,
  ownSecurity,
  opponentSecurity,
  onReturnToMenu,
}: ResultOverlayProps) {
  if (!isGameOver) return null;

  const getLabel = (id: number) => playerLabels[id] ?? `Player ${id}`;
  const isWin = winner === localPlayer;
  const isDraw = winner === null;
  const isSurrender = surrenderedBy != null;

  let headingText: string;
  let subText: string;
  let crestSymbol: string;

  if (isDraw) {
    headingText = 'Draw';
    subText = 'The match ended in a draw.';
    crestSymbol = '=';
  } else if (isWin) {
    headingText = 'Victory';
    subText =
      isSurrender && surrenderedBy !== localPlayer
        ? `${getLabel(surrenderedBy)} surrendered.`
        : `You defeated ${getLabel(localPlayer === 1 ? 2 : 1)}.`;
    crestSymbol = '★';
  } else {
    headingText = isSurrender && surrenderedBy === localPlayer ? 'Surrendered' : 'Defeat';
    subText =
      isSurrender && surrenderedBy === localPlayer
        ? 'You conceded the match.'
        : `${getLabel(winner!)} won the match.`;
    crestSymbol = '✕';
  }

  return (
    <div data-testid="result-overlay" className="hs-result-backdrop">
      <div
        role="dialog"
        aria-label={headingText}
        className={`hs-result-card ${isWin ? 'hs-result-card--win' : isDraw ? '' : 'hs-result-card--loss'}`}
      >
        <div className="hs-result-card__crest" aria-hidden="true">
          {crestSymbol}
        </div>

        <h1 className="hs-result-card__title">{headingText}</h1>
        <p className="hs-result-card__sub">{subText}</p>

        {turnCount != null && ownSecurity != null && opponentSecurity != null && (
          <div className="hs-result-stats">
            <div className="hs-result-stat">
              <span className="hs-result-stat__k">Turns</span>
              <span className="hs-result-stat__v">{turnCount}</span>
            </div>
            <div className="hs-result-stat">
              <span className="hs-result-stat__k">Your Sec</span>
              <span className="hs-result-stat__v">{ownSecurity}</span>
            </div>
            <div className="hs-result-stat">
              <span className="hs-result-stat__k">Opp Sec</span>
              <span className="hs-result-stat__v">{opponentSecurity}</span>
            </div>
          </div>
        )}

        {gameSeed && (
          <div className="hs-result-seed">
            <span className="hs-result-stat__k">Shuffle Seed</span>
            <code className="select-all">{gameSeed}</code>
            <button
              type="button"
              aria-label="Copy seed"
              onClick={() => void navigator.clipboard?.writeText(gameSeed)}
              className="ds-btn ds-btn--ghost"
            >
              Copy Seed
            </button>
          </div>
        )}

        <button
          type="button"
          data-testid="result-return"
          onClick={onReturnToMenu}
          className="ds-btn ds-btn--primary w-full justify-center py-2.5 text-sm"
        >
          Return to Launcher
        </button>
      </div>
    </div>
  );
}
