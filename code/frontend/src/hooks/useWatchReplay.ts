import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as gameApi from '@/api/gameApi';
import { useGameStore } from '@/stores/gameStore';
import { cardChoiceToken } from '@/types/cardChoice';
import type { GameEvent, GameState } from '@/types/game';
import {
  createWatchGame,
  loadWatchGame,
  loadWatchQueue,
  watchGameUrl,
  watchPlaylist,
  type WatchGameDetail,
  type WatchGameRow,
  type WatchSearchView,
} from '@/features/watch/watchQueue';

/** Milliseconds between replayed moves. */
export const WATCH_SPEEDS = { slow: 2000, normal: 1000, fast: 400, faster: 120 } as const;
export type WatchSpeed = keyof typeof WATCH_SPEEDS;
const NEXT_GAME_DELAY_MS = 4000;

type ApplyResponse = (res: {
  state: GameState;
  action_mask: number[];
  logs?: string[];
  events?: GameEvent[];
  agent_pending?: boolean;
  seed?: string;
}) => void;

export interface WatchReplay {
  active: boolean;
  row: WatchGameRow | null;
  detail: WatchGameDetail | null;
  position: number;
  playlistLength: number;
  /** Number of recorded moves already applied to the board. */
  step: number;
  totalSteps: number;
  playing: boolean;
  /** True while jumping to a move (replaying the moves before it). */
  seeking: boolean;
  speed: WatchSpeed;
  autoAdvance: boolean;
  error: string | null;
  /** Search view for the move that produced the current board (null if it was forced). */
  lastSearch: WatchSearchView | null;
  /** Search view for the move about to be played. */
  nextSearch: WatchSearchView | null;
  togglePlay: () => void;
  stepOnce: () => void;
  stepBack: () => void;
  /** Jump to the board after `target` recorded moves. */
  seek: (target: number) => void;
  setSpeed: (speed: WatchSpeed) => void;
  setAutoAdvance: (on: boolean) => void;
  goTo: (position: number) => void;
  restart: () => void;
}

/**
 * Drives a recorded game on the board (watch mode). Inert unless the URL
 * carries `?watch=<queue>&pos=<n>`; GamePage blocks manual input while active.
 *
 * Going backwards is reset-and-replay: the engine has no undo, so a step back
 * creates a fresh game from the same seed and replays the earlier moves
 * (`&at=<move>` in the URL) before showing the board.
 */
