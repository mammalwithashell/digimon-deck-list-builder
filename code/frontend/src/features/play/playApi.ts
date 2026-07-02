import client from '@/api/client';
import * as deckLibrary from '@/api/deckLibraryAdapter';
import * as gameApi from '@/api/gameApi';
import type { PlayerKind } from '@/api/gameApi';
import * as lobbyApi from '@/api/lobbyApi';
import * as matchmaking from '@/api/matchmaking';
import type { DeckResponse } from '@/types/deck';
import type { PlayFormatId } from './formatCatalog';
import { formatToQueueType } from './formatCatalog';
import { resolveModelForDeck } from '@/api/desktopModelsApi';
import { isStarterDeckId, STARTER_DECKS } from './starterDecks.generated';
import type { StarterDeckId } from './starterDecks.generated';
import { fnv1a } from './hash';

const MANIFEST_BASE = (import.meta.env.VITE_MODELS_MANIFEST_URL as string | undefined) ?? '';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

function hasTauriBridge(): boolean {
  return Boolean((globalThis as { isTauri?: boolean }).isTauri);
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

export async function setRoomSeed(params: {
  gameId: string;
  seed: string | null;
}): Promise<lobbyApi.LobbyState> {
  return lobbyApi.setSeed(params.gameId, params.seed);
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
  seed?: string | null;
}): Promise<{ game_id: string; seed?: string }> {
  const deck1 = [...params.deck.egg_deck, ...params.deck.main_deck];
  const deck2 = [...params.opponentDeck.egg_deck, ...params.opponentDeck.main_deck];
  if (!hasTauriBridge()) {
    const { data } = await client.post<{ game_id: string; seed?: string }>('/games', {
      deck1,
      deck2,
      player1_type: 'human',
      player2_type: 'agent',
      player2_policy: 'greedy',
      seed: params.seed,
    });
    return { game_id: data.game_id, seed: data.seed };
  }
  const response = await gameApi.createGame({
    deck1,
    deck2,
    player_kinds: ['human', 'greedy'],
    player_model_ids: [null, null],
    seed: params.seed,
  });
  return { game_id: response.game_id, seed: response.seed };
}

/** The 6 bundled starter decks (same data in desktop + browser builds). */
export async function listStarterDecks(): Promise<DeckResponse[]> {
  return STARTER_DECKS;
}

/** Deterministic-from-seed index in [0, count). FNV-1a so a given shuffle
 *  seed reproduces the same AI deck; random when no seed is set. */
export function starterIndexFromSeed(seed: string | null, count: number): number {
  if (!seed) return Math.floor(Math.random() * count);
  return fnv1a(seed) % count;
}

/** Sentinel for "let the AI pick a random starter deck (seed-derived)". Any
 *  other `opponent` value is a specific starter deck id the AI will pilot. */
export const AI_STARTER_RANDOM = 'random' as const;

export type AiStarterOpponent = typeof AI_STARTER_RANDOM | StarterDeckId;

export function isAiStarterOpponent(value: string): value is AiStarterOpponent {
  return value === AI_STARTER_RANDOM || isStarterDeckId(value);
}

/** Start a game vs the AI: player pilots `deck`. The AI pilots either a
 *  seed-derived random starter deck (`opponent` unset / `'random'`) or a
 *  specific starter deck the player chose. Each starter deck is piloted by its
 *  trained specialist (matched by `starter_deck` slug) on desktop when one is
 *  published; otherwise the greedy CPU. */
export async function createAiStarterGame(params: {
  deck: DeckResponse;
  starterDecks: DeckResponse[];
  seed?: string | null;
  /** `'random'`/unset -> seed-derived deck; otherwise a starter deck id the
   *  AI pilots with that deck's specialist. */
  opponent?: AiStarterOpponent;
}): Promise<{ game_id: string; seed?: string; aiDeckName: string }> {
  const seed = params.seed ?? null;
  const opponent = params.opponent ?? AI_STARTER_RANDOM;
  const chosen =
    opponent !== AI_STARTER_RANDOM
      ? params.starterDecks.find((d) => d.id === opponent)
      : undefined;
  const aiDeck =
    chosen ?? params.starterDecks[starterIndexFromSeed(seed, params.starterDecks.length)]!;
  const deck1 = [...params.deck.egg_deck, ...params.deck.main_deck];
  const deck2 = [...aiDeck.egg_deck, ...aiDeck.main_deck];

  // Resolve the trained specialist for the AI's deck (desktop only); any
  // failure / no published specialist -> greedy CPU.
  let modelId: string | null = null;
  if (IS_DESKTOP && MANIFEST_BASE) {
    try {
      modelId = await resolveModelForDeck(MANIFEST_BASE, aiDeck.id);
    } catch {
      modelId = null;
    }
  }

  if (!hasTauriBridge()) {
    const { data } = await client.post<{ game_id: string; seed?: string }>('/games', {
      deck1,
      deck2,
      player1_type: 'human',
      player2_type: 'agent',
      player2_policy: 'greedy',
      seed,
    });
    return { game_id: data.game_id, seed: data.seed, aiDeckName: aiDeck.name };
  }

  const kinds: PlayerKind[] = ['human', modelId ? 'trained' : 'greedy'];
  const response = await gameApi.createGame({
    deck1,
    deck2,
    player_kinds: kinds,
    player_model_ids: [null, modelId],
    seed,
  });
  return { game_id: response.game_id, seed: response.seed, aiDeckName: aiDeck.name };
}
