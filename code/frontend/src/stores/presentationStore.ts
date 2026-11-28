import { create } from 'zustand';
import type { GameState } from '@/types/game';
import type {
  CardFlightCue,
  NarrationToast,
  OpponentShowcase,
  PresentationBeat,
} from '@/pacing/eventCatalog';

const MAX_TOASTS = 6;

interface PresentationStore {
  /** Snapshot currently painted on GameBoard (lags behind liveState only while beats drain). */
  presentedState: GameState | null;
  queue: PresentationBeat[];
  activeBeat: PresentationBeat | null;
  toasts: NarrationToast[];
  showcase: OpponentShowcase | null;
  flight: CardFlightCue | null;
  memoryDelta: number | null;
  isPresenting: boolean;

  enqueueBeats: (beats: PresentationBeat[], finalState: GameState) => void;
  advanceBeat: (finalState: GameState) => void;
  commitImmediate: (state: GameState, immediateToasts?: Omit<NarrationToast, 'createdAt'>[]) => void;
  pruneExpiredToasts: (now: number, lifetimeMs: number) => void;
  skipAll: (liveState: GameState | null) => void;
  reset: () => void;
}

function appendToast(
  current: NarrationToast[],
  toast: Omit<NarrationToast, 'createdAt'> | null | undefined,
): NarrationToast[] {
  if (!toast) return current;
  if (current.some((t) => t.id === toast.id)) return current;
  return [...current, { ...toast, createdAt: Date.now() }].slice(-MAX_TOASTS);
}

export const usePresentationStore = create<PresentationStore>((set, get) => ({
  presentedState: null,
  queue: [],
  activeBeat: null,
  toasts: [],
  showcase: null,
  flight: null,
  memoryDelta: null,
  isPresenting: false,

  enqueueBeats: (beats, finalState) => {
    if (beats.length === 0) {
      set({
        presentedState: finalState,
        activeBeat: null,
        queue: [],
        showcase: null,
        flight: null,
        memoryDelta: null,
        isPresenting: false,
      });
      return;
    }
    const { activeBeat, queue, toasts } = get();
    if (!activeBeat) {
      const [first, ...rest] = beats;
      set({
        presentedState: first!.snapshot,
        activeBeat: first!,
        queue: rest,
        showcase: first!.showcase ?? null,
        flight: first!.flight ?? null,
        memoryDelta: first!.memoryDelta ?? null,
        toasts: appendToast(toasts, first!.toast),
        isPresenting: true,
      });
    } else {
      set({
        queue: [...queue, ...beats],
        isPresenting: true,
      });
    }
  },

  advanceBeat: (finalState) => {
    const { queue, toasts } = get();
    if (queue.length === 0) {
      set({
        presentedState: finalState,
        activeBeat: null,
        showcase: null,
        flight: null,
        memoryDelta: null,
        isPresenting: false,
      });
      return;
    }
    const [next, ...rest] = queue;
    set({
      presentedState: next!.snapshot,
      activeBeat: next!,
      queue: rest,
      showcase: next!.showcase ?? null,
      flight: next!.flight ?? null,
      memoryDelta: next!.memoryDelta ?? null,
      toasts: appendToast(toasts, next!.toast),
      isPresenting: true,
    });
  },

  commitImmediate: (state, immediateToasts = []) => {
    let nextToasts = get().toasts;
    for (const t of immediateToasts) {
      nextToasts = appendToast(nextToasts, t);
    }
    set({
      presentedState: state,
      queue: [],
      activeBeat: null,
      showcase: null,
      flight: null,
      memoryDelta: null,
      toasts: nextToasts,
      isPresenting: false,
    });
  },

  pruneExpiredToasts: (now, lifetimeMs) => {
    const { toasts } = get();
    if (toasts.length === 0) return;
    const kept = toasts.filter((t) => now - t.createdAt < lifetimeMs);
    if (kept.length !== toasts.length) {
      set({ toasts: kept });
    }
  },

  skipAll: (liveState) => {
    const { queue, toasts } = get();
    let merged = toasts;
    for (const b of queue) {
      merged = appendToast(merged, b.toast);
    }
    set({
      presentedState: liveState,
      queue: [],
      activeBeat: null,
      showcase: null,
      flight: null,
      memoryDelta: null,
      toasts: merged,
      isPresenting: false,
    });
  },

  reset: () =>
    set({
      presentedState: null,
      queue: [],
      activeBeat: null,
      toasts: [],
      showcase: null,
      flight: null,
      memoryDelta: null,
      isPresenting: false,
    }),
}));
