import { describe, expect, it } from 'vitest';
import type { GameEvent, GameState, PermanentInfo, PlayerState } from '@/types/game';
import { formatEvent, formatEvents } from './gameLogFormat';

const basePermanent: PermanentInfo = {
  topCardId: 'BT1-001',
  topCardName: 'Agumon',
  dp: 2000,
  level: 3,
  isSuspended: false,
  sourceCount: 1,
  keywords: [],
  keywordBreakdown: { innate: [], gained: [] },
  securityAttackModifier: 0,
  linkedCardIds: [],
  sources: [],
  mainEffectText: '',
  inheritedEffects: [],
  modifiers: [],
  dpBreakdown: { base: 2000, sources: [], temporary: 0, total: 2000 },
  turnPlayed: 1,
  colors: [],
};

const player = (id: number, battleArea: PermanentInfo[] = []): PlayerState => ({
  id,
  memory: 0,
  handCount: 0,
  handIds: [],
  handCards: [],
  securityCount: 5,
  securityIds: [],
  deckCount: 45,
  eggDeckCount: 5,
  battleAreaCount: battleArea.length,
  battleArea,
  breedingArea: null,
  trashIds: [],
});

const state: GameState = {
  turnCount: 1,
  currentPhase: 3,
  currentPlayer: 0,
  memoryGauge: 0,
  isGameOver: false,
  winner: null,
  player1: player(1, [basePermanent]),
  player2: player(2),
  revealedCards: [],
  pendingSelection: null,
  pendingAttack: null,
};

const event = (overrides: Partial<GameEvent>): GameEvent => ({
  type: 'play',
  seq: 1,
  player: 1,
  source_card_id: null,
  source_slot: null,
  target_card_id: null,
  target_slot: null,
  meta: {},
  ...overrides,
});

describe('formatEvent', () => {
  it('formats play events with clickable card references and player labels', () => {
    const lines = formatEvent(
      event({
        type: 'play',
        source_card_id: 'BT1-001',
        meta: { card_name: 'Agumon', cost_paid: 3 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual(['You played [BT1-001:Agumon] for 3 memory.']);
  });

  it('falls back to current board state when an event lacks a card name', () => {
    const lines = formatEvent(
      event({
        type: 'digivolve',
        source_card_id: 'BT1-002',
        source_slot: 0,
        meta: { memory_paid: 2 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual(['You digivolved into [BT1-002:Agumon] for 2 memory.']);
  });

  it('formats memory, security, and game-over events in sequence order', () => {
    const lines = formatEvents(
      [
        event({ type: 'SecurityReveal', seq: 3, player: 2, source_card_id: 'BT3-003' }),
        event({ type: 'MemoryChange', seq: 2, player: 1, meta: { delta: -2, total: 1 } }),
        event({ type: 'GameOver', seq: 4, player: 0, meta: { winner: 1 } }),
      ],
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You lost 2 memory (now 1).',
      "Bot's security revealed [BT3-003:BT3-003].",
      'You won the game.',
    ]);
  });

  it('skips unknown or unrenderable events safely', () => {
    expect(formatEvent(event({ type: 'debug_internal' }), { state })).toEqual([]);
  });
});
