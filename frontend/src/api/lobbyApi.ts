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

export async function createLobby(params: CreateLobbyParams): Promise<CreateLobbyResponse> {
  const { data } = await client.post<CreateLobbyResponse>('/lobby/create', params);
  return data;
}

export async function joinLobby(joinCode: string, params: JoinLobbyParams): Promise<JoinLobbyResponse> {
  const { data } = await client.post<JoinLobbyResponse>(`/lobby/join/${joinCode}`, params);
  return data;
}

export async function cancelLobby(gameId: string): Promise<void> {
  await client.delete(`/lobby/${gameId}`);
}
