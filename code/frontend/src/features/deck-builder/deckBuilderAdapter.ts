import * as deckApi from '@/api/deckApi';
import {
  getDeck as getLibraryDeck,
  listDecks as listLibraryDecks,
} from '@/api/deckLibraryAdapter';
import * as deckStore from '@/storage/deckStore';
import { useAuthStore } from '@/stores/authStore';
import type { DeckResponse, DeckSummary } from '@/types/deck';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

function hasTauriBridge(): boolean {
  return Boolean((globalThis as { isTauri?: boolean }).isTauri);
}

function usesDesktopStorage(): boolean {
  return IS_DESKTOP && hasTauriBridge();
}

export interface SaveBuilderDeckParams {
  deckId?: string | null;
  name: string;
  main_deck: string[];
  egg_deck: string[];
  main_deck_alt_arts?: boolean[];
  egg_deck_alt_arts?: boolean[];
  game_mode?: string;
  commander_id?: string | null;
}

export function listBuilderDecks(): Promise<DeckSummary[]> {
  return listLibraryDecks();
}

export function getBuilderDeck(deckId: string): Promise<DeckResponse> {
  return getLibraryDeck(deckId);
}

export async function saveBuilderDeck(
  params: SaveBuilderDeckParams,
): Promise<DeckResponse> {
  const payload = {
    name: params.name,
    main_deck: params.main_deck,
    egg_deck: params.egg_deck,
    main_deck_alt_arts: params.main_deck_alt_arts,
    egg_deck_alt_arts: params.egg_deck_alt_arts,
    commander_id: params.commander_id ?? null,
    // Thread the selected format through updates too — previously dropped here,
    // so editing a non-standard deck silently reverted its mode.
    ...(params.game_mode ? { game_mode: params.game_mode } : {}),
  };

  if (usesDesktopStorage()) {
    const existing = params.deckId ? await deckStore.getDeck(params.deckId) : null;
    const ownerId = existing?.owner_id ?? useAuthStore.getState().user?.id ?? 'guest';

    return deckStore.putDeck({
      ...existing,
      id: params.deckId ?? existing?.id,
      owner_id: ownerId,
      name: params.name,
      game_mode: params.game_mode ?? existing?.game_mode ?? 'standard',
      main_deck: params.main_deck,
      egg_deck: params.egg_deck,
      main_deck_alt_arts: params.main_deck_alt_arts ?? existing?.main_deck_alt_arts,
      egg_deck_alt_arts: params.egg_deck_alt_arts ?? existing?.egg_deck_alt_arts,
      commander_id: params.commander_id ?? null,
    });
  }

  if (params.deckId) {
    return deckApi.updateDeck(params.deckId, payload);
  }

  return deckApi.createDeck({
    ...payload,
    game_mode: params.game_mode ?? 'standard',
  });
}
