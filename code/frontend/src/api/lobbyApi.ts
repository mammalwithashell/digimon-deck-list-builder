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

export type FirstPlayerChoice = '1' | 'random' | '2';

export interface LobbyState {
  game_id: string;
  join_code: string | null;
  host_display_name: string | null;
  joiner_display_name: string | null;
  host_deck_ready: boolean;
  joiner_deck_ready: boolean;
  first_player: FirstPlayerChoice | null;
  started: boolean;
  your_seat: 1 | 2 | null;
  allow_spectators?: boolean;
  spectator_mode?: 'hidden' | 'open';
}

interface JoinLobbyResponse {
  game_id: string;
  your_seat: 2;
  state: LobbyState;
}

interface SetDeckParams {
  deck?: string[];
  deck_raw?: string;
}

export async function createLobby(params: CreateLobbyParams): Promise<CreateLobbyResponse> {
  const { data } = await client.post<CreateLobbyResponse>('/lobby/create', params);
  return data;
}

export async function getLobbyState(gameId: string): Promise<LobbyState> {
  const { data } = await client.get<LobbyState>(`/lobby/${gameId}/state`);
  return data;
}

export async function setLobbyDeck(gameId: string, params: SetDeckParams): Promise<LobbyState> {
  const { data } = await client.put<LobbyState>(`/lobby/${gameId}/deck`, params);
  return data;
}

/** Reserve the joiner seat. No deck required — pick one inside the room. */
export async function joinLobby(joinCode: string): Promise<JoinLobbyResponse> {
  const { data } = await client.post<JoinLobbyResponse>(`/lobby/join/${joinCode.trim()}`);
  return data;
}

export async function setFirstPlayer(
  gameId: string,
  firstPlayer: FirstPlayerChoice,
): Promise<LobbyState> {
  const { data } = await client.put<LobbyState>(`/lobby/${gameId}/first-player`, {
    first_player: firstPlayer,
  });
  return data;
}

/** Host-only: constructs the game once both decks are locked. */
export async function startLobby(gameId: string): Promise<LobbyState> {
  const { data } = await client.post<LobbyState>(`/lobby/${gameId}/start`);
  return data;
}

/** Joiner-only: vacate the seat before start. */
export async function leaveLobby(gameId: string): Promise<LobbyState> {
  const { data } = await client.post<LobbyState>(`/lobby/${gameId}/leave`);
  return data;
}

export async function cancelLobby(gameId: string): Promise<void> {
  await client.delete(`/lobby/${gameId}`);
}
