import { describe, expect, it, vi } from 'vitest';
import type { DigimonCardData } from '@/types/cards';
import type { DeckResponse, DeckSummary } from '@/types/deck';
import {
  buildDeckAnalytics,
  filterAndSortDecks,
  formatRelativeTime,
} from './deckLibrary';

function summary(overrides: Partial<DeckSummary>): DeckSummary {
  return {
    id: overrides.id ?? 'deck-1',
    name: overrides.name ?? 'Alpha Deck',
    description: overrides.description ?? '',
    game_mode: 'standard',
    is_valid: overrides.is_valid ?? true,
    is_public: false,
    is_pinned: overrides.is_pinned ?? false,
    folder_id: overrides.folder_id ?? null,
    card_count: overrides.card_count ?? 54,
    main_count: overrides.main_count ?? 50,
    egg_count: overrides.egg_count ?? 4,
    tags: overrides.tags ?? [],
    meta_tier: overrides.meta_tier ?? null,
    meta_archetype: overrides.meta_archetype ?? null,
    colors: [],
    highest_level: null,
    created_at: overrides.created_at ?? '2026-04-20T00:00:00Z',
    updated_at: overrides.updated_at ?? '2026-04-20T00:00:00Z',
  };
}

function card(overrides: Partial<DigimonCardData>): DigimonCardData {
  return {
    name: 'Card',
    type: 'Digimon',
    color: 'Blue',
    stage: '',
    digi_type: '',
    attribute: '',
    level: '3',
    play_cost: '3',
    evolution_cost: '',
    cardrarity: '',
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

describe('deck library helpers', () => {
  it('filters pinned decks and searches archetypes and tags', () => {
    const decks = [
      summary({ id: 'a', name: 'Blue Flare', is_pinned: true, meta_archetype: 'Hybrid Aggro' }),
      summary({ id: 'b', name: 'Green Bugs', tags: ['locals'], folder_id: 'f1' }),
    ];

    expect(filterAndSortDecks(decks, { activeFolder: 'pinned', search: 'hybrid', sort: 'name' }))
      .toHaveLength(1);
    expect(filterAndSortDecks(decks, { activeFolder: 'f1', search: 'locals', sort: 'name' })[0]?.id)
      .toBe('b');
  });

  it('sorts recent decks by updated date descending', () => {
    const decks = [
      summary({ id: 'old', updated_at: '2026-04-20T00:00:00Z' }),
      summary({ id: 'new', updated_at: '2026-04-22T00:00:00Z' }),
    ];

    expect(filterAndSortDecks(decks, { activeFolder: 'all', search: '', sort: 'recent' })[0]?.id)
      .toBe('new');
  });

  it('formats relative time without negative output', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-26T10:00:00Z'));
    expect(formatRelativeTime('2026-04-26T09:45:00Z')).toBe('15m');
    expect(formatRelativeTime('2026-04-27T09:45:00Z')).toBe('just now');
    vi.useRealTimers();
  });

  it('derives colors, level curve, average cost, and highest level from card data', () => {
    const deck: DeckResponse = {
      id: 'd1',
      owner_id: 'u1',
      folder_id: null,
      name: 'Test',
      description: '',
      game_mode: 'standard',
      main_deck: ['BT1-001', 'BT1-002', 'BT1-002'],
      egg_deck: ['BT1-003'],
      main_deck_alt_arts: [],
      egg_deck_alt_arts: [],
      commander_id: null,
      is_valid: true,
      validation_errors: [],
      is_public: false,
      is_pinned: false,
      tags: [],
      created_at: '2026-04-26T00:00:00Z',
      updated_at: '2026-04-26T00:00:00Z',
    };
    const cards = new Map([
      ['BT1-001', card({ color: 'Blue', level: '3', play_cost: '3' })],
      ['BT1-002', card({ color: 'Red', level: '6', play_cost: '11' })],
      ['BT1-003', card({ color: 'Yellow', level: '2', play_cost: '0' })],
    ]);

    const analytics = buildDeckAnalytics(deck, cards);

    expect(analytics.colors.map((c) => c.name)).toContain('Red');
    expect(analytics.levelCurve[6]).toBe(2);
    expect(analytics.averagePlayCost).toBe('6.3');
    expect(analytics.highestLevel).toBe(6);
  });
});
