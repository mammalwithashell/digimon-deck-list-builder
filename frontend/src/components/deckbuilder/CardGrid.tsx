import { useCallback } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { Card } from '@/components/shared/Card';
import { CARDS_PER_PAGE } from '@/utils/constants';
import type { DigimonCardData } from '@/types/cards';

export function CardGrid() {
  const {
    searchResults,
    searchPage,
    setSearchPage,
    setSelectedCardId,
    addCardToDeck,
    mainDeck,
    eggDeck,
    isSearching,
    testedCardIds,
  } = useDeckBuilderStore();

  const start = searchPage * CARDS_PER_PAGE;
  const pageResults = searchResults.slice(start, start + CARDS_PER_PAGE);
  const totalPages = Math.ceil(searchResults.length / CARDS_PER_PAGE);

  const getCountInDeck = useCallback(
    (cardId: string) => {
      // Sum across both art variants — the 4-per-card limit is shared.
      const main = mainDeck
        .filter((e) => e.cardId === cardId)
        .reduce((sum, e) => sum + e.count, 0);
      const egg = eggDeck
        .filter((e) => e.cardId === cardId)
        .reduce((sum, e) => sum + e.count, 0);
      return main + egg;
    },
    [mainDeck, eggDeck],
  );

  const handleDoubleClick = (card: DigimonCardData) => {
    addCardToDeck(card.cardnumber, card, card.isAltArt ?? false);
  };

  if (isSearching) {
    return (
      <div className="flex items-center justify-center h-48 text-gray-500">
        Searching...
      </div>
    );
  }

  if (searchResults.length === 0) {
    const message =
      testedCardIds === null
        ? 'Loading alpha card pool...'
        : 'No tested cards match this search (alpha release).';
    return (
      <div className="flex items-center justify-center h-48 text-gray-500 text-center px-4">
        {message}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(80px,1fr))] gap-2">
        {pageResults.map((card) => {
          const count = getCountInDeck(card.cardnumber);
          const variantKey = `${card.cardnumber}${card.isAltArt ? '-alt' : ''}`;
          return (
            <div key={variantKey} className="relative">
              <Card
                cardId={card.cardnumber}
                cardName={card.name}
                size="md"
                isAltArt={card.isAltArt}
                onClick={() => setSelectedCardId(card.cardnumber)}
                onMouseEnter={() => setSelectedCardId(card.cardnumber)}
              />
              {count > 0 && (
                <div className="absolute -top-1 -right-1 bg-blue-600 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center">
                  {count}
                </div>
              )}
              <button
                className="absolute bottom-0 right-0 bg-green-600/80 hover:bg-green-500 text-white text-[10px] rounded-tl px-1"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDoubleClick(card);
                }}
              >
                +
              </button>
            </div>
          );
        })}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-2 py-2">
          <button
            disabled={searchPage === 0}
            onClick={() => setSearchPage(searchPage - 1)}
            className="text-sm px-2 py-1 bg-gray-700 rounded disabled:opacity-30 hover:bg-gray-600"
          >
            Prev
          </button>
          <span className="text-xs text-gray-400">
            {searchPage + 1} / {totalPages} ({searchResults.length} cards)
          </span>
          <button
            disabled={searchPage >= totalPages - 1}
            onClick={() => setSearchPage(searchPage + 1)}
            className="text-sm px-2 py-1 bg-gray-700 rounded disabled:opacity-30 hover:bg-gray-600"
          >
            Next
          </button>
        </div>
      )}
    </div>
  );
}
