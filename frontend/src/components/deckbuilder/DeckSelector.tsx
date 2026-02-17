import { useEffect } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import * as deckApi from '@/api/deckApi';

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
      loadDeck(
        deck.id,
        deck.name,
        deck.main_deck.map((cardId) => ({ cardId, count: 1 })),
        deck.egg_deck.map((cardId) => ({ cardId, count: 1 })),
      );
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
