import client from './client';

interface CreateLobbyParams {
  deck?: string[];
  deck_raw?: string;
  is_public?: boolean;
  allow_spectators?: boolean;
  spectator_mode?: 'hidden' | 'open';
}

interface CreateLobbyResponse {
  game_id: string;
  join_code: string;
}

interface JoinLobbyParams {
  deck?: string[];
  deck_raw?: string;
}

interface JoinLobbyResponse {
  game_id: string;
  player_id: number;
}

export interface LobbyState {
  game_id: string;
  join_code: string | null;
  host_display_name: string | null;
  host_deck_ready: boolean;
  joiner_deck_ready: boolean;
  started: boolean;
  allow_spectators?: boolean;
  spectator_mode?: 'hidden' | 'open';
}

export async function createLobby(params: CreateLobbyParams): Promise<CreateLobbyResponse> {
  const { data } = await client.post<CreateLobbyResponse>('/lobby/create', params);
  return data;
}

export async function getLobbyState(gameId: string): Promise<LobbyState> {
  const { data } = await client.get<LobbyState>(`/lobby/${gameId}/state`);
  return data;
}

export async function setLobbyDeck(gameId: string, params: JoinLobbyParams): Promise<LobbyState> {
  const { data } = await client.put<LobbyState>(`/lobby/${gameId}/deck`, params);
  return data;
}

export async function joinLobby(joinCode: string, params: JoinLobbyParams): Promise<JoinLobbyResponse> {
  const { data } = await client.post<JoinLobbyResponse>(`/lobby/join/${joinCode}`, params);
  return data;
}

export async function cancelLobby(gameId: string): Promise<void> {
  await client.delete(`/lobby/${gameId}`);
}
