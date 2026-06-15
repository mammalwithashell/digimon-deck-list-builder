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
  player2: player(2, [basePermanent]),
  revealedCards: [],
  pendingSelection: null,
  pendingAttack: null,
};

const event = (overrides: Partial<GameEvent>): GameEvent => ({
  type: 'play',
  seq: 1,
  player: 1,
  source_card_id: null,
  source_card_name: null,
  source_slot: null,
  target_card_id: null,
  target_card_name: null,
  target_slot: null,
  meta: {},
  ...overrides,
});

describe('formatEvent', () => {
  it('formats play events as [CARD-ID: Name] from event identity', () => {
    const lines = formatEvent(
      event({
        type: 'play',
        source_card_id: 'BT1-001',
        source_card_name: 'Agumon',
        meta: { cost_paid: 3 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual(['You played [BT1-001: Agumon] for 3 memory.']);
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

    expect(lines).toEqual(['You digivolved into [BT1-002: Agumon] for 2 memory.']);
  });

  it('names attacker and target Digimon instead of slot numbers', () => {
    const lines = formatEvent(
      event({
        type: 'Attack',
        source_card_id: 'BT1-009',
        source_card_name: 'Greymon',
        source_slot: 0,
        target_card_id: 'BT25-020',
        target_card_name: 'Tyrannomon',
        target_slot: 0,
        meta: { target_player: 2 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You attacked [BT25-020: Tyrannomon] with [BT1-009: Greymon].',
    ]);
  });

  it('annotates a battle with each Digimon DP when present', () => {
    const lines = formatEvent(
      event({
        type: 'Attack',
        source_card_id: 'BT1-009',
        source_card_name: 'Greymon',
        source_slot: 0,
        target_card_id: 'BT25-020',
        target_card_name: 'Tyrannomon',
        target_slot: 0,
        meta: { target_player: 2, attacker_dp: 5000, target_dp: 3000 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You attacked [BT25-020: Tyrannomon] (3000 DP) with [BT1-009: Greymon] (5000 DP).',
    ]);
  });

  it('renders a security attack target as "security"', () => {
    const lines = formatEvent(
      event({
        type: 'Attack',
        source_card_id: 'BT1-009',
        source_card_name: 'Greymon',
        source_slot: 0,
        meta: { target_player: 2 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual(['You attacked security with [BT1-009: Greymon].']);
  });

  it('attributes effect-driven memory changes to their source card', () => {
    const lines = formatEvent(
      event({
        type: 'MemoryChange',
        source_card_id: 'BT25-098',
        source_card_name: 'Cyber Engage',
        meta: { delta: 1, total: 6 },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You gained 1 memory from [BT25-098: Cyber Engage] (now 6).',
    ]);
  });

  it('leaves sourceless memory changes unattributed', () => {
    const lines = formatEvent(
      event({ type: 'MemoryChange', meta: { delta: -2, total: 1 } }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual(['You lost 2 memory (now 1).']);
  });

  it('formats an effect-target line naming source and targets', () => {
    const lines = formatEvent(
      event({
        type: 'EffectTarget',
        source_card_id: 'BT1-009',
        source_card_name: 'Greymon',
        meta: {
          targets: [{ card_id: 'BT25-020', card_name: 'Tyrannomon' }],
        },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      "[BT1-009: Greymon]'s effect targeted [BT25-020: Tyrannomon].",
    ]);
  });

  it('formats reveal lines by source zone', () => {
    const deckTop = formatEvent(
      event({
        type: 'Reveal',
        source_card_id: 'BT25-052',
        source_card_name: 'Logimon',
        meta: { source_zone: 'DeckTop' },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );
    expect(deckTop).toEqual([
      'You revealed [BT25-052: Logimon] from the top of their deck.',
    ]);

    const mill = formatEvent(
      event({
        type: 'Reveal',
        source_card_id: 'BT25-056',
        source_card_name: 'Bootmon',
        meta: { source_zone: 'TrashFromDeckTop' },
      }),
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );
    expect(mill).toEqual([
      'You trashed [BT25-056: Bootmon] from the top of their deck.',
    ]);
  });

  it('shows a milled card once (suppresses the paired Trash line)', () => {
    const lines = formatEvents(
      [
        event({
          type: 'Reveal',
          seq: 1,
          player: 1,
          source_card_id: 'BT25-056',
          source_card_name: 'Bootmon',
          meta: { source_zone: 'TrashFromDeckTop' },
        }),
        event({
          type: 'Trash',
          seq: 2,
          player: 1,
          source_card_id: 'BT25-056',
          source_card_name: 'Bootmon',
        }),
      ],
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You trashed [BT25-056: Bootmon] from the top of their deck.',
    ]);
  });

  it('still shows a non-mill trash even when an unrelated card was milled', () => {
    const lines = formatEvents(
      [
        event({
          type: 'Reveal',
          seq: 1,
          player: 1,
          source_card_id: 'BT25-056',
          source_card_name: 'Bootmon',
          meta: { source_zone: 'TrashFromDeckTop' },
        }),
        event({
          type: 'Trash',
          seq: 2,
          player: 1,
          source_card_id: 'BT25-056',
          source_card_name: 'Bootmon',
        }),
        event({
          type: 'Trash',
          seq: 3,
          player: 1,
          source_card_id: 'BT1-010',
          source_card_name: 'Tentomon',
        }),
      ],
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You trashed [BT25-056: Bootmon] from the top of their deck.',
      'You trashed [BT1-010: Tentomon].',
    ]);
  });

  it('formats memory, security, and game-over events in sequence order', () => {
    const lines = formatEvents(
      [
        event({
          type: 'SecurityReveal',
          seq: 3,
          player: 2,
          source_card_id: 'BT3-003',
          source_card_name: 'Greymon',
        }),
        event({ type: 'MemoryChange', seq: 2, player: 1, meta: { delta: -2, total: 1 } }),
        event({ type: 'GameOver', seq: 4, player: 0, meta: { winner: 1 } }),
      ],
      { state, playerLabels: { 1: 'You', 2: 'Bot' } },
    );

    expect(lines).toEqual([
      'You lost 2 memory (now 1).',
      "Bot's security revealed [BT3-003: Greymon].",
      'You won the game.',
    ]);
  });

  it('skips unknown or unrenderable events safely', () => {
    expect(formatEvent(event({ type: 'debug_internal' }), { state })).toEqual([]);
  });
});
