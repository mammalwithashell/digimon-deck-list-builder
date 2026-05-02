import * as deckApi from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
import type { DeckFolder, DeckResponse, DeckSummary } from '@/types/deck';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

function hasTauriBridge(): boolean {
  return Boolean((globalThis as { isTauri?: boolean }).isTauri);
}

function usesDesktopStorage(): boolean {
  return IS_DESKTOP && hasTauriBridge();
}

function deckBackend() {
  return usesDesktopStorage() ? deckStore : deckApi;
}

export function listDecks(): Promise<DeckSummary[]> {
  return deckBackend().listDecks();
}

export function listDeckFolders(): Promise<DeckFolder[]> {
  return deckBackend().listDeckFolders();
}

export function createDeckFolder(name: string): Promise<DeckFolder> {
  return deckBackend().createDeckFolder(name);
}

export function updateDeckFolder(
  folderId: string,
  params: { name?: string; sort_order?: number },
): Promise<DeckFolder> {
  return deckBackend().updateDeckFolder(folderId, params);
}

export async function deleteDeckFolder(folderId: string): Promise<void> {
  await deckBackend().deleteDeckFolder(folderId);
}

export function getDeck(deckId: string): Promise<DeckResponse> {
  return deckBackend().getDeck(deckId);
}

export function updateDeck(
  deckId: string,
  params: Parameters<typeof deckApi.updateDeck>[1],
): Promise<DeckResponse> {
  if (usesDesktopStorage()) {
    return deckStore.getDeck(deckId).then((existing) =>
      deckStore.putDeck({
        ...existing,
        ...params,
        main_deck: params.main_deck ?? existing.main_deck,
        egg_deck: params.egg_deck ?? existing.egg_deck,
        name: params.name ?? existing.name,
        game_mode: existing.game_mode,
      }),
    );
  }
  return deckApi.updateDeck(deckId, params);
}

export function updateDeckLibraryFields(
  deckId: string,
  params: { folder_id?: string | null; is_pinned?: boolean },
): Promise<DeckResponse> {
  return deckBackend().updateDeckLibraryFields(deckId, params);
}

export async function duplicateDeck(deckId: string): Promise<DeckResponse> {
  const source = await getDeck(deckId);
  const name = `${source.name} Copy`;
  if (usesDesktopStorage()) {
    return deckStore.putDeck({
      ...source,
      id: '',
      name,
      is_pinned: false,
      created_at: '',
      updated_at: '',
    });
  }
  const created = await deckApi.createDeck({
    name,
    description: source.description,
    game_mode: source.game_mode,
    main_deck: source.main_deck,
    egg_deck: source.egg_deck,
    main_deck_alt_arts: source.main_deck_alt_arts,
    egg_deck_alt_arts: source.egg_deck_alt_arts,
    is_public: source.is_public,
    tags: source.tags,
  });
  if (source.folder_id) {
    return deckApi.updateDeckLibraryFields(created.id, {
      folder_id: source.folder_id,
      is_pinned: false,
    });
  }
  return created;
}

export async function deleteDeck(deckId: string): Promise<void> {
  await deckBackend().deleteDeck(deckId);
}
