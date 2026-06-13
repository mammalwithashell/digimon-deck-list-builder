import client from './client';
import { isInTauriRuntime } from './engineRuntime';
import type { DigimonCardData } from '@/types/cards';
import type { CardLegality, DeckFolder, DeckFormat, DeckResponse, DeckSummary } from '@/types/deck';

// Mirrors `gameApi.ts` — when running inside Tauri, dispatch parse /
// validate / tested-cards through `invoke()` into the embedded
// `digimon-engine` deck_tools module. Outside Tauri (browser-dev,
// hosted-web) hit the FastAPI endpoints under `/decks/...`. Detection
// is RUNTIME (not build-time) so the same desktop bundle works in a
// plain browser for testing — the build-time `VITE_BUILD_TARGET`
// gate would still route through Tauri and 404 here.
function useTauriBackend(): boolean {
  return isInTauriRuntime();
}

type TauriInvoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await import('@tauri-apps/api/core');
  const invoke = mod.invoke as TauriInvoke;
  return invoke<T>(cmd, args);
}

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

export async function listDeckFolders(): Promise<DeckFolder[]> {
  const { data } = await client.get<DeckFolder[]>('/decks/folders');
  return data;
}

export async function createDeckFolder(name: string): Promise<DeckFolder> {
  const { data } = await client.post<DeckFolder>('/decks/folders', { name });
  return data;
}

export async function updateDeckFolder(
  folderId: string,
  params: { name?: string; sort_order?: number },
): Promise<DeckFolder> {
  const { data } = await client.put<DeckFolder>(`/decks/folders/${folderId}`, params);
  return data;
}

export async function deleteDeckFolder(folderId: string): Promise<void> {
  await client.delete(`/decks/folders/${folderId}`);
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

export async function updateDeckLibraryFields(
  deckId: string,
  params: { folder_id?: string | null; is_pinned?: boolean },
): Promise<DeckResponse> {
  const { data } = await client.patch<DeckResponse>(`/decks/${deckId}/library`, params);
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
  if (useTauriBackend()) {
    return invokeTauri<ParseDeckResponse>('rust_parse_deck', { deck: deckString });
  }
  const { data } = await client.post<ParseDeckResponse>('/decks/parse', { deck: deckString });
  return data;
}

interface TestedCardsResponse {
  card_ids: string[];
  card_count: number;
}

export async function listTestedCards(): Promise<string[]> {
  if (useTauriBackend()) {
    const resp = await invokeTauri<TestedCardsResponse>('rust_list_tested_cards');
    return resp.card_ids;
  }
  const { data } = await client.get<TestedCardsResponse>('/decks/tested-cards');
  return data.card_ids;
}

/** Shape served by `rust_card_database` (Tauri) and `/decks/card-database`
 *  (hosted API) — `digimon_engine::deck_tools::CardMeta`. */
interface CardMetaDto {
  card_id: string;
  name: string;
  card_type: string;
  colors: string[];
  level: number | null;
  play_cost: number | null;
  evolution_cost: number | null;
  rarity: string;
  dp: number | null;
  stage: string;
  digi_type: string;
  attribute: string;
  main_effect: string;
  inherited_effect: string;
  security_effect: string;
}

function cardMetaToCardData(meta: CardMetaDto): DigimonCardData {
  // The digimoncard.io API folds [Security] text into the main effect;
  // cards.json keeps it separate. Re-fold so both sources render alike.
  const securityLine =
    meta.security_effect && !meta.main_effect.includes(meta.security_effect)
      ? `${meta.main_effect ? '\n' : ''}[Security] ${meta.security_effect}`
      : '';
  return {
    name: meta.name,
    type: meta.card_type,
    color: meta.colors[0] ?? '',
    color2: meta.colors[1],
    stage: meta.stage,
    digi_type: meta.digi_type,
    attribute: meta.attribute,
    level: meta.level != null ? String(meta.level) : '',
    play_cost: meta.play_cost != null ? String(meta.play_cost) : '',
    evolution_cost: meta.evolution_cost != null ? String(meta.evolution_cost) : '',
    cardrarity: meta.rarity,
    artist: '',
    dp: meta.dp != null ? String(meta.dp) : '',
    cardnumber: meta.card_id,
    maineffect: `${meta.main_effect}${securityLine}`,
    soureeffect: meta.inherited_effect,
    set_name: meta.card_id.split('-')[0] ?? '',
    card_sets: [],
    image_url: '',
    isAltArt: false,
  };
}

/** Full metadata for every implemented (allowlisted) card. Local data —
 *  embedded cards.json on desktop, the hosted API's copy in browser —
 *  so the deck builder can browse the whole implemented pool offline. */
export async function listCardDatabase(): Promise<DigimonCardData[]> {
  const dtos = useTauriBackend()
    ? await invokeTauri<CardMetaDto[]>('rust_card_database')
    : (await client.get<CardMetaDto[]>('/decks/card-database')).data;
  return dtos.map(cardMetaToCardData);
}

/** All deck formats from the engine registry. Drives the deck-builder format
 *  selector and the play/format catalog — never hardcode the list. */
export async function listFormats(): Promise<DeckFormat[]> {
  if (useTauriBackend()) {
    return invokeTauri<DeckFormat[]>('rust_list_formats');
  }
  const { data } = await client.get<DeckFormat[]>('/decks/formats');
  return data;
}

/** Per-card legality under a format — powers the pool legality filter and
 *  per-card ban/limit/anomaly badges. */
export async function cardLegality(cardId: string, gameMode = 'standard'): Promise<CardLegality> {
  if (useTauriBackend()) {
    return invokeTauri<CardLegality>('rust_card_legality', { cardId, gameMode });
  }
  const { data } = await client.get<CardLegality & { card_id: string; game_mode: string }>(
    '/decks/card-legality',
    { params: { card_id: cardId, game_mode: gameMode } },
  );
  return { legal: data.legal, max_copies: data.max_copies, reason: data.reason };
}

/** Legality for the whole tested pool under a format, keyed by card id — one
 *  round-trip to drive the pool "format-legal only" filter and badges. */
export async function cardLegalityBulk(
  gameMode = 'standard',
): Promise<Record<string, CardLegality>> {
  if (useTauriBackend()) {
    return invokeTauri<Record<string, CardLegality>>('rust_card_legality_bulk', { gameMode });
  }
  const { data } = await client.get<{ legality: Record<string, CardLegality> }>(
    '/decks/card-legality-bulk',
    { params: { game_mode: gameMode } },
  );
  return data.legality;
}

export async function validateDeckRaw(
  mainDeck: string[],
  eggDeck: string[],
  gameMode?: string,
): Promise<ValidateDeckResponse> {
  const data: BackendValidateDeckResponse = useTauriBackend()
    ? await invokeTauri<BackendValidateDeckResponse>('rust_validate_deck_raw', {
        mainDeck,
        eggDeck,
        gameMode: gameMode ?? 'standard',
      })
    : (
        await client.post<BackendValidateDeckResponse>('/decks/validate', {
          main_deck: mainDeck,
          egg_deck: eggDeck,
          game_mode: gameMode ?? 'standard',
        })
      ).data;

  return {
    valid: data.is_valid,
    errors: data.errors.map((message) => ({ field: 'deck', message })),
    warnings: data.warnings.map((message) => ({ field: 'deck', message })),
  };
}
