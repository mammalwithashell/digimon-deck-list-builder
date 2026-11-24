/**
 * Watch queues: recorded model-vs-model games replayed in the board UI.
 *
 * A queue is built offline by `code/tools/build_watch_queue.py` from a
 * completed frozen-evaluation panel and served from `public/watch/<name>/`.
 * The engine is deterministic from (decks, seed), so a game is replayed by
 * creating a two-human game with the recorded seed and submitting the
 * recorded actions in order — the board shows exactly what the models played,
 * including their search, without running search in the UI.
 */
import * as gameApi from '@/api/gameApi';

export interface WatchPlanView {
  label: string;
  plan: number[];
  prior: number;
  visits: number;
  share: number;
  q: number | null;
  selected: boolean;
}

export interface WatchSearchView {
  /** 0-indexed seat that searched. */
  actor: number;
  phase: string;
  simulations: number | null;
  candidates: number;
  plans: WatchPlanView[];
}

export interface WatchGameRow {
  id: string;
  block: number;
  archetypes: [string, string];
  /** Index 0 = seat 0 = UI Player 1. */
  seat_labels: [string, string];
  first_player: number;
  /** 0-indexed winning seat, or null for no winner. */
  winner: number | null;
  terminal_reason: string | null;
  moves: number;
  searched: number | null;
  /** Absent when the queue was built without `--verify`. */
  replayable?: boolean;
  replay_problem?: string;
}

export interface WatchQueueIndex {
  title: string;
  source: string;
  games: WatchGameRow[];
  verified_games?: number;
}

export interface WatchGameDetail {
  id: string;
  seed: string;
  deck1: string[];
  deck2: string[];
  actions: number[];
  seat_labels: [string, string];
  /** Keyed by decision index (= index into `actions`). Forced moves are absent. */
  searches: Record<string, WatchSearchView>;
}

const BASE = `${import.meta.env.BASE_URL ?? '/'}watch`;

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}/${path}`);
  if (!res.ok) throw new Error(`Watch queue file ${path} not found (HTTP ${res.status})`);
  return (await res.json()) as T;
}

export function loadWatchQueue(queue: string): Promise<WatchQueueIndex> {
  return getJson<WatchQueueIndex>(`${encodeURIComponent(queue)}/index.json`);
}

export function loadWatchGame(queue: string, gameId: string): Promise<WatchGameDetail> {
  return getJson<WatchGameDetail>(
    `${encodeURIComponent(queue)}/games/${encodeURIComponent(gameId)}.json`,
  );
}

/** Same archetype on both sides. */
export function isMirrorMatch(g: WatchGameRow): boolean {
  return g.archetypes[0] === g.archetypes[1];
}

/**
 * Games that can be replayed on the current engine (unverified rows count as
 * playable). Deck mirrors (same archetype on both sides) are left out.
 */
export function playableGames(index: WatchQueueIndex): WatchGameRow[] {
  return index.games.filter((g) => g.replayable !== false && !isMirrorMatch(g));
}

/** Create the engine game for one recorded game; both seats are driven by the replay. */
export async function createWatchGame(detail: WatchGameDetail): Promise<string> {
  const resp = await gameApi.createGame({
    deck1: detail.deck1,
    deck2: detail.deck2,
    player_kinds: ['human', 'human'],
    player_model_ids: [null, null],
    seed: detail.seed,
  });
  return resp.game_id;
}

/**
 * URL for the board view of a watch game. `position` is the playlist index;
 * `at` (optional) is the move to fast-forward to after the game is created.
 */
export function watchGameUrl(
  gameId: string,
  queue: string,
  position: number,
  filter: string,
  at = 0,
): string {
  const params = new URLSearchParams({ watch: queue, pos: String(position) });
  if (filter) params.set('filter', filter);
  if (at > 0) params.set('at', String(at));
  return `/game/${gameId}?${params.toString()}`;
}

/** The playlist a watch session walks: playable rows, optionally one archetype. */
export function watchPlaylist(index: WatchQueueIndex, filter: string): WatchGameRow[] {
  const rows = playableGames(index);
  if (!filter) return rows;
  return rows.filter((g) => g.archetypes.includes(filter));
}
