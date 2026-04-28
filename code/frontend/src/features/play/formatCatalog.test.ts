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
  it('exposes standard as the only engine-launchable format', () => {
    expect(PLAY_FORMATS.find((f) => f.id === 'standard')?.enabled).toBe(true);
    expect(PLAY_FORMATS.filter((f) => f.enabled).map((f) => f.id)).toEqual(['standard']);
  });

  it('accepts a 50 plus 4 standard deck for standard', () => {
    expect(canUseDeckForFormat(deck({}), 'standard')).toEqual({ ok: true });
  });

  it('rejects incomplete and invalid standard decks', () => {
    expect(canUseDeckForFormat(deck({ main_count: 43, card_count: 47 }), 'standard')).toEqual({
      ok: false,
      reason: 'Standard requires 50 main cards and 0-5 eggs.',
    });
    expect(canUseDeckForFormat(deck({ is_valid: false }), 'standard')).toEqual({
      ok: false,
      reason: 'Deck must pass validation before queueing.',
    });
  });

  it('maps standard quick match to casual queue', () => {
    expect(formatToQueueType('standard')).toBe('casual');
  });
});
