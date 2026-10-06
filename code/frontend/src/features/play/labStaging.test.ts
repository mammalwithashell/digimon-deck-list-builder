import { describe, expect, it } from 'vitest';
import {
  mainSlotLabel,
  moveCard,
  seatProblem,
  shuffled,
  toEngineDeck,
  toStaged,
} from './labStaging';

describe('mainSlotLabel', () => {
  it('labels hand, security (top checked first) and draws', () => {
    expect(mainSlotLabel(0)).toEqual({ zone: 'hand', text: 'HAND' });
    expect(mainSlotLabel(4).zone).toBe('hand');
    expect(mainSlotLabel(5)).toEqual({ zone: 'security', text: 'SEC 5' });
    expect(mainSlotLabel(9)).toEqual({ zone: 'security', text: 'SEC 1' });
    expect(mainSlotLabel(10)).toEqual({ zone: 'draw', text: 'DRAW 1' });
    expect(mainSlotLabel(12).text).toBe('DRAW 3');
  });
});

describe('moveCard', () => {
  it('moves up, down, to the top and clamps out-of-range targets', () => {
    const l = ['a', 'b', 'c', 'd'];
    expect(moveCard(l, 2, 0)).toEqual(['c', 'a', 'b', 'd']);
    expect(moveCard(l, 0, 3)).toEqual(['b', 'c', 'd', 'a']);
    expect(moveCard(l, 1, 99)).toEqual(['a', 'c', 'd', 'b']);
    expect(moveCard(l, 1, 1)).toBe(l);
    expect(l).toEqual(['a', 'b', 'c', 'd']);
  });
});

describe('shuffled', () => {
  it('is a permutation and does not mutate', () => {
    const l = [1, 2, 3, 4, 5];
    const out = shuffled(l, () => 0);
    expect([...out].sort()).toEqual(l);
    expect(l).toEqual([1, 2, 3, 4, 5]);
  });
});

describe('toEngineDeck / seatProblem', () => {
  const seat = (n: number) => ({
    deckId: 'd',
    deckName: 'D',
    main: toStaged(Array.from({ length: n }, (_, i) => `M${i}`)),
    eggs: toStaged(['E0', 'E1']),
  });

  it('emits eggs then main, both top-first', () => {
    expect(toEngineDeck(seat(3))).toEqual(['E0', 'E1', 'M0', 'M1', 'M2']);
  });

  it('requires a deck and enough cards for hand + security', () => {
    expect(seatProblem({ ...seat(10), deckId: null })).toBe('Pick a deck');
    expect(seatProblem(seat(9))).toMatch(/at least 10/);
    expect(seatProblem(seat(10))).toBeNull();
  });
});
