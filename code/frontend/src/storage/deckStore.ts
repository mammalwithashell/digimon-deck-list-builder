// Client-side deck storage for desktop builds. Backed by Tauri commands
// in `src-tauri/src/deck_storage.rs`. The shapes match `deckApi.ts` so
// callers can swap behind a single import.
//
// Browser-dev fallback: when this same bundle runs in a plain browser
// (no Tauri runtime), the read operations fall back to the FastAPI
// `/desktop-decks` router which reads from the same local file
// location Tauri uses. Write operations stay desktop-only — browser
// is a test harness, not a deckbuilder.

import { invoke } from '@tauri-apps/api/core';

import { httpJson, isInTauriRuntime } from '@/api/engineRuntime';
import type { DeckFolder, DeckResponse, DeckSummary } from '@/types/deck';

export type Deck = DeckResponse;

// Re-export the shared DeckSummary so callers can treat this module and
// `deckApi.ts` as swap-compatible.
export type { DeckSummary } from '@/types/deck';

export async function listDecks(): Promise<DeckSummary[]> {
  if (!isInTauriRuntime()) {
    return httpJson<DeckSummary[]>('/desktop-decks');
  }
  return invoke<DeckSummary[]>('decks_list');
}

export async function listDeckFolders(): Promise<DeckFolder[]> {
  if (!isInTauriRuntime()) {
    return httpJson<DeckFolder[]>('/desktop-decks/folders');
  }
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
  if (!isInTauriRuntime()) {
    return httpJson<Deck>(`/desktop-decks/${encodeURIComponent(deckId)}`);
  }
  return invoke<Deck>('decks_get', { deckId });
}

export async function putDeck(deck: Partial<Deck> & {
  name: string;
  game_mode: string;
  main_deck: string[];
  egg_deck: string[];
}): Promise<Deck> {
  // Re-run validation against the engine on every save so the `is_valid`
  // flag and `validation_errors` always reflect the current card lists.
  // Without this, a deck saved while incomplete keeps `is_valid: false`
  // forever — even after the user adds the missing eggs — because the
  // deckbuilder never explicitly flips the flag back. The DeckSelectPage's
  // `canUseDeckForFormat` reads `is_valid` directly, so a stale-false flag
  // blocks "USE THIS DECK" until the user fiddles with the file by hand.
  let isValid = deck.is_valid ?? false;
  let validationErrors = deck.validation_errors ?? [];
  try {
    const result = await invoke<{
      is_valid: boolean;
      errors: string[];
      warnings: string[];
    }>('rust_validate_deck_raw', {
      mainDeck: deck.main_deck,
      eggDeck: deck.egg_deck,
      gameMode: deck.game_mode,
    });
    isValid = result.is_valid;
    validationErrors = result.errors;
  } catch (err) {
    // Validation backend failed — leave the caller-supplied flag intact
    // rather than blocking the save. Log so the developer notices.
    console.warn('putDeck: validation backend errored; using caller flag', err);
  }

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
    is_valid: isValid,
    validation_errors: validationErrors,
    is_public: deck.is_public ?? false,
    is_pinned: deck.is_pinned ?? false,
    tags: deck.tags ?? [],
    meta_tier: deck.meta_tier ?? null,
    meta_archetype: deck.meta_archetype ?? null,
    created_at: deck.created_at ?? now,
    updated_at: now,
  };
  if (!isInTauriRuntime()) {
    return httpJson<Deck>('/desktop-decks', { method: 'POST', body: full });
  }
  return invoke<Deck>('decks_put', { deck: full });
}

export async function deleteDeck(deckId: string): Promise<boolean> {
  if (!isInTauriRuntime()) {
    return httpJson<boolean>(`/desktop-decks/${encodeURIComponent(deckId)}`, { method: 'DELETE' });
  }
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
