import { getGameClient } from './client';
import type { GameState, GameEvent } from '@/types/game';
import * as rustGameApi from './rustGameApi';

// When the frontend is built with `VITE_ENGINE=rust`, all game API calls
// dispatch through the in-process Rust engine via Tauri `invoke()` instead
// of the Python sidecar. The adapter matches these exports' shapes, so
// call sites (store, hooks, pages) are unchanged. See `rustGameApi.ts`.
const USE_RUST_ENGINE = import.meta.env.VITE_ENGINE === 'rust';

interface CreateGameParams {
  deck1?: string[];
  deck2?: string[];
  deck1_raw?: string;
  deck2_raw?: string;
  player1_type?: string;
  player2_type?: string;
  player1_policy?: string;
  player2_policy?: string;
  /** Slug from the model catalog or legacy `.onnx` filename. The sidecar
   *  resolves both against its cache + bundled dirs. Pair with
   *  `player*_policy = 'trained'` so the engine actually loads it. */
  player1_model?: string;
  player2_model?: string;
  agent_action_delay_ms?: number;
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
}

interface StepResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  events?: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
}

export async function createGame(params: CreateGameParams): Promise<CreateGameResponse> {
  if (USE_RUST_ENGINE) return rustGameApi.createGame(params);
  const client = getGameClient();
  const { data } = await client.post<CreateGameResponse>('/games', params);
  return data;
}

export async function sendAction(gameId: string, action: number): Promise<ActionResponse> {
  if (USE_RUST_ENGINE) return rustGameApi.sendAction(gameId, action);
  const client = getGameClient();
  const { data } = await client.post<ActionResponse>(`/games/${gameId}/actions`, { action });
  return data;
}

export async function stepGame(gameId: string): Promise<StepResponse> {
  if (USE_RUST_ENGINE) return rustGameApi.stepGame(gameId);
  const client = getGameClient();
  const { data } = await client.post<StepResponse>(`/games/${gameId}/steps`);
  return data;
}

export async function getState(gameId: string): Promise<GameState> {
  if (USE_RUST_ENGINE) return rustGameApi.getState(gameId);
  const client = getGameClient();
  const { data } = await client.get<GameState>(`/games/${gameId}/state`);
  return data;
}

export async function getMask(gameId: string): Promise<number[]> {
  if (USE_RUST_ENGINE) return rustGameApi.getMask(gameId);
  const client = getGameClient();
  const { data } = await client.get<{ action_mask: number[] }>(`/games/${gameId}/action-mask`);
  return data.action_mask;
}

export async function getLog(gameId: string): Promise<string[]> {
  if (USE_RUST_ENGINE) return rustGameApi.getLog(gameId);
  const client = getGameClient();
  const { data } = await client.get<{ logs: string[] }>(`/games/${gameId}/logs`);
  return data.logs;
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
  if (USE_RUST_ENGINE) return rustGameApi.surrenderGame(gameId, playerId);
  const client = getGameClient();
  const { data } = await client.post<SurrenderResponse>(`/games/${gameId}/surrender`, { player_id: playerId });
  return data;
}

export async function deleteGame(gameId: string): Promise<void> {
  if (USE_RUST_ENGINE) return rustGameApi.deleteGame(gameId);
  const client = getGameClient();
  await client.delete(`/games/${gameId}`);
}
