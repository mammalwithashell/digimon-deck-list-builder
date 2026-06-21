import { useCallback, useEffect, useRef, useState } from 'react';
import type { GameState, GameEvent } from '@/types/game';

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

/** Default deadline (ms) for the first `state_update` to arrive before the
 *  connection is treated as failed. Guards the "socket opens (or upgrade
 *  404s) but no game state ever streams" hang. */
export const DEFAULT_CONNECT_TIMEOUT_MS = 15000;

interface StateUpdatePayload {
  type: 'state_update';
  state: GameState;
  action_mask?: number[];
  action_descriptions?: Record<number, string>;
  current_player_id: number;
  is_game_over: boolean;
  winner_id?: number | null;
  logs?: string[];
  events?: GameEvent[];
  your_player_id?: number;
  seed?: string | null;
}

interface PlayerJoinedPayload {
  type: 'player_joined';
  player_id: number;
  display_name: string;
}

interface GameOverPayload {
  type: 'game_over';
  winner_id: number | null;
}

interface SpectatorCountPayload {
  type: 'spectator_count';
  count: number;
}

type ServerMessage =
  | StateUpdatePayload
  | PlayerJoinedPayload
  | GameOverPayload
  | SpectatorCountPayload
  | { type: 'player_disconnected'; player_id: number }
  | { type: 'player_reconnected'; player_id: number }
  | { type: 'error'; message: string }
  | { type: 'pong' };

export interface UseWebSocketGameOptions {
  gameId: string;
  role?: 'player' | 'spectator';
  /** Override the first-state_update deadline (ms). Defaults to
   *  {@link DEFAULT_CONNECT_TIMEOUT_MS}. */
  connectTimeoutMs?: number;
  onStateUpdate?: (payload: StateUpdatePayload) => void;
  onPlayerJoined?: (payload: PlayerJoinedPayload) => void;
  onPlayerDisconnected?: (playerId: number) => void;
  onPlayerReconnected?: (playerId: number) => void;
  onGameOver?: (payload: GameOverPayload) => void;
  onSpectatorCount?: (count: number) => void;
  onError?: (message: string) => void;
}

