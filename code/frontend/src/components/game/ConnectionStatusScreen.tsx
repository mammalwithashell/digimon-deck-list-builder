import type { ConnectionScreenState } from '@/utils/connectionScreen';

interface ConnectionStatusScreenProps {
  /** Which connection phase to render (never `ready` — that renders the board). */
  state: Exclude<ConnectionScreenState, 'ready'>;
  /** Human-readable failure detail, shown only in the `error` state. */
  error?: string | null;
  /** Re-attempt the connection (error state's primary action). */
  onRetry: () => void;
  /** Bail out back to the play screen. Available in every state so the
   *  player is never stuck — the original hang had no exit at all. */
  onLeave: () => void;
}

const DEFAULT_ERROR =
  "We couldn't connect to the game server. It may be temporarily unavailable — please try again in a moment.";

/**
 * Connection-status screen shown by GamePage for WebSocket-backed games
 * (PvP / vs-AI-online / spectator) while the socket is connecting, retrying,
 * or has failed. Replaces the indefinite "Loading game..." board so a dead
 * socket (e.g. the server returning HTTP 404 on every `/ws/games/...`
 * upgrade) surfaces a clear failure with a way back instead of hanging.
 */
export function ConnectionStatusScreen({
  state,
  error,
  onRetry,
  onLeave,
}: ConnectionStatusScreenProps) {
  const isError = state === 'error';

  const heading = isError
    ? 'Connection failed'
    : state === 'reconnecting'
      ? 'Reconnecting…'
      : 'Connecting…';

  const detail = isError
    ? (error?.trim() ? error : DEFAULT_ERROR)
    : state === 'reconnecting'
      ? 'Lost contact with the game server. Trying to reconnect…'
      : 'Connecting to the game server…';

  return (
    <div
      data-testid="connection-status-screen"
      className="flex flex-col items-center justify-center h-full gap-5 p-8 text-center"
    >
      {!isError && (
        <div
          aria-hidden="true"
          className="h-10 w-10 rounded-full border-2 border-[var(--line-1)] border-t-[var(--accent)] animate-spin"
        />
      )}

      <div className="flex flex-col gap-2 max-w-sm">
        <h2
          className={`text-xl font-semibold ${
            isError ? 'text-[var(--danger)]' : 'text-[var(--ink-0)]'
          }`}
        >
          {heading}
        </h2>
        <p className="text-sm text-[var(--ink-1)]">{detail}</p>
      </div>

      <div className="flex items-center gap-3">
        {isError && (
          <button
            type="button"
            onClick={onRetry}
            className="px-5 py-2 rounded bg-[var(--accent)] hover:opacity-90 text-[var(--accent-ink)] text-sm font-medium"
          >
            Try again
          </button>
        )}
        <button
          type="button"
          onClick={onLeave}
          className="px-5 py-2 rounded border border-[var(--line-1)] bg-[var(--surface-raised)] hover:opacity-90 text-[var(--ink-0)] text-sm font-medium"
        >
          Back to play
        </button>
      </div>
    </div>
  );
}
