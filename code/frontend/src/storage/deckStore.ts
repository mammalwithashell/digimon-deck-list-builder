// Client-side deck storage for desktop builds. Backed by Tauri commands
// in `src-tauri/src/deck_storage.rs`. The shapes match `deckApi.ts` so
// callers can swap behind a single import.

import { invoke } from '@tauri-apps/api/core';

import type { DeckFolder, DeckResponse, DeckSummary } from '@/types/deck';

export type Deck = DeckResponse;

// Re-export the shared DeckSummary so callers can treat this module and
// `deckApi.ts` as swap-compatible.
export type { DeckSummary } from '@/types/deck';

export async function listDecks(): Promise<DeckSummary[]> {
  return invoke<DeckSummary[]>('decks_list');
}

export async function listDeckFolders(): Promise<DeckFolder[]> {
  return invoke<DeckFolder[]>('deck_folders_list');
}

export async function createDeckFolder(name: string): Promise<DeckFolder> {
  return invoke<DeckFolder>('deck_folders_create', { name });
}

export async function updateDeckFolder(
  folderId: string,
  params: { name?: string; sort_order?: number },
): Promise<DeckFolder> {
  return invoke<DeckFolder>('deck_folders_update', {
    folderId,
    name: params.name,
    sortOrder: params.sort_order,
  });
}

export async function deleteDeckFolder(folderId: string): Promise<boolean> {
  return invoke<boolean>('deck_folders_delete', { folderId });
}

export async function getDeck(deckId: string): Promise<Deck> {
  return invoke<Deck>('decks_get', { deckId });
}

export async function putDeck(deck: Partial<Deck> & {
  name: string;
  game_mode: string;
  main_deck: string[];
  egg_deck: string[];
}): Promise<Deck> {
  // Fill required fields so the desktop deck struct deserializes cleanly.
  const now = new Date().toISOString();
  const full: Deck = {
    id: deck.id ?? '',
    folder_id: deck.folder_id ?? null,
    // TODO(task-8): once bootstrap/guest.ts lands, stamp the actual guest
    // user_id from localStorage instead of the 'guest' placeholder.
    owner_id: deck.owner_id ?? 'guest',
    name: deck.name,
    description: deck.description ?? '',
    game_mode: deck.game_mode,
    main_deck: deck.main_deck,
    egg_deck: deck.egg_deck,
    main_deck_alt_arts: deck.main_deck_alt_arts ?? [],
    egg_deck_alt_arts: deck.egg_deck_alt_arts ?? [],
    commander_id: deck.commander_id ?? null,
    is_valid: deck.is_valid ?? false,
    validation_errors: deck.validation_errors ?? [],
    is_public: deck.is_public ?? false,
    is_pinned: deck.is_pinned ?? false,
    tags: deck.tags ?? [],
    meta_tier: deck.meta_tier ?? null,
    meta_archetype: deck.meta_archetype ?? null,
    created_at: deck.created_at ?? now,
    updated_at: now,
  };
  return invoke<Deck>('decks_put', { deck: full });
}

export async function deleteDeck(deckId: string): Promise<boolean> {
  return invoke<boolean>('decks_delete', { deckId });
}

export async function updateDeckLibraryFields(
  deckId: string,
  params: { folder_id?: string | null; is_pinned?: boolean },
): Promise<Deck> {
  return invoke<Deck>('decks_update_library', {
    deckId,
    folderId: params.folder_id ?? undefined,
    clearFolder: params.folder_id === null,
    isPinned: params.is_pinned,
  });
}
