import { describe, expect, it, vi } from 'vitest';
import { listFormats } from '@/api/deckApi';
import type { DeckFormat, DeckSummary } from '@/types/deck';
import {
  canUseDeckForFormat,
  formatToQueueType,
  loadPlayFormats,
  PLAY_FORMATS,
} from './formatCatalog';

vi.mock('@/api/deckApi', () => ({ listFormats: vi.fn() }));

const deck = (overrides: Partial<DeckSummary>): DeckSummary => ({
  id: 'd1',
  name: 'Standard Legal',
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
  meta_tier: 'rogue',
  meta_archetype: 'Test Archetype',
  colors: ['Red'],
  highest_level: 6,
  created_at: '2026-04-27T00:00:00.000Z',
  updated_at: '2026-04-27T00:00:00.000Z',
  ...overrides,
});

describe('formatCatalog', () => {
  it('lists exactly the engine-registry playable formats and no concept placeholders', () => {
    expect(PLAY_FORMATS.find((f) => f.id === 'standard')?.enabled).toBe(true);
    expect(PLAY_FORMATS.map((f) => f.id)).toEqual([
      'standard',
      'no_restriction',
      'pauper',
      'eden',
      'eden_singleton',
    ]);
    expect(PLAY_FORMATS.every((f) => f.enabled)).toBe(true);
    // The exact-list assertion above already proves no concept placeholders
    // (titan / edh_commander / draft / tutorial) are present.
  });

  it('accepts a 50 plus 4 deck for an enabled format', () => {
    expect(canUseDeckForFormat(deck({}), 'standard')).toEqual({ ok: true });
    expect(canUseDeckForFormat(deck({ game_mode: 'eden' }), 'eden')).toEqual({ ok: true });
  });

  it('rejects a deck built for a different format', () => {
    // A Standard deck is not selectable in the Eden queue, even if its shape
    // fits — its validity was computed under Standard rules.
    const result = canUseDeckForFormat(deck({ game_mode: 'standard' }), 'eden');
    expect(result.ok).toBe(false);
  });

  it('rejects incomplete and invalid decks', () => {
    expect(canUseDeckForFormat(deck({ main_count: 43, card_count: 47 }), 'standard')).toEqual({
      ok: false,
      reason: 'Requires 50 main cards and 0-5 eggs.',
    });
    expect(canUseDeckForFormat(deck({ is_valid: false }), 'standard')).toEqual({
      ok: false,
      reason: 'Deck must pass validation before queueing.',
    });
  });

  it('maps quick match to casual queue', () => {
    expect(formatToQueueType('standard')).toBe('casual');
    expect(formatToQueueType('eden_singleton')).toBe('casual');
  });

  it('loadPlayFormats returns registry formats only, with no concept placeholders', async () => {
    const registry: DeckFormat[] = [
      {
        id: 'standard',
        name: 'Standard',
        description: '50-card decks.',
        deck_size: 50,
        egg_max: 5,
        rarity_policy: 'all',
        singleton: false,
        default_max_copies: 4,
        playable: true,
      },
      {
        id: 'eden_singleton',
        name: 'EDEN Singleton',
        description: 'Highlander EDEN.',
        deck_size: 50,
        egg_max: 5,
        rarity_policy: 'eden_anomaly',
        singleton: true,
        default_max_copies: 1,
        playable: true,
      },
    ];
    vi.mocked(listFormats).mockResolvedValue(registry);

    const formats = await loadPlayFormats();

    expect(formats.map((f) => f.id)).toEqual(['standard', 'eden_singleton']);
    expect(formats.every((f) => f.enabled)).toBe(true);
    expect(formats.find((f) => f.id === 'eden_singleton')?.deckLabel).toBe('50 singleton');
  });
});
