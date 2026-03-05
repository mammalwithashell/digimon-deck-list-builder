import { useCallback, useEffect, useRef, useState } from 'react';
import type { GameState } from '@/types/game';

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

interface StateUpdatePayload {
  type: 'state_update';
  state: GameState;
  action_mask?: number[];
  action_descriptions?: Record<number, string>;
  current_player_id: number;
  is_game_over: boolean;
  winner_id?: number | null;
  logs?: string[];
  your_player_id?: number;
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
  const [myPlayerId, setMyPlayerId] = useState<number | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const retriesRef = useRef(0);
  const maxRetries = 5;
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const connect = useCallback(() => {
    if (!options) return;
    const { gameId, role = 'player' } = options;

    const token = localStorage.getItem('access_token');
    if (!token) {
      setStatus('error');
      options.onError?.('Not authenticated');
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
    const url = `${wsBase}/ws/games/${gameId}?token=${encodeURIComponent(token)}&role=${role}`;

    setStatus('connecting');
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setStatus('connected');
      retriesRef.current = 0;
    };

    ws.onmessage = (event) => {
      const msg: ServerMessage = JSON.parse(event.data);
      const opts = optionsRef.current;
      if (!opts) return;

      switch (msg.type) {
        case 'state_update':
          if (msg.your_player_id != null) {
            setMyPlayerId(msg.your_player_id);
          }
          opts.onStateUpdate?.(msg);
          break;
        case 'player_joined':
          opts.onPlayerJoined?.(msg);
          break;
        case 'player_disconnected':
          opts.onPlayerDisconnected?.(msg.player_id);
          break;
        case 'player_reconnected':
          opts.onPlayerReconnected?.(msg.player_id);
          break;
        case 'game_over':
          opts.onGameOver?.(msg);
          break;
        case 'spectator_count':
          opts.onSpectatorCount?.(msg.count);
          break;
        case 'error':
          opts.onError?.(msg.message);
          break;
        case 'pong':
          break;
      }
    };

    ws.onclose = (event) => {
      wsRef.current = null;
      if (event.code === 4001 || event.code === 4003 || event.code === 4004) {
        // Auth failure or game not found — don't retry
        setStatus('error');
        options.onError?.(event.reason || 'Connection rejected');
        return;
      }
      setStatus('disconnected');

      // Retry with backoff
      if (retriesRef.current < maxRetries) {
        const delay = Math.min(1000 * 2 ** retriesRef.current, 30000);
        retriesRef.current++;
        setTimeout(() => {
          if (optionsRef.current) connect();
        }, delay);
      } else {
        setStatus('error');
        options.onError?.('Connection lost after multiple retries');
      }
    };

    ws.onerror = () => {
      // onclose will fire after onerror
    };
  }, [options?.gameId, options?.role]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (options) {
      connect();
    }
    return () => {
      wsRef.current?.close(1000, 'Component unmounted');
      wsRef.current = null;
    };
  }, [options?.gameId, options?.role]); // eslint-disable-line react-hooks/exhaustive-deps

  const sendAction = useCallback((actionId: number) => {
    wsRef.current?.send(JSON.stringify({ type: 'action', action_id: actionId }));
  }, []);

  const disconnect = useCallback(() => {
    wsRef.current?.close(1000, 'User disconnected');
    wsRef.current = null;
    setStatus('disconnected');
  }, []);

  return { sendAction, disconnect, status, myPlayerId };
}
