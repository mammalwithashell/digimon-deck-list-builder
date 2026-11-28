import { useCallback, useEffect, useRef } from 'react';
import { useGameStore } from '@/stores/gameStore';
import { useUiStore } from '@/stores/uiStore';
import { usePresentationStore } from '@/stores/presentationStore';
import { buildPresentationBeats } from '@/pacing/eventCatalog';
import { BASE_PACING } from '@/pacing/pacingConfig';
import { normalizeGameEvents } from '@/utils/gameEvents';
import type { GameEvent, GameState } from '@/types/game';

/**
 * All-mode client-side presentation queue driver (local bot, WebSocket PvP,
 * vs-AI-online, and Watch replays).
 *
 * - Compiles each incoming `(prevState, nextState, events)` into observable beats
 * - Advances beats on their scaled durations, syncing `activeEffectSlot` on `gameStore`
 * - Pauses Narration Stack toast decay while a human decision is open
 * - Binds `Space` to skip remaining beats immediately without mutating live state
 */
export function usePresentationQueue() {
  const activeBeat = usePresentationStore((s) => s.activeBeat);
  const presentedState = usePresentationStore((s) => s.presentedState);
  const isPresenting = usePresentationStore((s) => s.isPresenting);
  const memoryDelta = usePresentationStore((s) => s.memoryDelta);
  const enqueueBeats = usePresentationStore((s) => s.enqueueBeats);
  const advanceBeat = usePresentationStore((s) => s.advanceBeat);
  const commitImmediate = usePresentationStore((s) => s.commitImmediate);
  const pruneExpiredToasts = usePresentationStore((s) => s.pruneExpiredToasts);
  const skipAll = usePresentationStore((s) => s.skipAll);

  const pendingSelection = useGameStore((s) => s.pendingSelection);
  const setActiveEffect = useGameStore((s) => s.setActiveEffect);
  const liveStateRef = useRef<GameState | null>(null);

  const presentTransition = useCallback(
    (nextState: GameState, rawEvents?: GameEvent[]) => {
      const prevState = liveStateRef.current;
      liveStateRef.current = nextState;

      const normalized = normalizeGameEvents(rawEvents);
      const { botSpeed, motion } = useUiStore.getState();

      if (!prevState || normalized.length === 0 || botSpeed === 'instant' || motion === 'off') {
        commitImmediate(nextState);
        return;
      }

      const beats = buildPresentationBeats({
        prevState,
        nextState,
        events: normalized,
        speed: botSpeed,
        motion,
      });
      enqueueBeats(beats, nextState);
    },
    [commitImmediate, enqueueBeats],
  );

  // Drive active beat timer + source highlight sync
  useEffect(() => {
    if (!activeBeat) {
      setActiveEffect(null, null);
      return;
    }
    if (activeBeat.activeSource) {
      setActiveEffect(activeBeat.activeSource.slot, activeBeat.activeSource.player);
    } else {
      setActiveEffect(null, null);
    }
    const timer = setTimeout(() => {
      const finalState = liveStateRef.current;
      if (finalState) advanceBeat(finalState);
    }, activeBeat.durationMs);
    return () => clearTimeout(timer);
  }, [activeBeat, advanceBeat, setActiveEffect]);

  // Periodically prune expired Narration Stack toasts (paused while deciding)
  useEffect(() => {
    if (pendingSelection?.selectingPlayer === 1) return;
    const interval = setInterval(() => {
      pruneExpiredToasts(Date.now(), BASE_PACING.toastLifetimeMs);
    }, 600);
    return () => clearInterval(interval);
  }, [pendingSelection, pruneExpiredToasts]);

  const handleSkip = useCallback(() => {
    skipAll(liveStateRef.current);
  }, [skipAll]);

  // Spacebar skips currently draining animations cleanly
  useEffect(() => {
    if (!isPresenting) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code !== 'Space') return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'BUTTON') return;
      e.preventDefault();
      handleSkip();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handleSkip, isPresenting]);

  return {
    presentedState,
    isPresenting,
    memoryDelta,
    presentTransition,
    skipPresentation: handleSkip,
  };
}
