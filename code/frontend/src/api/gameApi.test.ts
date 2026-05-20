import { describe, expect, it } from 'vitest';
import type { ActionTrace, TensorSummary } from '@/types/game';
import { dtoToGameState, toTensorSummary } from './gameApi';

describe('toTensorSummary', () => {
  it('translates tensor profile metadata', () => {
    const summary = toTensorSummary({
      player_id: 0,
      profile_id: 'standard_v1',
      profile_version: 1,
      tensor_size: 1375,
      mask_size: 2192,
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
      maskSize: 2192,
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
    expect(trace.tensorSummary?.maskSize).toBe(2192);
    expect(trace.decoded.label).toBe('Pass / decline');
  });
});

describe('dtoToGameState', () => {
  it('maps dual cards to the option-compatible card kind', () => {
    const state = dtoToGameState({
      turn_count: 1,
      turn_player: 0,
      current_phase: 'Main',
      memory: 0,
      game_over: false,
      winner: null,
      mulligan_current_player: null,
      mulligan_used: [false, false],
      players: [
        {
          id: 0,
          hand: [
            {
              card_id: 'DUAL-001',
              card_name: 'Dual Test',
              card_kind: 'Dual',
              level: 6,
              dp: 12000,
              play_cost: 5,
              colors: ['Purple'],
            },
          ],
          battle_area: [],
          breeding: null,
          deck_count: 49,
          trash_count: 0,
          security_count: 5,
          is_eliminated: false,
        },
        {
          id: 1,
          hand: [],
          battle_area: [],
          breeding: null,
          deck_count: 50,
          trash_count: 0,
          security_count: 5,
          is_eliminated: false,
        },
      ],
    });

    expect(state.player1.handCards[0]?.cardKind).toBe(2);
  });
});
