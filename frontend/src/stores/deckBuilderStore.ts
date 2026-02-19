import { create } from 'zustand';
import type { DigimonCardData, CardFilters } from '@/types/cards';
import type { DeckEntry, DeckSummary, DeckValidationResult } from '@/types/deck';
import { DEFAULT_FILTERS } from '@/types/cards';

interface DeckBuilderStore {
  // Search
  searchQuery: string;
  filters: CardFilters;
  searchResults: DigimonCardData[];
  searchPage: number;
  isSearching: boolean;
  selectedCardId: string | null;

  // Current deck
  deckId: string | null;
  deckName: string;
  mainDeck: DeckEntry[];
  eggDeck: DeckEntry[];
  isDirty: boolean;
  validationResult: DeckValidationResult | null;

  // Saved decks
  savedDecks: DeckSummary[];

  // Actions
  setSearchQuery: (q: string) => void;
  setFilters: (filters: Partial<CardFilters>) => void;
  resetFilters: () => void;
  setSearchResults: (results: DigimonCardData[]) => void;
  setSearchPage: (page: number) => void;
  setIsSearching: (loading: boolean) => void;
  setSelectedCardId: (id: string | null) => void;

  addCardToDeck: (cardId: string, cardData: DigimonCardData) => void;
  removeCardFromDeck: (cardId: string) => void;
  setDeckName: (name: string) => void;
  setDeckId: (id: string | null) => void;
  loadDeck: (id: string | null, name: string, main: DeckEntry[], egg: DeckEntry[]) => void;
  clearDeck: () => void;
  setValidationResult: (result: DeckValidationResult | null) => void;
  setSavedDecks: (decks: DeckSummary[]) => void;
  setIsDirty: (dirty: boolean) => void;
}

export const useDeckBuilderStore = create<DeckBuilderStore>((set) => ({
  searchQuery: '',
  filters: { ...DEFAULT_FILTERS },
  searchResults: [],
  searchPage: 0,
  isSearching: false,
  selectedCardId: null,

  deckId: null,
  deckName: 'New Deck',
  mainDeck: [],
  eggDeck: [],
  isDirty: false,
  validationResult: null,

  savedDecks: [],

  setSearchQuery: (q) => set({ searchQuery: q, searchPage: 0 }),
  setFilters: (filters) =>
    set((s) => ({ filters: { ...s.filters, ...filters }, searchPage: 0 })),
  resetFilters: () => set({ filters: { ...DEFAULT_FILTERS }, searchQuery: '', searchPage: 0 }),
  setSearchResults: (results) => set({ searchResults: results }),
  setSearchPage: (page) => set({ searchPage: page }),
  setIsSearching: (loading) => set({ isSearching: loading }),
  setSelectedCardId: (id) => set({ selectedCardId: id }),

  addCardToDeck: (cardId, cardData) =>
    set((s) => {
      const isEgg = cardData.type === 'Digi-Egg';
      const deck = isEgg ? [...s.eggDeck] : [...s.mainDeck];
      const existing = deck.find((e) => e.cardId === cardId);
      if (existing) {
        if (existing.count >= 4) return s;
        existing.count += 1;
      } else {
        deck.push({ cardId, count: 1, cardData });
      }
      return isEgg
        ? { eggDeck: deck, isDirty: true }
        : { mainDeck: deck, isDirty: true };
    }),

  removeCardFromDeck: (cardId) =>
    set((s) => {
      let mainDeck = [...s.mainDeck];
      let eggDeck = [...s.eggDeck];
      const mainIdx = mainDeck.findIndex((e) => e.cardId === cardId);
      if (mainIdx !== -1) {
        const entry = mainDeck[mainIdx]!;
        if (entry.count > 1) {
          entry.count -= 1;
        } else {
          mainDeck.splice(mainIdx, 1);
        }
      } else {
        const eggIdx = eggDeck.findIndex((e) => e.cardId === cardId);
        if (eggIdx !== -1) {
          const entry = eggDeck[eggIdx]!;
          if (entry.count > 1) {
            entry.count -= 1;
          } else {
            eggDeck.splice(eggIdx, 1);
          }
        }
      }
      return { mainDeck, eggDeck, isDirty: true };
    }),

  setDeckName: (name) => set({ deckName: name, isDirty: true }),
  setDeckId: (id) => set({ deckId: id }),
  loadDeck: (id, name, main, egg) =>
    set({ deckId: id, deckName: name, mainDeck: main, eggDeck: egg, isDirty: false }),
  clearDeck: () =>
    set({ deckId: null, deckName: 'New Deck', mainDeck: [], eggDeck: [], isDirty: false, validationResult: null }),
  setValidationResult: (result) => set({ validationResult: result }),
  setSavedDecks: (decks) => set({ savedDecks: decks }),
  setIsDirty: (dirty) => set({ isDirty: dirty }),
}));
