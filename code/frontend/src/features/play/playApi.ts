import { invoke } from '@tauri-apps/api/core';
import client from '@/api/client';
import * as deckLibrary from '@/api/deckLibraryAdapter';
import * as gameApi from '@/api/gameApi';
import * as lobbyApi from '@/api/lobbyApi';
import * as matchmaking from '@/api/matchmaking';
import type { DeckResponse } from '@/types/deck';
import type { PlayFormat, PlayFormatId } from './formatCatalog';
import { PLAY_FORMATS, formatToQueueType } from './formatCatalog';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

function hasTauriBridge(): boolean {
  return Boolean((globalThis as { isTauri?: boolean }).isTauri);
}

interface FormatDto {
  id: PlayFormatId;
  name: string;
  tagline: string;
  description: string;
  deck_label: string;
  population_pct: number;
  enabled: boolean;
  disabled_reason?: string | null;
}

function fromDto(dto: FormatDto): PlayFormat {
  return {
    id: dto.id,
    name: dto.name,
    tagline: dto.tagline,
    description: dto.description,
    deckLabel: dto.deck_label,
    populationPct: dto.population_pct,
    enabled: dto.enabled,
    disabledReason: dto.disabled_reason ?? undefined,
  };
}

export async function listFormats(): Promise<PlayFormat[]> {
  if (IS_DESKTOP) {
    try {
      return (await invoke<FormatDto[]>('formats_list')).map(fromDto);
    } catch {
      return PLAY_FORMATS;
    }
  }
  try {
    const { data } = await client.get<FormatDto[]>('/formats');
    return data.map(fromDto);
  } catch {
    return PLAY_FORMATS;
  }
}

export async function getDeck(deckId: string): Promise<DeckResponse> {
  return deckLibrary.getDeck(deckId);
}

export async function queueQuickMatch(params: {
  formatId: PlayFormatId;
  deck: DeckResponse;
}): Promise<matchmaking.QueueResponse> {
  return matchmaking.queue({
    queue_type: formatToQueueType(params.formatId),
    main_deck: params.deck.main_deck,
    egg_deck: params.deck.egg_deck,
    game_mode: params.formatId,
  });
}

export async function createRoom(params: {
  formatId: PlayFormatId;
}): Promise<{ game_id: string; join_code: string }> {
  void params.formatId;
  return lobbyApi.createLobby({
    is_public: false,
    allow_spectators: true,
    spectator_mode: 'hidden',
  });
}

export async function setRoomDeck(params: {
  gameId: string;
  deck: DeckResponse;
}): Promise<lobbyApi.LobbyState> {
  return lobbyApi.setLobbyDeck(params.gameId, {
    deck: [...params.deck.egg_deck, ...params.deck.main_deck],
  });
}

export async function getRoomState(gameId: string): Promise<lobbyApi.LobbyState> {
  return lobbyApi.getLobbyState(gameId);
}

/** Reserve the joiner seat in a room by its 5-digit code. */
export async function joinRoom(code: string): Promise<{ game_id: string; your_seat: 2 }> {
  const { game_id, your_seat } = await lobbyApi.joinLobby(code);
  return { game_id, your_seat };
}

export async function setRoomFirstPlayer(params: {
  gameId: string;
  firstPlayer: lobbyApi.FirstPlayerChoice;
}): Promise<lobbyApi.LobbyState> {
  return lobbyApi.setFirstPlayer(params.gameId, params.firstPlayer);
}

export async function startRoom(gameId: string): Promise<lobbyApi.LobbyState> {
  return lobbyApi.startLobby(gameId);
}

export async function leaveRoom(gameId: string): Promise<lobbyApi.LobbyState> {
  return lobbyApi.leaveLobby(gameId);
}

export async function cancelRoom(gameId: string): Promise<void> {
  return lobbyApi.cancelLobby(gameId);
}

export async function createBotGame(params: {
  deck: DeckResponse;
  opponentDeck: DeckResponse;
}): Promise<{ game_id: string }> {
  const deck1 = [...params.deck.egg_deck, ...params.deck.main_deck];
  const deck2 = [...params.opponentDeck.egg_deck, ...params.opponentDeck.main_deck];
  if (!hasTauriBridge()) {
    const { data } = await client.post<{ game_id: string }>('/games', {
      deck1,
      deck2,
      player1_type: 'human',
      player2_type: 'agent',
      player2_policy: 'greedy',
    });
    return { game_id: data.game_id };
  }
  const response = await gameApi.createGame({
    deck1,
    deck2,
    player_kinds: ['human', 'greedy'],
    player_model_ids: [null, null],
  });
  return { game_id: response.game_id };
}
