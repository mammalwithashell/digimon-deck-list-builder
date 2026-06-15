import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ActionTrace, TensorSummary } from '@/types/game';
import {
  createGame,
  dtoToGameState,
  normalizeSeedInput,
  toTensorSummary,
} from './gameApi';

afterEach(() => {
  vi.unstubAllGlobals();
});

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

  // The engine reports `winner` as a raw Rust player id (0/1), but the UI
  // (ResultOverlay/PhaseIndicator `localPlayer={1}`, playerLabels `{1,2}`) is
  // 1-indexed. Without the +1 conversion, the local human (engine 0) winning
  // compares `0 === 1` → false → "Defeat" shown on a win. Map it like
  // `selectingPlayer`.
  it('maps the engine winner (0/1) to the UI 1-indexed convention', () => {
    const player = (id: number) => ({
      id,
      hand: [],
      battle_area: [],
      breeding: null,
      deck_count: 50,
      trash_count: 0,
      security_count: 5,
      is_eliminated: false,
    });
    const dto = (winner: number | null) => ({
      turn_count: 1,
      turn_player: 0,
      current_phase: 'Main',
      memory: 0,
      game_over: winner !== null,
      winner,
      mulligan_current_player: null,
      mulligan_used: [false, false],
      players: [player(0), player(1)],
    });

    expect(dtoToGameState(dto(0)).winner).toBe(1); // engine 0 (human) → UI 1
    expect(dtoToGameState(dto(1)).winner).toBe(2); // engine 1 (opponent) → UI 2
    expect(dtoToGameState(dto(null)).winner).toBeNull();
  });

  // Engine `memory` is from the TURN PLAYER's perspective; `memoryGauge` (like
  // the browser wire's to_ui_json) must be from PLAYER 1's perspective. Without
  // the flip, the opponent's memory renders on the local player's side of the
  // gauge during paced bot turns (regression caught playtesting
  // add-bot-action-pacing).
  it('orients memoryGauge to player 1 regardless of whose turn it is', () => {
    const player = (id: number) => ({
      id,
      hand: [],
      battle_area: [],
      breeding: null,
      deck_count: 50,
      trash_count: 0,
      security_count: 5,
      is_eliminated: false,
    });
    const dto = (turnPlayer: number, memory: number) => ({
      turn_count: 2,
      turn_player: turnPlayer,
      current_phase: 'Main',
      memory,
      game_over: false,
      winner: null,
      mulligan_current_player: null,
      mulligan_used: [false, false],
      players: [player(0), player(1)],
    });

    // My turn, I hold 6 memory → my side of the gauge (positive).
    expect(dtoToGameState(dto(0, 6)).memoryGauge).toBe(6);
    // Opponent's turn, THEY hold 6 memory → their side (negative for p1).
    expect(dtoToGameState(dto(1, 6)).memoryGauge).toBe(-6);
    // Player states stay consistent with the gauge.
    expect(dtoToGameState(dto(1, 6)).player1.memory).toBe(-6);
    expect(dtoToGameState(dto(1, 6)).player2.memory).toBe(6);
  });
});

describe('createGame', () => {
  it('preserves opening response events and normalizes their type names', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        json: async () => ({
          game_id: 'game-1',
          state: {
            turnCount: 1,
            currentPhase: 3,
            currentPlayer: 1,
            memoryGauge: 0,
            isGameOver: false,
            winner: null,
            player1: {},
            player2: {},
            revealedCards: [],
            pendingSelection: null,
            pendingAttack: null,
          },
          action_mask: [1, 0],
          logs: [],
          events: [
            {
              type: 'MemoryChange',
              seq: 1,
              player: 2,
              source_card_id: null,
              source_card_name: null,
              source_slot: null,
              target_card_id: null,
              target_card_name: null,
              target_slot: null,
              meta: { delta: -3, total: 0 },
            },
          ],
          agent_pending: true,
        }),
      })),
    );

    const response = await createGame({
      deck1: ['BT1-001'],
      deck2: ['BT1-001'],
      player1_type: 'human',
      player2_type: 'agent',
      paced: true,
    });

    expect(response.events?.map((e) => e.type)).toEqual(['memory_change']);
  });

  it('forwards and returns the effective seed as a lossless string', async () => {
    const fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toMatchObject({
        seed: '18446744073709551615',
      });
      return {
        ok: true,
        json: async () => ({
          game_id: 'game-1',
          seed: '18446744073709551615',
          state: {
            turnCount: 1,
            currentPhase: 3,
            currentPlayer: 1,
            memoryGauge: 0,
            isGameOver: false,
            winner: null,
            player1: {},
            player2: {},
            revealedCards: [],
            pendingSelection: null,
            pendingAttack: null,
          },
          action_mask: [1, 0],
          logs: [],
          events: [],
        }),
      };
    });
    vi.stubGlobal('fetch', fetchMock);

    const response = await createGame({
      deck1: ['BT1-001'],
      deck2: ['BT1-001'],
      player1_type: 'human',
      player2_type: 'agent',
      seed: '18446744073709551615',
    });

    expect(response.seed).toBe('18446744073709551615');
  });
});

describe('normalizeSeedInput', () => {
  it('accepts blank input as generated-seed mode', () => {
    expect(normalizeSeedInput('')).toBeNull();
    expect(normalizeSeedInput('   ')).toBeNull();
  });

  it('preserves large u64 decimal strings exactly', () => {
    expect(normalizeSeedInput('18446744073709551615')).toBe(
      '18446744073709551615',
    );
  });

  it('rejects invalid decimal seed input', () => {
    expect(() => normalizeSeedInput('-1')).toThrow(/base-10/);
    expect(() => normalizeSeedInput('1.5')).toThrow(/base-10/);
    expect(() => normalizeSeedInput('18446744073709551616')).toThrow(/u64/);
  });
});
