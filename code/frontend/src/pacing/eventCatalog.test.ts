import { describe, it, expect } from 'vitest';
import { buildPresentationBeats } from './eventCatalog';
import { GamePhase, type GameEvent, type GameState } from '@/types/game';

const MOCK_STATE: GameState = {
  turnCount: 2,
  currentPhase: GamePhase.Main,
  currentPlayer: 2,
  firstPlayer: 0,
  memoryGauge: -3,
  isGameOver: false,
  winner: null,
  player1: {
    id: 1,
    memory: -3,
    handCount: 5,
    handIds: [],
    handCards: [],
    securityCount: 5,
    securityIds: [],
    deckCount: 40,
    eggDeckCount: 4,
    battleAreaCount: 0,
    battleArea: [],
    breedingArea: null,
    trashIds: [],
  },
  player2: {
    id: 2,
    memory: 3,
    handCount: 4,
    handIds: [],
    handCards: [],
    securityCount: 5,
    securityIds: [],
    deckCount: 40,
    eggDeckCount: 4,
    battleAreaCount: 1,
    battleArea: [],
    breedingArea: null,
    trashIds: [],
  },
  revealedCards: [],
  pendingSelection: null,
  pendingAttack: null,
};

function ev(type: string, extra: Record<string, unknown> = {}): GameEvent {
  return {
    type,
    seq: 1,
    player: 2,
    source_card_id: null,
    source_card_name: null,
    source_slot: null,
    target_card_id: null,
    target_card_name: null,
    target_slot: null,
    meta: {},
    ...extra,
  } as GameEvent;
}

describe('buildPresentationBeats', () => {
  it('gives opponent plays a readable showcase hold and coalesces structural memory payment', () => {
    const beats = buildPresentationBeats({
      prevState: MOCK_STATE,
      nextState: MOCK_STATE,
      events: [
        ev('play', { player: 2, card_id: 'BT1-010', card_name: 'Agumon', field_index: 0, cost_paid: 3 }),
        ev('memory_change', { player: 2, delta: -3, total: 0 }),
      ],
      speed: 'normal',
      motion: 'full',
    });

    expect(beats).toHaveLength(1);
    expect(beats[0]!.showcase?.cardId).toBe('BT1-010');
    expect(beats[0]!.memoryDelta).toBe(-3);
    expect(beats[0]!.durationMs).toBeGreaterThanOrEqual(500);
  });

  it('accelerates chain tail effects (#3+) so long combos stay brisk', () => {
    const events = [0, 1, 2, 3].map((i) =>
      ev('effect_activated', {
        seq: i + 1,
        player: 2,
        source_card_id: `BT1-00${i + 1}`,
        source_card_name: `Card ${i + 1}`,
        source_slot: 0,
        timing: 'OnPlay',
      }),
    );

    const beats = buildPresentationBeats({
      prevState: MOCK_STATE,
      nextState: MOCK_STATE,
      events,
      speed: 'normal',
      motion: 'full',
    });

    expect(beats).toHaveLength(4);
    expect(beats[0]!.durationMs).toBe(beats[1]!.durationMs);
    expect(beats[3]!.durationMs).toBeLessThan(beats[0]!.durationMs);
  });

  it('returns empty beats in instant or motion-off mode', () => {
    const events = [ev('play', { player: 2, card_id: 'BT1-010', card_name: 'Agumon' })];
    expect(
      buildPresentationBeats({
        prevState: MOCK_STATE,
        nextState: MOCK_STATE,
        events,
        speed: 'instant',
        motion: 'full',
      }),
    ).toEqual([]);
    expect(
      buildPresentationBeats({
        prevState: MOCK_STATE,
        nextState: MOCK_STATE,
        events,
        speed: 'normal',
        motion: 'off',
      }),
    ).toEqual([]);
  });

  it('emits directional card-flight cues for draw, mill/trash, bounce, and recovery events', () => {
    const beats = buildPresentationBeats({
      prevState: MOCK_STATE,
      nextState: MOCK_STATE,
      events: [
        ev('draw', { seq: 1, player: 2, count: 1 }),
        ev('mill', { seq: 2, player: 1, card_id: 'BT1-010', card_name: 'Agumon' }),
        ev('return_to_hand', { seq: 3, player: 1, card_id: 'BT1-015', card_name: 'Greymon' }),
        ev('recovery', { seq: 4, player: 1, count: 1 }),
      ],
      speed: 'normal',
      motion: 'full',
    });

    expect(beats).toHaveLength(4);
    expect(beats[0]!.flight).toEqual({
      cardId: null,
      cardName: null,
      toZone: 'hand',
      actor: 'opp',
      label: 'Draw',
    });
    expect(beats[1]!.flight?.toZone).toBe('trash');
    expect(beats[1]!.flight?.label).toBe('Milled');
    expect(beats[2]!.flight?.toZone).toBe('hand');
    expect(beats[3]!.flight?.toZone).toBe('security');
  });
});
