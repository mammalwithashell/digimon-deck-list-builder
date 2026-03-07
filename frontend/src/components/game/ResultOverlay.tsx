interface ResultOverlayProps {
  isGameOver: boolean;
  winner: number | null;
  localPlayer: number;
  playerLabels: Record<number, string>;
  surrenderedBy?: number | null;
  onReturnToMenu: () => void;
}

export function ResultOverlay({
  isGameOver,
  winner,
  localPlayer,
  playerLabels,
  surrenderedBy,
  onReturnToMenu,
}: ResultOverlayProps) {
  if (!isGameOver) return null;

  const getLabel = (id: number) => playerLabels[id] ?? `Player ${id}`;

  const isWin = winner === localPlayer;
  const isDraw = winner === null;
  const isSurrender = surrenderedBy != null;

  let headingText: string;
  let headingColor: string;
  let bgGradient: string;
  let subText: string;

  if (isDraw) {
    headingText = 'Draw';
    headingColor = 'text-gray-200';
    bgGradient = 'from-gray-900/95 to-gray-800/95';
    subText = 'The game ended in a draw.';
  } else if (isWin) {
    headingText = 'Victory!';
    headingColor = 'text-yellow-300';
    bgGradient = 'from-yellow-900/95 to-amber-900/95';
    subText = isSurrender && surrenderedBy !== localPlayer
      ? `${getLabel(surrenderedBy)} surrendered.`
      : 'Congratulations!';
  } else {
    headingText = isSurrender && surrenderedBy === localPlayer ? 'Surrendered' : 'Defeat';
    headingColor = 'text-red-300';
    bgGradient = 'from-red-900/95 to-gray-900/95';
    subText = isSurrender && surrenderedBy === localPlayer
      ? 'You surrendered the game.'
      : `${getLabel(winner!)} wins.`;
  }

  return (
    <div
      data-testid="result-overlay"
      className="fixed inset-0 z-50 flex items-center justify-center"
    >
      <div className="absolute inset-0 bg-black/70" />
      <div
        className={`relative flex flex-col items-center gap-6 px-16 py-12 rounded-2xl bg-gradient-to-b ${bgGradient} shadow-2xl border border-white/10`}
      >
        {/* Decorative top line */}
        <div
          className={`w-24 h-1 rounded-full ${
            isWin ? 'bg-yellow-400' : isDraw ? 'bg-gray-400' : 'bg-red-500'
          }`}
        />

        <h1 className={`text-5xl font-extrabold tracking-wide ${headingColor}`}>
          {headingText}
        </h1>

        <p className="text-lg text-gray-300">{subText}</p>

        <button
          data-testid="result-return"
          onClick={onReturnToMenu}
          className="mt-4 px-8 py-3 bg-blue-600 hover:bg-blue-500 text-white font-semibold text-lg rounded-lg transition-colors"
        >
          Return to Menu
        </button>
      </div>
    </div>
  );
}
