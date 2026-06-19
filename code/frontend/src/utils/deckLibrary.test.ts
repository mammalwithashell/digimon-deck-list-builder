import { describe, expect, it, vi } from 'vitest';
import type { DigimonCardData } from '@/types/cards';
import type { DeckFormat, DeckResponse, DeckSummary } from '@/types/deck';
import {
  buildDeckAnalytics,
  countByFormat,
  deriveFormatBuckets,
  filterAndSortDecks,
  formatLabel,
  formatRelativeTime,
} from './deckLibrary';

function summary(overrides: Partial<DeckSummary>): DeckSummary {
  return {
    id: overrides.id ?? 'deck-1',
    name: overrides.name ?? 'Alpha Deck',
    description: overrides.description ?? '',
    game_mode: overrides.game_mode ?? 'standard',
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

function fmt(id: string, name: string): DeckFormat {
  return {
    id,
    name,
    description: '',
    deck_size: 50,
    egg_max: 5,
    rarity_policy: 'all',
    singleton: false,
    default_max_copies: 4,
    playable: true,
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

describe('deck library format filtering', () => {
  const decks = [
    summary({ id: 's1', game_mode: 'standard' }),
    summary({ id: 's2', game_mode: 'standard' }),
    summary({ id: 'e1', game_mode: 'eden' }),
    summary({ id: 't1', game_mode: 'titan' }), // legacy / not in registry
  ];
  const formats = [fmt('standard', 'STANDARD'), fmt('eden', 'EDEN')];

  it('narrows decks to the selected format', () => {
    const out = filterAndSortDecks(decks, { activeFolder: 'all', search: '', sort: 'name', format: 'eden' });
    expect(out.map((d) => d.id)).toEqual(['e1']);
  });

  it("treats 'all' and undefined format as a no-op", () => {
    const all = filterAndSortDecks(decks, { activeFolder: 'all', search: '', sort: 'name', format: 'all' });
    const undef = filterAndSortDecks(decks, { activeFolder: 'all', search: '', sort: 'name' });
    expect(all).toHaveLength(4);
    expect(undef).toHaveLength(4);
  });

  it('composes the format filter with folder and search', () => {
    const scoped = [
      summary({ id: 'a', game_mode: 'standard', folder_id: 'f1', name: 'Red Hybrid' }),
      summary({ id: 'b', game_mode: 'eden', folder_id: 'f1', name: 'Red Hybrid' }),
      summary({ id: 'c', game_mode: 'standard', folder_id: 'f2', name: 'Red Hybrid' }),
    ];
    const out = filterAndSortDecks(scoped, {
      activeFolder: 'f1',
      search: 'hybrid',
      sort: 'name',
      format: 'standard',
    });
    expect(out.map((d) => d.id)).toEqual(['a']);
  });

  it('matches game_mode in search', () => {
    const out = filterAndSortDecks(decks, { activeFolder: 'all', search: 'eden', sort: 'name' });
    expect(out.map((d) => d.id)).toEqual(['e1']);
  });

  it('counts decks per format across the whole library', () => {
    const counts = countByFormat(decks);
    expect(counts.get('standard')).toBe(2);
    expect(counts.get('eden')).toBe(1);
    expect(counts.get('titan')).toBe(1);
  });

  it('derives buckets in registry order, then appends unknown ids, skipping empties', () => {
    const buckets = deriveFormatBuckets(decks, formats);
    expect(buckets).toEqual([
      { id: 'standard', label: 'STANDARD', count: 2 },
      { id: 'eden', label: 'EDEN', count: 1 },
      { id: 'titan', label: 'TITAN', count: 1 },
    ]);
  });

  it('omits registry formats that have no decks', () => {
    const withPauper = [...formats, fmt('pauper', 'PAUPER')];
    const buckets = deriveFormatBuckets(decks, withPauper);
    expect(buckets.map((b) => b.id)).not.toContain('pauper');
  });

  it('labels a format by registry name and falls back to the raw id', () => {
    expect(formatLabel('eden', formats)).toBe('EDEN');
    expect(formatLabel('mystery_mode', formats)).toBe('MYSTERY_MODE');
  });
});
