import client, { getGameClient } from './client';
import type { ActionTrace, GameState, GameEvent, TensorSummary } from '@/types/game';
import * as rustGameApi from './rustGameApi';

// Desktop builds dispatch every game API call through the in-process Rust
// engine via Tauri `invoke()`; web builds hit the hosted FastAPI game
// endpoints over HTTP. The adapter matches these exports' shapes so call
// sites (store, hooks, pages) are unchanged. See `rustGameApi.ts`.
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

export type PlayerKind = 'human' | 'greedy' | 'trained';

interface CreateGameParams {
  deck1?: string[];
  deck2?: string[];
  deck1_raw?: string;
  deck2_raw?: string;
  player1_type?: string;
  player2_type?: string;
  player1_policy?: string;
  player2_policy?: string;
  agent_action_delay_ms?: number;
  /// Per-player kinds used by the Rust backend (position == PlayerId).
  /// The hosted API ignores these and uses `playerN_type`/`playerN_policy`
  /// instead; the Rust adapter translates between the two so callers can
  /// pass either shape.
  player_kinds?: PlayerKind[];
  /// Per-player ONNX model IDs — only honoured for seats whose kind is
  /// 'trained'. Must be pre-loaded via `desktopModelsApi.loadCached(id)`
  /// before `createGame` is called.
  player_model_ids?: (string | null)[];
}

interface CreateGameResponse {
  game_id: string;
  state: GameState;
  action_mask: number[];
  recording_metadata?: Record<string, unknown>;
  player_labels?: Record<number, string>;
}

interface ActionResponse {
  state: GameState;
  action_mask: number[];
  is_game_over: boolean;
  logs?: string[];
  events?: GameEvent[];
  action_context?: Record<string, unknown>;
  action_traces?: ActionTrace[];
}

interface StepResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  events?: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
  action_traces?: ActionTrace[];
}

export async function createGame(params: CreateGameParams): Promise<CreateGameResponse> {
  if (IS_DESKTOP) return rustGameApi.createGame(params);
  const client = getGameClient();
  const { data } = await client.post<CreateGameResponse>('/games', params);
  return data;
}

export async function sendAction(gameId: string, action: number): Promise<ActionResponse> {
  if (IS_DESKTOP) return rustGameApi.sendAction(gameId, action);
  const client = getGameClient();
  const { data } = await client.post<ActionResponse>(`/games/${gameId}/actions`, { action });
  return data;
}

export async function stepGame(gameId: string): Promise<StepResponse> {
  if (IS_DESKTOP) return rustGameApi.stepGame(gameId);
  const client = getGameClient();
  const { data } = await client.post<StepResponse>(`/games/${gameId}/steps`);
  return data;
}

export async function getState(gameId: string): Promise<GameState> {
  if (IS_DESKTOP) return rustGameApi.getState(gameId);
  const client = getGameClient();
  const { data } = await client.get<GameState>(`/games/${gameId}/state`);
  return data;
}

export async function getMask(gameId: string): Promise<number[]> {
  if (IS_DESKTOP) return rustGameApi.getMask(gameId);
  const client = getGameClient();
  const { data } = await client.get<{ action_mask: number[] }>(`/games/${gameId}/action-mask`);
  return data.action_mask;
}

export async function getLog(gameId: string): Promise<string[]> {
  if (IS_DESKTOP) return rustGameApi.getLog(gameId);
  const client = getGameClient();
  const { data } = await client.get<{ logs: string[] }>(`/games/${gameId}/logs`);
  return data.logs;
}

export async function getBoardTensorSummary(
  gameId: string,
  playerId: number,
): Promise<TensorSummary> {
  if (IS_DESKTOP) return rustGameApi.getBoardTensorSummary(gameId, playerId);
  throw new Error('Board tensor summary is only available in the desktop client');
}

interface SurrenderResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  events?: GameEvent[];
  is_game_over: boolean;
  surrendered_by: number;
}

export async function surrenderGame(gameId: string, playerId: number): Promise<SurrenderResponse> {
  if (IS_DESKTOP) return rustGameApi.surrenderGame(gameId, playerId);
  const client = getGameClient();
  const { data } = await client.post<SurrenderResponse>(`/games/${gameId}/surrender`, { player_id: playerId });
  return data;
}

export async function deleteGame(gameId: string): Promise<void> {
  if (IS_DESKTOP) return rustGameApi.deleteGame(gameId);
  const client = getGameClient();
  await client.delete(`/games/${gameId}`);
}

/** Stage a manifest model on the server, then create a server-side
 *  vs-AI game. Used by the Models page `Try online` button. */
export async function createVsAiGame(params: {
  modelId: string;
  userDeck: { main_deck: string[]; egg_deck: string[] };
  opponentDeck: { main_deck: string[]; egg_deck: string[] };
}): Promise<{ game_id: string }> {
  // Step 1: ask the server to stage the ONNX blob by model_id.
  const { data: prepared } = await client.post<{ filename: string; downloaded: boolean }>(
    `/models/${params.modelId}/prepare`,
  );
  // Step 2: create the game against the staged filename.
  const { data } = await client.post<{ game_id: string }>('/games', {
    deck1: [...params.userDeck.egg_deck, ...params.userDeck.main_deck],
    deck2: [...params.opponentDeck.egg_deck, ...params.opponentDeck.main_deck],
    player1_type: 'human',
    player2_type: 'agent',
    player1_policy: 'human',
    player2_policy: 'trained',
    player2_model: prepared.filename,
  });
  return data;
}