export function useWebSocketGame(options: UseWebSocketGameOptions | null) {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');
  // Last failure detail, surfaced to the UI so the page can show *why* the
  // connection failed instead of hanging on an indefinite loading state.
  const [error, setError] = useState<string | null>(null);
  // Number of reconnect attempts made since the last successful open — lets
  // the UI distinguish an initial "connecting" from an in-progress "reconnecting".
  const [retryCount, setRetryCount] = useState(0);
  const [myPlayerId, setMyPlayerId] = useState<number | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const retriesRef = useRef(0);
  const maxRetries = 5;
  // True once the first `state_update` has arrived — the real "game is live"
  // signal (a bare socket open isn't enough; the server can 404 the upgrade
  // or accept it and never stream state).
  const gotStateRef = useRef(false);
  // Hard stop: suppresses auto-reconnect after a terminal failure, a timeout,
  // a user disconnect, or unmount.
  const stoppedRef = useRef(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const clearConnectTimeout = useCallback(() => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  // Transition to a terminal error state with a user-facing message.
  const fail = useCallback(
    (message: string) => {
      clearConnectTimeout();
      stoppedRef.current = true;
      setStatus('error');
      setError(message);
      optionsRef.current?.onError?.(message);
    },
    [clearConnectTimeout],
  );

  const connect = useCallback(() => {
    const opts = optionsRef.current;
    if (!opts) return;
    const { gameId, role = 'player' } = opts;

    const token = localStorage.getItem('access_token');
    if (!token) {
      fail('You are not signed in. Please log in and try again.');
      return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const apiBase = import.meta.env.VITE_API_URL ?? '';
    // If API_BASE is a relative path like '/api', build WS URL from window.location
    let wsBase: string;
    if (apiBase.startsWith('http')) {
      wsBase = apiBase.replace(/^http/, 'ws');
    } else {
      wsBase = `${protocol}//${window.location.host}${apiBase}`;
    }
    const engineVersion = (import.meta.env.VITE_ENGINE_VERSION as string | undefined) ?? '0.1.0';
    const url = `${wsBase}/ws/games/${gameId}?token=${encodeURIComponent(token)}&role=${role}&engine_version=${encodeURIComponent(engineVersion)}`;

    setStatus('connecting');
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setStatus('connected');
      retriesRef.current = 0;
      setRetryCount(0);
    };

    ws.onmessage = (event) => {
      const msg: ServerMessage = JSON.parse(event.data);
      const currentOpts = optionsRef.current;
      if (!currentOpts) return;

      switch (msg.type) {
        case 'state_update':
          // First real state — the game is live. Cancel the connect deadline.
          if (!gotStateRef.current) {
            gotStateRef.current = true;
            clearConnectTimeout();
          }
          if (msg.your_player_id != null) {
            setMyPlayerId(msg.your_player_id);
          }
          currentOpts.onStateUpdate?.(msg);
          break;
        case 'player_joined':
          currentOpts.onPlayerJoined?.(msg);
          break;
        case 'player_disconnected':
          currentOpts.onPlayerDisconnected?.(msg.player_id);
          break;
        case 'player_reconnected':
          currentOpts.onPlayerReconnected?.(msg.player_id);
          break;
        case 'game_over':
          currentOpts.onGameOver?.(msg);
          break;
        case 'spectator_count':
          currentOpts.onSpectatorCount?.(msg.count);
          break;
        case 'error':
          currentOpts.onError?.(msg.message);
          break;
        case 'pong':
          break;
      }
    };

    ws.onclose = (event) => {
      // Ignore a close from a socket we've already replaced (reconnect race)
      // or are no longer tracking.
      if (wsRef.current !== ws) return;
      wsRef.current = null;
      if (stoppedRef.current) return;

      if (event.code === 4001 || event.code === 4003 || event.code === 4004) {
        // Auth failure or game not found — don't retry.
        fail(event.reason || 'The server rejected the connection (the game may no longer exist).');
        return;
      }
      setStatus('disconnected');

      // Retry with backoff
      if (retriesRef.current < maxRetries) {
        const delay = Math.min(1000 * 2 ** retriesRef.current, 30000);
        retriesRef.current++;
        setRetryCount(retriesRef.current);
        setTimeout(() => {
          if (optionsRef.current && !stoppedRef.current) connect();
        }, delay);
      } else {
        fail('Lost connection to the game server after several attempts.');
      }
    };

    ws.onerror = () => {
      // onclose will fire after onerror
    };
  }, [options?.gameId, options?.role]); // eslint-disable-line react-hooks/exhaustive-deps

  // Arm the first-state_update deadline. Fires once per connection lifecycle
  // (set at mount and on manual reconnect); a successful state_update or a
  // terminal failure clears it.
  const armConnectTimeout = useCallback(() => {
    clearConnectTimeout();
    const ms = optionsRef.current?.connectTimeoutMs ?? DEFAULT_CONNECT_TIMEOUT_MS;
    timeoutRef.current = setTimeout(() => {
      timeoutRef.current = null;
      if (gotStateRef.current) return;
      wsRef.current?.close(1000, 'Connection timeout');
      wsRef.current = null;
      fail("The game server isn't responding. It may be temporarily unavailable — try again in a moment.");
    }, ms);
  }, [clearConnectTimeout, fail]);

  useEffect(() => {
    if (options) {
      stoppedRef.current = false;
      gotStateRef.current = false;
      connect();
      armConnectTimeout();
    }
    return () => {
      stoppedRef.current = true;
      clearConnectTimeout();
      wsRef.current?.close(1000, 'Component unmounted');
      wsRef.current = null;
    };
  }, [options?.gameId, options?.role]); // eslint-disable-line react-hooks/exhaustive-deps

  const sendAction = useCallback((actionId: number) => {
    wsRef.current?.send(JSON.stringify({ type: 'action', action_id: actionId }));
  }, []);

  const sendSurrender = useCallback(() => {
    wsRef.current?.send(JSON.stringify({ type: 'surrender' }));
  }, []);

  const disconnect = useCallback(() => {
    stoppedRef.current = true;
    clearConnectTimeout();
    wsRef.current?.close(1000, 'User disconnected');
    wsRef.current = null;
    setStatus('disconnected');
  }, [clearConnectTimeout]);

  // Manually re-attempt the connection (e.g. the failure UI's "Try again").
  const reconnect = useCallback(() => {
    clearConnectTimeout();
    stoppedRef.current = false;
    gotStateRef.current = false;
    retriesRef.current = 0;
    setRetryCount(0);
    setError(null);
    const old = wsRef.current;
    wsRef.current = null;
    old?.close(1000, 'Reconnecting');
    connect();
    armConnectTimeout();
  }, [armConnectTimeout, clearConnectTimeout, connect]);

  return { sendAction, sendSurrender, disconnect, reconnect, status, error, retryCount, myPlayerId };
}
