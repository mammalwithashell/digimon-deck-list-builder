import client from './client';
import type { GameState } from '@/types/game';

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
  action_context?: Record<string, unknown>;
}

interface StepResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  is_human_turn: boolean;
  is_game_over: boolean;
}

export async function createGame(params: CreateGameParams): Promise<CreateGameResponse> {
  const { data } = await client.post<CreateGameResponse>('/games', params);
  return data;
}

export async function sendAction(gameId: string, action: number): Promise<ActionResponse> {
  const { data } = await client.post<ActionResponse>(`/games/${gameId}/actions`, { action });
  return data;
}

export async function stepGame(gameId: string): Promise<StepResponse> {
  const { data } = await client.post<StepResponse>(`/games/${gameId}/steps`);
  return data;
}

export async function getState(gameId: string): Promise<GameState> {
  const { data } = await client.get<GameState>(`/games/${gameId}/state`);
  return data;
}

export async function getMask(gameId: string): Promise<number[]> {
  const { data } = await client.get<{ action_mask: number[] }>(`/games/${gameId}/action-mask`);
  return data.action_mask;
}

export async function getLog(gameId: string): Promise<string[]> {
  const { data } = await client.get<{ logs: string[] }>(`/games/${gameId}/logs`);
  return data.logs;
}

export async function deleteGame(gameId: string): Promise<void> {
  await client.delete(`/games/${gameId}`);
}
