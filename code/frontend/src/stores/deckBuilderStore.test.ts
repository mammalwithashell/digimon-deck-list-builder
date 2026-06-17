import { beforeEach, describe, expect, it } from 'vitest';
import type { DigimonCardData } from '@/types/cards';
import type { DeckEntry } from '@/types/deck';
import { useDeckBuilderStore } from './deckBuilderStore';

function card(overrides: Partial<DigimonCardData> = {}): DigimonCardData {
  return {
    name: 'Card',
    type: 'Digimon',
    color: 'Blue',
    stage: '',
    digi_type: '',
    attribute: '',
    level: '3',
    play_cost: '',
    evolution_cost: '',
    cardrarity: 'C',
    artist: '',
    dp: '',
    cardnumber: 'BT1-001',
    maineffect: '',
    soureeffect: '',
    set_name: '',
    card_sets: [],
    image_url: '',
    ...overrides,
  };
}

function entry(cardData: DigimonCardData, count: number): DeckEntry {
  return {
    cardId: cardData.cardnumber,
    isAltArt: false,
    count,
    cardData,
  };
}

describe('deck builder store deck icon state', () => {
  beforeEach(() => {
    useDeckBuilderStore.setState({
      deckId: null,
      deckName: 'New Deck',
      gameMode: 'standard',
      mainDeck: [],
      eggDeck: [],
      deckIconCardId: null,
      isDirty: false,
      validationResult: null,
      testedCardIds: null,
    });
  });

  it('loads and updates the explicit deck icon card id', () => {
    const rookie = card({ cardnumber: 'BT1-002', name: 'Veemon' });
    const store = useDeckBuilderStore.getState();

    store.loadDeck('deck-1', 'Blue Deck', [entry(rookie, 4)], [], 'standard', rookie.cardnumber);
    expect(useDeckBuilderStore.getState().deckIconCardId).toBe(rookie.cardnumber);

    useDeckBuilderStore.getState().setDeckIconCardId(null);
    expect(useDeckBuilderStore.getState().deckIconCardId).toBeNull();
    expect(useDeckBuilderStore.getState().isDirty).toBe(true);
  });

  it('clears the explicit icon when the final matching card leaves the deck', () => {
    const rookie = card({ cardnumber: 'BT1-002', name: 'Veemon' });
    const store = useDeckBuilderStore.getState();

    store.loadDeck('deck-1', 'Blue Deck', [entry(rookie, 1)], [], 'standard', rookie.cardnumber);
    useDeckBuilderStore.getState().removeCardFromDeck(rookie.cardnumber);

    expect(useDeckBuilderStore.getState().mainDeck).toEqual([]);
    expect(useDeckBuilderStore.getState().deckIconCardId).toBeNull();
  });
});
