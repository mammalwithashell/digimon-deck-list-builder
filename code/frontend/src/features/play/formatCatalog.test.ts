import { describe, expect, it } from 'vitest';
import { canUseDeckForFormat, formatToQueueType, PLAY_FORMATS } from './formatCatalog';
import type { DeckSummary } from '@/types/deck';

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
  it('enables every engine-registry format and keeps concept formats disabled', () => {
    expect(PLAY_FORMATS.find((f) => f.id === 'standard')?.enabled).toBe(true);
    expect(PLAY_FORMATS.filter((f) => f.enabled).map((f) => f.id)).toEqual([
      'standard',
      'no_restriction',
      'pauper',
      'eden',
      'eden_singleton',
    ]);
    expect(PLAY_FORMATS.find((f) => f.id === 'titan')?.enabled).toBe(false);
    expect(PLAY_FORMATS.find((f) => f.id === 'edh_commander')?.enabled).toBe(false);
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

  it('rejects decks for disabled concept formats', () => {
    expect(canUseDeckForFormat(deck({}), 'titan').ok).toBe(false);
  });

  it('maps quick match to casual queue', () => {
    expect(formatToQueueType('standard')).toBe('casual');
    expect(formatToQueueType('eden_singleton')).toBe('casual');
  });
});
