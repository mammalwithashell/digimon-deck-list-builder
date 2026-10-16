import { useUiStore, type BoardStyle } from '@/stores/uiStore';

const STYLES: { value: BoardStyle; label: string; title: string }[] = [
  { value: 'classic', label: 'Classic', title: 'Console board that follows the app theme' },
  { value: 'arena', label: 'Arena', title: 'Tabletop arena with a coin memory track' },
];

/** Segmented control for the in-game board look. Persisted in uiStore; the
 *  board restyles immediately without restarting the game. */
export function BoardStyleControl() {
  const boardStyle = useUiStore((s) => s.boardStyle);
  const setBoardStyle = useUiStore((s) => s.setBoardStyle);

  return (
    <div
      data-testid="board-style-control"
      className="flex items-center gap-1 text-xs"
      role="group"
      aria-label="Board style"
    >
      <span className="text-[var(--ib-bone-dd)] mr-1">Board</span>
      {STYLES.map(({ value, label, title }) => (
        <button
          key={value}
          type="button"
          title={title}
          aria-pressed={boardStyle === value}
          data-testid={`board-style-${value}`}
          onClick={() => setBoardStyle(value)}
          className={`px-2 py-0.5 rounded border transition-colors ${
            boardStyle === value
              ? 'bg-cyan-700/60 border-cyan-500 text-cyan-100'
              : 'bg-[var(--ib-panel)] border-[var(--ib-line)] text-[var(--ib-bone-dd)] hover:text-[var(--ib-bone)]'
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
