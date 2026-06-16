import { describe, expect, it } from 'vitest';
import type { GameEvent } from '@/types/game';
import { normalizeGameEvent, normalizeGameEvents } from './gameEvents';

describe('normalizeGameEvent', () => {
  it('maps Rust/PyO3 PascalCase event names to canonical frontend names', () => {
    const event: GameEvent = {
      type: 'Digivolve',
      seq: 7,
      player: 1,
      source_card_id: 'BT1-001',
      source_card_name: 'Agumon',
      source_slot: 0,
      target_card_id: null,
      target_card_name: null,
      target_slot: null,
      meta: {},
    };

    expect(normalizeGameEvent(event).type).toBe('digivolve');
  });

  it('preserves legacy lowercase event names used by existing animations', () => {
    const event: GameEvent = {
      type: 'security_reveal',
      seq: 8,
      player: 2,
      source_card_id: 'BT2-001',
      source_card_name: null,
      source_slot: null,
      target_card_id: null,
      target_card_name: null,
      target_slot: null,
      meta: {},
    };

    expect(normalizeGameEvent(event)).toEqual(event);
  });

  it('normalizes batches without mutating the original events', () => {
    const events: GameEvent[] = [
      {
        type: 'MemoryChange',
        seq: 1,
        player: 1,
        source_card_id: null,
        source_card_name: null,
        source_slot: null,
        target_card_id: null,
        target_card_name: null,
        target_slot: null,
        meta: { delta: -3, total: -3 },
      },
    ];

    const normalized = normalizeGameEvents(events);

    expect(normalized[0]?.type).toBe('memory_change');
    expect(events[0]?.type).toBe('MemoryChange');
  });
});