export function useWatchReplay(opts: {
  queue: string | null;
  position: number;
  filter: string;
  /** Move to fast-forward to once the game is loaded (from `&at=`). */
  startAt: number;
  urlGameId: string | undefined;
  applyGameResponse: ApplyResponse;
}): WatchReplay {
  const { queue, position, filter, startAt, urlGameId, applyGameResponse } = opts;
  const active = Boolean(queue);
  const navigate = useNavigate();
  const storeGameId = useGameStore((s) => s.gameId);
  const isGameOver = useGameStore((s) => s.isGameOver);
  const setPlayerLabels = useGameStore((s) => s.setPlayerLabels);
  const resetStore = useGameStore((s) => s.reset);

  const [playlist, setPlaylist] = useState<WatchGameRow[]>([]);
  const [detail, setDetail] = useState<WatchGameDetail | null>(null);
  const [step, setStep] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [seeking, setSeeking] = useState(false);
  const [speed, setSpeed] = useState<WatchSpeed>('normal');
  const [autoAdvance, setAutoAdvance] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const busyRef = useRef(false);
  const startAtDoneRef = useRef(false);

  // Load the playlist and this game's recording.
  useEffect(() => {
    if (!queue) return;
    let cancelled = false;
    setDetail(null);
    setStep(0);
    setError(null);
    startAtDoneRef.current = false;
    (async () => {
      try {
        const index = await loadWatchQueue(queue);
        const rows = watchPlaylist(index, filter);
        const row = rows[position];
        if (!row) throw new Error(`No game at position ${position} in queue "${queue}"`);
        const game = await loadWatchGame(queue, row.id);
        if (cancelled) return;
        setPlaylist(rows);
        setDetail(game);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [queue, position, filter, urlGameId]);

  const boardReady = active && detail !== null && storeGameId !== null && storeGameId === urlGameId;

  // Seat labels from the recording (seat 0 = Player 1).
  useEffect(() => {
    if (!boardReady || !detail) return;
    setPlayerLabels({ 1: detail.seat_labels[0], 2: detail.seat_labels[1] });
  }, [boardReady, detail, setPlayerLabels]);

  /**
   * Submit recorded moves `from`..`to` (exclusive) and show the board after
   * the last one. Intermediate responses are not rendered, so a long jump
   * doesn't replay every animation.
   */
  const advance = useCallback(
    async (from: number, to: number): Promise<boolean> => {
      if (!detail || !urlGameId || busyRef.current) return false;
      const end = Math.min(to, detail.actions.length);
      if (end <= from) return false;
      busyRef.current = true;
      // A pending card choice must be answered with its current token, the
      // same as a human click (the server's checked step rejects otherwise).
      let token = cardChoiceToken(useGameStore.getState().pendingSelection?.cardChoice);
      let k = from;
      try {
        let last: Awaited<ReturnType<typeof gameApi.sendAction>> | null = null;
        for (; k < end; k++) {
          const action = detail.actions[k];
          if (action === undefined) break;
          last = await gameApi.sendAction(urlGameId, action, { paced: false, expectedCardChoice: token });
          token = cardChoiceToken(last.state.pendingSelection?.cardChoice);
        }
        if (last) applyGameResponse(last);
        setStep(k);
        return true;
      } catch (err) {
        setPlaying(false);
        setStep(k);
        setError(
          `Move ${k + 1} (action ${detail.actions[k]}) was rejected: ` +
            `${err instanceof Error ? err.message : String(err)}. ` +
            'If you reloaded mid-game, use Restart.',
        );
        return false;
      } finally {
        busyRef.current = false;
      }
    },
    [applyGameResponse, detail, urlGameId],
  );

  // Fast-forward to `&at=` once, after the fresh game is on the board.
  useEffect(() => {
    if (!boardReady || startAtDoneRef.current) return;
    startAtDoneRef.current = true;
    if (startAt <= 0) return;
    setSeeking(true);
    void advance(0, startAt).finally(() => setSeeking(false));
  }, [advance, boardReady, startAt]);

  // Timer: one recorded move per beat while playing.
  useEffect(() => {
    if (!boardReady || !playing || seeking || error || !detail) return;
    if (step >= detail.actions.length) return;
    const timer = setTimeout(() => void advance(step, step + 1), WATCH_SPEEDS[speed]);
    return () => clearTimeout(timer);
  }, [advance, boardReady, detail, error, playing, seeking, speed, step]);

  const goTo = useCallback(
    async (target: number, at = 0) => {
      if (!queue) return;
      const row = playlist[target];
      if (!row) return;
      try {
        const game = await loadWatchGame(queue, row.id);
        const newId = await createWatchGame(game);
        const oldId = urlGameId;
        resetStore();
        navigate(watchGameUrl(newId, queue, target, filter, at), { replace: at > 0 });
        if (oldId) void gameApi.deleteGame(oldId).catch(() => undefined);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [filter, navigate, playlist, queue, resetStore, urlGameId],
  );

  const seek = useCallback(
    (target: number) => {
      if (!detail || seeking || busyRef.current) return;
      const clamped = Math.max(0, Math.min(target, detail.actions.length));
      if (clamped === step) return;
      setPlaying(false);
      setError(null);
      if (clamped > step) {
        setSeeking(true);
        void advance(step, clamped).finally(() => setSeeking(false));
      } else {
        // No undo in the engine: rebuild the game and replay up to the target.
        setSeeking(true);
        void goTo(position, clamped).finally(() => setSeeking(false));
      }
    },
    [advance, detail, goTo, position, seeking, step],
  );

  // Auto-advance to the next game after the recording finishes.
  const finished = detail !== null && step >= detail.actions.length;
  useEffect(() => {
    if (!active || !finished || !autoAdvance || !playing) return;
    if (position + 1 >= playlist.length) return;
    const timer = setTimeout(() => void goTo(position + 1), NEXT_GAME_DELAY_MS);
    return () => clearTimeout(timer);
  }, [active, autoAdvance, finished, goTo, playing, playlist.length, position]);

  // A recording that ends before the engine says the game is over (or the
  // reverse) means this engine no longer reproduces the game.
  useEffect(() => {
    if (!boardReady || !detail || seeking) return;
    if (isGameOver && step < detail.actions.length) {
      setPlaying(false);
      setError(`The engine ended the game after ${step} of ${detail.actions.length} recorded moves.`);
    }
  }, [boardReady, detail, isGameOver, seeking, step]);

  const row = playlist[position] ?? null;
  return {
    active,
    row,
    detail,
    position,
    playlistLength: playlist.length,
    step,
    totalSteps: detail?.actions.length ?? 0,
    playing,
    seeking,
    speed,
    autoAdvance,
    error,
    lastSearch: detail && step > 0 ? (detail.searches[String(step - 1)] ?? null) : null,
    nextSearch: detail ? (detail.searches[String(step)] ?? null) : null,
    togglePlay: () => setPlaying((p) => !p),
    stepOnce: () => {
      setPlaying(false);
      void advance(step, step + 1);
    },
    stepBack: () => seek(step - 1),
    seek,
    setSpeed,
    setAutoAdvance,
    goTo: (target: number) => void goTo(target),
    restart: () => {
      setError(null);
      setPlaying(true);
      void goTo(position);
    },
  };
}
