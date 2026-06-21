import { useEffect } from 'react';
import { usePeekStore } from '@/stores/peekStore';

/**
 * Floating "peek the board" toggle. Rendered while the local player has a
 * blocking decision overlay up (SelectionPanel / TrashSelectModal /
 * KeywordPromptDialog). Toggling hides those overlays so the player can read
 * the board, then restores them to make the choice. Backtick (`) toggles too.
 *
 * Sits at z-[55] — above the decision overlays (z-40) so it stays clickable
 * while they're hidden, below the enlarged card detail (z-[60]). Resets peek on
 * unmount so a stale peek never lingers past the decision.
 */
export function PeekButton() {
  const peeking = usePeekStore((s) => s.peeking);
  const toggle = usePeekStore((s) => s.toggle);
  const setPeeking = usePeekStore((s) => s.setPeeking);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== '`' || e.repeat) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
      e.preventDefault();
      usePeekStore.getState().toggle();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // Clear peek when the decision resolves (this button unmounts).
  useEffect(() => () => setPeeking(false), [setPeeking]);

  return (
    <button
      type="button"
      data-testid="peek-board"
      onClick={toggle}
      title="Temporarily hide the prompt to view the board (`)"
      className={`fixed top-12 left-1/2 z-[55] flex -translate-x-1/2 items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-medium shadow-lg backdrop-blur-sm transition-colors ${
        peeking
          ? 'bg-[var(--ib-player)] border-[var(--ib-player)] text-[#1a0d02]'
          : 'bg-[var(--ib-graphite)] border-[var(--ib-line)] text-[var(--ib-bone)] hover:border-[var(--ib-player)]'
      }`}
    >
      <span aria-hidden="true">{peeking ? '↩' : '👁'}</span>
      {peeking ? 'Resume prompt' : 'Peek board'}
      <kbd className="ml-0.5 rounded bg-black/15 px-1 text-[10px] opacity-80">{'`'}</kbd>
    </button>
  );
}
