import { describe, expect, it, vi } from 'vitest';
import type { DeckResponse, DeckSummary } from '@/types/deck';
import {
  BUILT_IN_STARTER_BOT_DECKS,
  buildBotOpponentDeckPool,
  chooseBotOpponentCandidate,
  chooseBotOpponentDeck,
} from './botOpponentDeckPool';

function summary(overrides: Partial<DeckSummary>): DeckSummary {
  return {
    id: 'deck',
    name: 'Deck',
    description: '',
    game_mode: 'standard',
    is_valid: true,
    is_public: false,
    is_pinned: false,
    folder_id: null,
    card_count: 54,
    main_count: 50,
    egg_count: 4,
    tags: [],
    meta_tier: null,
    meta_archetype: null,
    colors: [],
    highest_level: null,
    created_at: '2026-06-15T00:00:00.000Z',
    updated_at: '2026-06-15T00:00:00.000Z',
    ...overrides,
  };
}

function response(overrides: Partial<DeckResponse>): DeckResponse {
  return {
    id: 'deck',
    owner_id: 'guest',
    folder_id: null,
    name: 'Deck',
    description: '',
    game_mode: 'eden',
    main_deck: Array(50).fill('BT1-001'),
    egg_deck: Array(4).fill('BT1-002'),
    main_deck_alt_arts: [],
    egg_deck_alt_arts: [],
    commander_id: null,
    is_valid: true,
    validation_errors: [],
    is_public: false,
    is_pinned: false,
    tags: [],
    meta_tier: null,
    meta_archetype: null,
    created_at: '2026-06-15T00:00:00.000Z',
    updated_at: '2026-06-15T00:00:00.000Z',
    ...overrides,
  };
}

describe('built-in bot starter decks', () => {
  it('contains exactly one concrete ST1 through ST6 starter deck', () => {
    expect(BUILT_IN_STARTER_BOT_DECKS.map((deck) => deck.id)).toEqual([
      'starter-st1',
      'starter-st2',
      'starter-st3',
      'starter-st4',
      'starter-st5',
      'starter-st6',
    ]);
    expect(new Set(BUILT_IN_STARTER_BOT_DECKS.map((deck) => deck.id)).size).toBe(6);
    for (const deck of BUILT_IN_STARTER_BOT_DECKS) {
      expect(deck.egg_deck).toHaveLength(4);
      expect(deck.main_deck).toHaveLength(50);
    }
  });
});

describe('buildBotOpponentDeckPool', () => {
  it('keeps all six starter decks eligible in every selected format', () => {
    const pool = buildBotOpponentDeckPool([], 'missing', 'eden_singleton');

    expect(pool.filter((candidate) => candidate.source === 'starter')).toHaveLength(6);
  });

  it('adds only valid saved decks for the selected format and excludes the selected deck', () => {
    const pool = buildBotOpponentDeckPool(
      [
        summary({ id: 'selected', name: 'Selected EDEN', game_mode: 'eden' }),
        summary({ id: 'eden-other', name: 'Other EDEN', game_mode: 'eden' }),
        summary({ id: 'standard-other', name: 'Other Standard', game_mode: 'standard' }),
        summary({ id: 'invalid-eden', name: 'Invalid EDEN', game_mode: 'eden', is_valid: false }),
      ],
      'selected',
      'eden',
    );

    expect(pool.filter((candidate) => candidate.source === 'starter')).toHaveLength(6);
    expect(pool.filter((candidate) => candidate.source === 'saved').map((candidate) => candidate.id))
      .toEqual(['eden-other']);
  });
});

describe('chooseBotOpponentCandidate', () => {
  it('uses the injected random value to choose a launch-local candidate', () => {
    const pool = buildBotOpponentDeckPool(
      [summary({ id: 'saved-1', name: 'Saved One', game_mode: 'standard' })],
      'selected',
      'standard',
    );

    expect(chooseBotOpponentCandidate(pool, () => 0).id).toBe('starter-st1');
    expect(chooseBotOpponentCandidate(pool, () => 0.999).id).toBe('saved-1');
  });
});

describe('chooseBotOpponentDeck', () => {
  it('returns a starter deck without loading saved decks when the chosen candidate is built in', async () => {
    const getDeck = vi.fn();

    const deck = await chooseBotOpponentDeck({
      savedDecks: [],
      selectedDeckId: 'selected',
      formatId: 'eden',
      getDeck,
      random: () => 0,
    });

    expect(deck.id).toBe('starter-st1');
    expect(getDeck).not.toHaveBeenCalled();
  });

  it('loads and returns a saved deck when random selection picks a saved candidate', async () => {
    const saved = response({ id: 'saved-1', name: 'Saved One' });
    const getDeck = vi.fn(async () => saved);

    const deck = await chooseBotOpponentDeck({
      savedDecks: [summary({ id: 'saved-1', name: 'Saved One', game_mode: 'standard' })],
      selectedDeckId: 'selected',
      formatId: 'standard',
      getDeck,
      random: () => 0.999,
    });

    expect(getDeck).toHaveBeenCalledWith('saved-1');
    expect(deck).toBe(saved);
  });
});
