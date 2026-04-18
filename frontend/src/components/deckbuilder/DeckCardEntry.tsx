import { Card } from '@/components/shared/Card';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import type { DeckEntry } from '@/types/deck';

interface DeckCardEntryProps {
  entry: DeckEntry;
}

export function DeckCardEntry({ entry }: DeckCardEntryProps) {
  const { addCardToDeck, removeCardFromDeck, setSelectedCardId } = useDeckBuilderStore();

  return (
    <div
      className="flex items-center gap-2 py-1 px-1 rounded hover:bg-gray-700/50 group"
      onMouseEnter={() => setSelectedCardId(entry.cardId)}
    >
      <Card
        cardId={entry.cardId}
        cardName={entry.cardData?.name}
        size="sm"
        isAltArt={entry.isAltArt}
      />
      <div className="flex-1 min-w-0">
        <div className="text-xs text-gray-200 truncate">
          {entry.cardData?.name ?? entry.cardId}
          {entry.isAltArt && (
            <span className="ml-1 text-[9px] font-bold text-purple-300">(Alt)</span>
          )}
        </div>
        <div className="text-[10px] text-gray-500">{entry.cardId}</div>
      </div>
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          onClick={() => removeCardFromDeck(entry.cardId, entry.isAltArt)}
          className="w-5 h-5 flex items-center justify-center bg-red-700/60 hover:bg-red-600 rounded text-xs text-white"
        >
          -
        </button>
        <span className="text-xs text-gray-300 w-4 text-center">{entry.count}</span>
        <button
          onClick={() => {
            if (entry.cardData) addCardToDeck(entry.cardId, entry.cardData, entry.isAltArt);
          }}
          className="w-5 h-5 flex items-center justify-center bg-green-700/60 hover:bg-green-600 rounded text-xs text-white"
        >
          +
        </button>
      </div>
      <span className="text-xs text-gray-400 w-4 text-right group-hover:hidden">
        x{entry.count}
      </span>
    </div>
  );
}
