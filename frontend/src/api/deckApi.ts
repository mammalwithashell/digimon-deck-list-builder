import client from './client';
import type { DeckSummary } from '@/types/deck';

interface CreateDeckParams {
  name: string;
  description?: string;
  game_mode?: string;
  main_deck: string[];
  egg_deck: string[];
  /** Optional parallel bool arrays marking which slots are alt-art. */
  main_deck_alt_arts?: boolean[];
  egg_deck_alt_arts?: boolean[];
  is_public?: boolean;
  tags?: string[];
}

interface DeckResponse {
  id: string;
  owner_id: string;
  name: string;
  description: string;
  game_mode: string;
  main_deck: string[];
  egg_deck: string[];
  main_deck_alt_arts?: boolean[];
  egg_deck_alt_arts?: boolean[];
  commander_id: string | null;
  is_valid: boolean;
  validation_errors: string[];
  is_public: boolean;
  tags: string[];
  created_at: string;
  updated_at: string;
}

interface ParseDeckResponse {
  main_deck: string[];
  egg_deck: string[];
  warnings?: string[];
}

interface ValidateDeckResponse {
  valid: boolean;
  errors: { field: string; message: string }[];
  warnings: { field: string; message: string }[];
}

interface BackendValidateDeckResponse {
  is_valid: boolean;
  errors: string[];
  warnings: string[];
}

export async function createDeck(params: CreateDeckParams): Promise<DeckResponse> {
  const { data } = await client.post<DeckResponse>('/decks', params);
  return data;
}

export async function listDecks(gameMode?: string): Promise<DeckSummary[]> {
  const params = gameMode ? { game_mode: gameMode } : {};
  const { data } = await client.get<DeckSummary[]>('/decks', { params });
  return data;
}

export async function getDeck(deckId: string): Promise<DeckResponse> {
  const { data } = await client.get<DeckResponse>(`/decks/${deckId}`);
  return data;
}

export async function updateDeck(
  deckId: string,
  params: Partial<CreateDeckParams> & { change_note?: string },
): Promise<DeckResponse> {
  const { data } = await client.put<DeckResponse>(`/decks/${deckId}`, params);
  return data;
}

export async function deleteDeck(deckId: string): Promise<void> {
  await client.delete(`/decks/${deckId}`);
}

export async function validateDeck(deckId: string): Promise<DeckResponse> {
  const { data } = await client.post<DeckResponse>(`/decks/${deckId}/validate`);
  return data;
}

export async function parseDeck(deckString: string): Promise<ParseDeckResponse> {
  const { data } = await client.post<ParseDeckResponse>('/decks/parse', { deck: deckString });
  return data;
}

interface TestedCardsResponse {
  card_ids: string[];
  card_count: number;
}

export async function listTestedCards(): Promise<string[]> {
  const { data } = await client.get<TestedCardsResponse>('/decks/tested-cards');
  return data.card_ids;
}

export async function validateDeckRaw(
  mainDeck: string[],
  eggDeck: string[],
  gameMode?: string,
): Promise<ValidateDeckResponse> {
  const { data } = await client.post<BackendValidateDeckResponse>('/decks/validate', {
    main_deck: mainDeck,
    egg_deck: eggDeck,
    game_mode: gameMode ?? 'standard',
  });

  return {
    valid: data.is_valid,
    errors: data.errors.map((message) => ({ field: 'deck', message })),
    warnings: data.warnings.map((message) => ({ field: 'deck', message })),
  };
}
