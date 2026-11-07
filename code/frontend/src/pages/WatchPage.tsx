import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useGameStore } from '@/stores/gameStore';
import {
  createWatchGame,
  loadWatchGame,
  loadWatchQueue,
  watchGameUrl,
  watchPlaylist,
  type WatchQueueIndex,
} from '@/features/watch/watchQueue';

const DEFAULT_QUEUE = 'cp8-mirror';

/**
 * Lists a watch queue (recorded model-vs-model games) and starts playback.
 * Starting a game opens the normal board in watch mode, which then walks the
 * filtered playlist game by game.
 */
export function WatchPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const queue = searchParams.get('queue') ?? DEFAULT_QUEUE;
  const filter = searchParams.get('filter') ?? '';
  const resetStore = useGameStore((s) => s.reset);
  const [index, setIndex] = useState<WatchQueueIndex | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [starting, setStarting] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIndex(null);
    setError(null);
    loadWatchQueue(queue)
      .then((idx) => !cancelled && setIndex(idx))
      .catch((err) => !cancelled && setError(err instanceof Error ? err.message : String(err)));
    return () => {
      cancelled = true;
    };
  }, [queue]);

  const playlist = useMemo(() => (index ? watchPlaylist(index, filter) : []), [index, filter]);
  const archetypes = useMemo(
    () => (index ? [...new Set(index.games.flatMap((g) => g.archetypes))].sort() : []),
    [index],
  );
  const skipped = index ? index.games.filter((g) => g.replayable === false).length : 0;

  const start = async (position: number) => {
    const row = playlist[position];
    if (!row) return;
    setStarting(position);
    setError(null);
    try {
      const detail = await loadWatchGame(queue, row.id);
      const gameId = await createWatchGame(detail);
      resetStore();
      navigate(watchGameUrl(gameId, queue, position, filter));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStarting(null);
    }
  };

  return (
    <div className="h-full overflow-y-auto p-6 text-gray-100" data-testid="watch-page">
      <h1 className="text-2xl font-bold">Watch: {index?.title ?? queue}</h1>
      <p className="mt-1 text-sm text-gray-400">
        Recorded games replayed move by move on the board. Both seats played with search; the board
        panel shows what the search considered for each move.
        {' '}Same-archetype games are hidden.
        {skipped > 0 && ` ${skipped} games are hidden because the current engine no longer reproduces them.`}
      </p>
      {error && <div className="mt-3 text-red-300">{error}</div>}
      {index && (
        <>
          <div className="mt-4 flex flex-wrap items-center gap-3">
            <label className="text-sm">
              Archetype{' '}
              <select
                className="rounded bg-gray-800 px-2 py-1"
                value={filter}
                onChange={(e) => {
                  const next = new URLSearchParams(searchParams);
                  if (e.target.value) next.set('filter', e.target.value);
                  else next.delete('filter');
                  setSearchParams(next);
                }}
              >
                <option value="">All ({watchPlaylist(index, '').length})</option>
                {archetypes.map((a) => (
                  <option key={a} value={a}>{a}</option>
                ))}
              </select>
            </label>
            <button
              data-testid="watch-start-all"
              className="rounded bg-blue-700 px-3 py-1 hover:bg-blue-600 disabled:opacity-50"
              disabled={playlist.length === 0 || starting !== null}
              onClick={() => void start(0)}
            >
              Play queue from the start
            </button>
            <span className="text-sm text-gray-400">{playlist.length} games</span>
          </div>
          <table className="mt-4 w-full text-sm">
            <thead className="text-left text-gray-400">
              <tr>
                <th className="py-1">#</th>
                <th>Player 1</th>
                <th>Player 2</th>
                <th>Went first</th>
                <th>Winner</th>
                <th className="text-right">Moves</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {playlist.map((g, i) => (
                <tr key={g.id} className="border-t border-gray-800">
                  <td className="py-1 text-gray-500">{i + 1}</td>
                  <td>{g.archetypes[0]} <span className="text-gray-500">· {g.seat_labels[0]}</span></td>
                  <td>{g.archetypes[1]} <span className="text-gray-500">· {g.seat_labels[1]}</span></td>
                  <td>P{g.first_player + 1}</td>
                  <td>{g.winner === null ? '—' : `P${g.winner + 1}`}</td>
                  <td className="text-right tabular-nums">{g.moves}</td>
                  <td className="text-right">
                    <button
                      className="rounded bg-gray-700 px-2 py-0.5 hover:bg-gray-600 disabled:opacity-50"
                      disabled={starting !== null}
                      onClick={() => void start(i)}
                    >
                      {starting === i ? 'Starting…' : 'Watch'}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
