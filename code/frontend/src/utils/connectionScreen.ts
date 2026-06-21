import type { ConnectionStatus } from '@/hooks/useWebSocketGame';

/**
 * What the GamePage should render for a WebSocket-backed game (PvP /
 * vs-AI-online / spectator) given the live connection state.
 *
 * - `ready`        — play the game normally (board is hydrated, or it's a
 *                    local game that never streams over a socket).
 * - `connecting`   — first connection attempt, no game state yet.
 * - `reconnecting` — the socket dropped and the hook is retrying.
 * - `error`        — the connection failed terminally (rejected, timed out,
 *                    or retries exhausted); show a failure UI with a way out.
 */
export type ConnectionScreenState = 'ready' | 'connecting' | 'reconnecting' | 'error';

export interface ConnectionScreenInput {
  /** Whether this game streams state over the WebSocket hook at all. */
  useWebSocket: boolean;
  /** Live socket status from `useWebSocketGame`. */
  status: ConnectionStatus;
  /** Retry attempt count from `useWebSocketGame` (0 before any retry). */
  retryCount: number;
  /** True once the first `state_update` has populated both players. */
  boardHydrated: boolean;
}

/**
 * Decide whether GamePage should gate the board behind a connection screen,
 * and which variant to show. Pure so the decision is unit-testable without
 * rendering the whole page.
 */
export function connectionScreenState({
  useWebSocket,
  status,
  retryCount,
  boardHydrated,
}: ConnectionScreenInput): ConnectionScreenState {
  // Local games drive state over HTTP — never gate them on the socket.
  if (!useWebSocket) return 'ready';

  // A terminal failure always wins, even if the board never hydrated.
  if (status === 'error') return 'error';

  // Once the first state_update lands the game is live; a later transient
  // blip shouldn't yank the player off a populated board.
  if (boardHydrated) return 'ready';

  // Not hydrated yet and not errored: distinguish the very first attempt
  // from an in-progress reconnect so the UI can say which is happening.
  if (status === 'disconnected' || retryCount > 0) return 'reconnecting';
  return 'connecting';
}
