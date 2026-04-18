import { useCallback, useEffect, useRef, useState } from 'react';
import * as mmApi from '@/api/matchmaking';

export type MatchmakingStatus =
  | 'idle'
  | 'connecting'
  | 'waiting'
  | 'matched'
  | 'cancelled'
  | 'error';

export interface MatchFoundPayload {
  join_code: string;
  game_id: string;
  opponent: { user_id: string | null; display_name: string | null };
}

interface WaitingMessage {
  type: 'waiting';
  waited_seconds: number;
  rating_window: number | null;
}

interface MatchFoundMessage {
  type: 'match_found';
  join_code: string;
  game_id: string;
  opponent: { user_id: string | null; display_name: string | null };
}

interface CancelledMessage {
  type: 'cancelled';
  reason: string;
}

type ServerMessage = WaitingMessage | MatchFoundMessage | CancelledMessage;

export interface QueueParams {
  queue_type: mmApi.QueueType;
  deck_id: string;
  opponent_tier_filter?: mmApi.TierFilter;
}

export interface UseMatchmakingResult {
  status: MatchmakingStatus;
  ticketId: string | null;
  waitedSeconds: number;
  ratingWindow: number | null;
  match: MatchFoundPayload | null;
  error: string | null;
  /** Submit a ticket. If the backend returns an immediate match, the hook
   *  transitions straight to `matched` without opening a WebSocket. */
  enqueue: (params: QueueParams) => Promise<void>;
  /** Cancel the current ticket. Safe to call repeatedly. */
  cancel: () => Promise<void>;
}

function buildWsUrl(ticketId: string, token: string): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const apiBase = (import.meta.env.VITE_API_URL as string | undefined) ?? '';
  let wsBase: string;
  if (apiBase.startsWith('http')) {
    wsBase = apiBase.replace(/^http/, 'ws');
  } else {
    wsBase = `${protocol}//${window.location.host}${apiBase}`;
  }
  return `${wsBase}/ws/matchmaking/${ticketId}?token=${encodeURIComponent(token)}`;
}

export function useMatchmaking(): UseMatchmakingResult {
  const [status, setStatus] = useState<MatchmakingStatus>('idle');
  const [ticketId, setTicketId] = useState<string | null>(null);
  const [waitedSeconds, setWaitedSeconds] = useState(0);
  const [ratingWindow, setRatingWindow] = useState<number | null>(null);
  const [match, setMatch] = useState<MatchFoundPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const ticketIdRef = useRef<string | null>(null);

  const closeSocket = useCallback((code = 1000) => {
    const ws = wsRef.current;
    wsRef.current = null;
    if (ws && ws.readyState !== WebSocket.CLOSED) {
      try {
        ws.close(code, 'client_close');
      } catch {
        // ignore
      }
    }
  }, []);

  const openSocket = useCallback((id: string) => {
    const token = localStorage.getItem('access_token');
    if (!token) {
      setStatus('error');
      setError('Not authenticated');
      return;
    }
    const ws = new WebSocket(buildWsUrl(id, token));
    wsRef.current = ws;
    setStatus('connecting');

    ws.onopen = () => {
      // The server sends `waiting` immediately on connect — we'll promote
      // to 'waiting' there.
    };

    ws.onmessage = (event) => {
      let msg: ServerMessage;
      try {
        msg = JSON.parse(event.data);
      } catch {
        return;
      }
      if (msg.type === 'waiting') {
        setStatus('waiting');
        setWaitedSeconds(msg.waited_seconds);
        setRatingWindow(msg.rating_window);
      } else if (msg.type === 'match_found') {
        setStatus('matched');
        setMatch({
          join_code: msg.join_code,
          game_id: msg.game_id,
          opponent: msg.opponent,
        });
      } else if (msg.type === 'cancelled') {
        setStatus('cancelled');
        setError(msg.reason);
      }
    };

    ws.onclose = (event) => {
      wsRef.current = null;
      // 4001/4003/4004 are permanent auth/ownership errors — surface them.
      if (event.code === 4001 || event.code === 4003 || event.code === 4004) {
        setStatus('error');
        setError(event.reason || 'Connection rejected');
      }
      // Clean closes leave whatever terminal status we already set.
    };

    ws.onerror = () => {
      // onclose will fire right after; don't duplicate the error here.
    };
  }, []);

  const enqueue = useCallback(
    async (params: QueueParams) => {
      setError(null);
      setMatch(null);
      setWaitedSeconds(0);
      setRatingWindow(null);
      setStatus('connecting');
      try {
        const resp = await mmApi.queue(params);
        setTicketId(resp.ticket_id);
        ticketIdRef.current = resp.ticket_id;
        if (resp.status === 'matched') {
          // Backend paired us immediately — skip the WS round-trip.
          setStatus('matched');
          setMatch({
            join_code: resp.join_code,
            game_id: resp.game_id,
            opponent: { user_id: null, display_name: null },
          });
          return;
        }
        openSocket(resp.ticket_id);
      } catch (err: unknown) {
        setStatus('error');
        const msg =
          typeof err === 'object' && err !== null && 'message' in err
            ? String((err as { message: unknown }).message)
            : 'Failed to queue';
        setError(msg);
      }
    },
    [openSocket],
  );

  const cancel = useCallback(async () => {
    const id = ticketIdRef.current;
    closeSocket(1000);
    if (id) {
      try {
        await mmApi.cancelTicket(id);
      } catch {
        // Ticket may already be gone; ignore.
      }
    }
    setStatus('cancelled');
    setTicketId(null);
    ticketIdRef.current = null;
  }, [closeSocket]);

  useEffect(() => {
    return () => {
      closeSocket(1000);
    };
  }, [closeSocket]);

  return {
    status,
    ticketId,
    waitedSeconds,
    ratingWindow,
    match,
    error,
    enqueue,
    cancel,
  };
}
