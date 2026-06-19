import { create } from 'zustand';

/**
 * Transient "peek the board" toggle. While `peeking` is true, the blocking
 * decision overlays (SelectionPanel, TrashSelectModal, KeywordPromptDialog)
 * fade out and stop catching pointer events so the player can read the board
 * before committing to a choice. Not persisted — it's a momentary view state
 * and always resets when the decision resolves (PeekButton unmounts).
 */
interface PeekState {
  peeking: boolean;
  setPeeking: (value: boolean) => void;
  toggle: () => void;
}

export const usePeekStore = create<PeekState>((set) => ({
  peeking: false,
  setPeeking: (value) => set({ peeking: value }),
  toggle: () => set((s) => ({ peeking: !s.peeking })),
}));
