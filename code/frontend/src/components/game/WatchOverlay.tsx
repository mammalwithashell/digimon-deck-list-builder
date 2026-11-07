import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { WATCH_SPEEDS, type WatchReplay, type WatchSpeed } from '@/hooks/useWatchReplay';
import type { WatchSearchView } from '@/features/watch/watchQueue';

function formatQ(q: number | null): string {
  if (q === null) return '—';
  return (q >= 0 ? '+' : '') + q.toFixed(2);
}

function SearchTable({ title, view, seatLabels }: {
  title: string;
  view: WatchSearchView | null;
  seatLabels: [string, string];
}) {
  if (!view) {
    return (
      <div className="text-xs text-gray-400">
        <div className="font-semibold text-gray-300 mb-1">{title}</div>
        Forced or no search recorded.
      </div>
    );
  }
  return (
    <div className="text-xs">
      <div className="font-semibold text-gray-300 mb-1">
        {title} · {seatLabels[view.actor] ?? `Seat ${view.actor + 1}`} · {view.phase} ·{' '}
        {view.candidates} options
      </div>
      <table className="w-full" data-testid="watch-search-table">
        <thead>
          <tr className="text-gray-500">
            <th className="text-left font-normal">Plan</th>
            <th className="text-right font-normal" title="Share of search visits">Visits</th>
            <th className="text-right font-normal" title="Policy prior before search">Prior</th>
            <th className="text-right font-normal" title="Search value estimate for this plan (−1 to +1)">Q</th>
          </tr>
        </thead>
        <tbody>
          {view.plans.map((p, i) => (
            <tr key={i} className={p.selected ? 'text-amber-300' : 'text-gray-200'}>
              <td className="pr-2 truncate max-w-[260px]" title={p.label}>
                {p.selected ? '▶ ' : ''}{p.label}
              </td>
              <td className="text-right tabular-nums">{Math.round(p.share * 100)}%</td>
              <td className="text-right tabular-nums">{Math.round(p.prior * 100)}%</td>
              <td className="text-right tabular-nums">{formatQ(p.q)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Scrub to any move; the jump happens when the handle is released. */
function MoveSlider({ replay }: { replay: WatchReplay }) {
  const [draft, setDraft] = useState<number | null>(null);
  const commit = () => {
    if (draft !== null) replay.seek(draft);
    setDraft(null);
  };
  return (
    <div className="flex items-center gap-2 text-xs text-gray-400">
      <input
        data-testid="watch-seek"
        type="range"
        className="flex-1"
        min={0}
        max={replay.totalSteps}
        value={draft ?? replay.step}
        disabled={replay.seeking || replay.totalSteps === 0}
        onChange={(e) => setDraft(Number(e.target.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        aria-label="Move"
      />
      <span className="w-14 text-right tabular-nums">{draft ?? replay.step}/{replay.totalSteps}</span>
    </div>
  );
}

/** Floating control panel for watch mode (recorded model-vs-model replay). */
export function WatchOverlay({ replay, queue }: { replay: WatchReplay; queue: string }) {
  const [collapsed, setCollapsed] = useState(false);

  // ← / → step, space plays/pauses (ignored while typing in a field).
  const { stepBack, stepOnce, togglePlay } = replay;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'SELECT' || t.tagName === 'TEXTAREA')) return;
      if (e.key === 'ArrowLeft') stepBack();
      else if (e.key === 'ArrowRight') stepOnce();
      else if (e.key === ' ') togglePlay();
      else return;
      e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [stepBack, stepOnce, togglePlay]);
  const { row, detail } = replay;
  const seatLabels: [string, string] = detail?.seat_labels ?? ['Player 1', 'Player 2'];
  const winner =
    row && row.winner !== null && replay.step >= replay.totalSteps
      ? `${seatLabels[row.winner]} won${row.terminal_reason ? ` (${row.terminal_reason.replace(/^Some\(|\)$/g, '')})` : ''}`
      : null;

  return (
    <div
      data-testid="watch-overlay"
      className="fixed bottom-3 right-3 z-50 w-[420px] rounded-lg border border-gray-700 bg-gray-900/95 p-3 text-sm text-gray-100 shadow-xl"
    >
      <div className="flex items-center justify-between gap-2">
        <div className="font-semibold">
          Watching {replay.position + 1}/{replay.playlistLength || '…'}
          {row ? ` · ${row.archetypes[0]} vs ${row.archetypes[1]}` : ''}
        </div>
        <button className="text-gray-400 hover:text-white" onClick={() => setCollapsed((c) => !c)}>
          {collapsed ? 'Show' : 'Hide'}
        </button>
      </div>
      {!collapsed && (
        <div className="mt-2 space-y-2">
          <div className="text-xs text-gray-400">
            P1: {seatLabels[0]} · P2: {seatLabels[1]} · move {replay.step}/{replay.totalSteps}
            {replay.seeking && <span className="text-amber-300"> · jumping…</span>}
          </div>
          <MoveSlider replay={replay} />
          <div className="flex flex-wrap items-center gap-2">
            <button
              data-testid="watch-play-toggle"
              className="rounded bg-blue-700 px-2 py-1 hover:bg-blue-600"
              onClick={replay.togglePlay}
            >
              {replay.playing ? 'Pause' : 'Play'}
            </button>
            <button
              data-testid="watch-step-back"
              className="rounded bg-gray-700 px-2 py-1 hover:bg-gray-600 disabled:opacity-40"
              disabled={replay.step <= 0 || replay.seeking}
              onClick={replay.stepBack}
              title="Back one move (←)"
            >
              ◀ Back
            </button>
            <button
              data-testid="watch-step"
              className="rounded bg-gray-700 px-2 py-1 hover:bg-gray-600 disabled:opacity-40"
              disabled={replay.step >= replay.totalSteps || replay.seeking}
              onClick={replay.stepOnce}
              title="Forward one move (→)"
            >
              Step ▶
            </button>
            <select
              className="rounded bg-gray-800 px-1 py-1"
              value={replay.speed}
              onChange={(e) => replay.setSpeed(e.target.value as WatchSpeed)}
            >
              {Object.keys(WATCH_SPEEDS).map((s) => (
                <option key={s} value={s}>{s}</option>
              ))}
            </select>
            <button
              className="rounded bg-gray-700 px-2 py-1 hover:bg-gray-600 disabled:opacity-40"
              disabled={replay.position <= 0}
              onClick={() => replay.goTo(replay.position - 1)}
            >
              Prev game
            </button>
            <button
              className="rounded bg-gray-700 px-2 py-1 hover:bg-gray-600 disabled:opacity-40"
              disabled={replay.position + 1 >= replay.playlistLength}
              onClick={() => replay.goTo(replay.position + 1)}
            >
              Next game
            </button>
            <label className="flex items-center gap-1 text-xs text-gray-300">
              <input
                type="checkbox"
                checked={replay.autoAdvance}
                onChange={(e) => replay.setAutoAdvance(e.target.checked)}
              />
              auto-next
            </label>
            <Link to={`/watch?queue=${encodeURIComponent(queue)}`} className="text-xs text-blue-300 hover:underline">
              Queue
            </Link>
          </div>
          {winner && <div className="font-semibold text-green-300">{winner}</div>}
          {replay.error && (
            <div className="text-xs text-red-300">
              {replay.error}{' '}
              <button className="underline" onClick={replay.restart}>Restart</button>
            </div>
          )}
          <SearchTable title="Last move" view={replay.lastSearch} seatLabels={seatLabels} />
          {replay.step < replay.totalSteps && (
            <SearchTable title="Next move" view={replay.nextSearch} seatLabels={seatLabels} />
          )}
        </div>
      )}
    </div>
  );
}
