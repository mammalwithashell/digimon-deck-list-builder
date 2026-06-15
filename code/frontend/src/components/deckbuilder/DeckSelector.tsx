import { useEffect } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { getCardById } from '@/api/digimonCardApi';
import * as deckApi from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
import type { DeckEntry } from '@/types/deck';

// Keep the deck selector's source in lockstep with DeckBuilderPage's
// save path — desktop reads/writes the local store, web hits the API.
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
const decks = IS_DESKTOP ? deckStore : deckApi;

/** Group a flat array of card IDs + parallel alt-art bool array into
 *  DeckEntry objects, keeping base and alt printings as separate entries. */
function groupCardIds(ids: string[], altArts: boolean[] = []): DeckEntry[] {
  const counts = new Map<string, { cardId: string; isAltArt: boolean; count: number }>();
  ids.forEach((cardId, i) => {
    const isAltArt = !!altArts[i];
    const key = `${cardId}|${isAltArt ? '1' : '0'}`;
    const existing = counts.get(key);
    if (existing) {
      existing.count += 1;
    } else {
      counts.set(key, { cardId, isAltArt, count: 1 });
    }
  });
  return Array.from(counts.values());
}

export function DeckSelector() {
  const { savedDecks, setSavedDecks, loadDeck, deckId, clearDeck } = useDeckBuilderStore();

  useEffect(() => {
    decks.listDecks().then(setSavedDecks).catch(() => {});
  }, [setSavedDecks]);

  const handleSelect = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value;
    if (val === 'new') {
      clearDeck();
      return;
    }
    try {
      const deck = await decks.getDeck(val);

      // Group duplicate IDs into entries with correct counts, preserving
      // which slots were saved as alt-art printings.
      const mainEntries = groupCardIds(deck.main_deck, deck.main_deck_alt_arts);
      const eggEntries = groupCardIds(deck.egg_deck, deck.egg_deck_alt_arts);

      // Fetch card data for display (names, images)
      const allIds = [...new Set([...deck.main_deck, ...deck.egg_deck])];
      const cardDataMap = new Map<string, Awaited<ReturnType<typeof getCardById>>>();
      await Promise.allSettled(
        allIds.map(async (id) => {
          const data = await getCardById(id);
          if (data) cardDataMap.set(id, data);
        }),
      );

      // Attach card data to entries
      for (const entry of [...mainEntries, ...eggEntries]) {
        const data = cardDataMap.get(entry.cardId);
        if (data) entry.cardData = data;
      }

      loadDeck(deck.id, deck.name, mainEntries, eggEntries, deck.game_mode, deck.commander_id);
    } catch {
      // Ignore
    }
  };

  return (
    <select
      value={deckId ?? 'new'}
      onChange={handleSelect}
      className="px-2 py-1 bg-gray-700 border border-gray-600 rounded text-sm text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      <option value="new">New Deck</option>
      {savedDecks.map((d) => (
        <option key={d.id} value={d.id}>
          {d.name}
        </option>
      ))}
    </select>
  );
}
