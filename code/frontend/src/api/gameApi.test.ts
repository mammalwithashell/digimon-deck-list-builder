import { describe, expect, it } from 'vitest';
import type { ActionTrace, TensorSummary } from '@/types/game';
import { toTensorSummary } from './gameApi';

describe('toTensorSummary', () => {
  it('translates tensor profile metadata', () => {
    const summary = toTensorSummary({
      player_id: 0,
      profile_id: 'standard_v1',
      profile_version: 1,
      tensor_size: 1375,
      mask_size: 2168,
      legal_action_count: 12,
      card_id_slot_count: 520,
      scalar_slot_count: 855,
      turn_count: 4,
      phase: 'Main',
      memory: 2,
      tensor_head: [0, 3, 0.2],
    });

    expect(summary.profileId).toBe('standard_v1');
    expect(summary.profileVersion).toBe(1);
    expect(summary.cardIdSlotCount).toBe(520);
    expect(summary.scalarSlotCount).toBe(855);
    expect(summary.tensorSize).toBe(1375);
  });
});

describe('engine trace types', () => {
  it('supports agent traces with tensor summaries', () => {
    const summary: TensorSummary = {
      playerId: 1,
      profileId: 'standard_v1',
      profileVersion: 1,
      tensorSize: 1375,
      maskSize: 2168,
      legalActionCount: 4,
      cardIdSlotCount: 520,
      scalarSlotCount: 855,
      turnCount: 3,
      phase: 'Main',
      memory: 2,
      tensorHead: [0.1, 3, 0.2],
    };
    const trace: ActionTrace = {
      actor: 'agent_trained',
      playerId: 1,
      actionId: 62,
      decoded: {
        actionId: 62,
        playerId: 1,
        phase: 'Main',
        kind: 'pass',
        label: 'Pass / decline',
        sourceZone: null,
        sourceIndex: null,
        targetZone: null,
        targetIndex: null,
        cardId: null,
        cardName: null,
      },
      tensorSummary: summary,
    };

    expect(trace.tensorSummary?.tensorSize).toBe(1375);
    expect(trace.tensorSummary?.maskSize).toBe(2168);
    expect(trace.decoded.label).toBe('Pass / decline');
  });
});
