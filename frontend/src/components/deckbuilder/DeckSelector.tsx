import { useEffect } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { getCardById } from '@/api/digimonCardApi';
import * as deckApi from '@/api/deckApi';

/** Group a flat array of card IDs into DeckEntry objects with counts. */
function groupCardIds(ids: string[]) {
  const counts = new Map<string, number>();
  for (const id of ids) {
    counts.set(id, (counts.get(id) ?? 0) + 1);
  }
  return Array.from(counts.entries()).map(([cardId, count]) => ({ cardId, count }));
}

export function DeckSelector() {
  const { savedDecks, setSavedDecks, loadDeck, deckId, clearDeck } = useDeckBuilderStore();

  useEffect(() => {
    deckApi.listDecks().then(setSavedDecks).catch(() => {});
  }, [setSavedDecks]);

  const handleSelect = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value;
    if (val === 'new') {
      clearDeck();
      return;
    }
    try {
      const deck = await deckApi.getDeck(val);

      // Group duplicate IDs into entries with correct counts
      const mainEntries = groupCardIds(deck.main_deck);
      const eggEntries = groupCardIds(deck.egg_deck);

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
        if (data) (entry as any).cardData = data;
      }

      loadDeck(deck.id, deck.name, mainEntries, eggEntries);
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
