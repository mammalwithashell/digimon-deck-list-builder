import { describe, expect, it } from 'vitest';
import { BOT_NAMES, pickAlias, liveGameLabels } from './botNames';

describe('BOT_NAMES roster', () => {
  it('is non-empty', () => {
    expect(BOT_NAMES.length).toBeGreaterThan(0);
  });

  it('has no duplicates', () => {
    expect(new Set(BOT_NAMES).size).toBe(BOT_NAMES.length);
  });

  it('contains the Shambala host-computer nod (Kunlun)', () => {
    expect(BOT_NAMES).toContain('Kunlun');
  });
});

describe('pickAlias', () => {
  it('always returns a roster member', () => {
    for (const seed of ['game-1:2', 'abc:1', '', 'x', '999999:2', 'g7:1']) {
      expect(BOT_NAMES).toContain(pickAlias(seed));
    }
  });

  it('is deterministic for a given seed (stable, no flicker)', () => {
    expect(pickAlias('game-42:2')).toBe(pickAlias('game-42:2'));
  });

  it('varies across seeds (spreads over the roster)', () => {
    const picks = new Set(
      Array.from({ length: 200 }, (_, i) => pickAlias(`game-${i}:2`)),
    );
    // With 200 distinct seeds over the roster, we expect broad coverage —
    // far more than a single constant value.
    expect(picks.size).toBeGreaterThan(5);
  });
});

describe('liveGameLabels', () => {
  it('keeps the seated player as "You" and aliases the opponent (P1 seat)', () => {
    const labels = liveGameLabels('game-7', 1);
    expect(labels[1]).toBe('You');
    expect(BOT_NAMES).toContain(labels[2]);
  });

  it('keeps the seated player as "You" and aliases the opponent (P2 seat)', () => {
    const labels = liveGameLabels('game-7', 2);
    expect(labels[2]).toBe('You');
    expect(BOT_NAMES).toContain(labels[1]);
  });

  it('aliases BOTH seats for a spectator (no seat)', () => {
    for (const noSeat of [null, undefined]) {
      const labels = liveGameLabels('game-7', noSeat);
      expect(BOT_NAMES).toContain(labels[1]);
      expect(BOT_NAMES).toContain(labels[2]);
      expect(labels[1]).not.toBe('Player 1');
      expect(labels[2]).not.toBe('Player 2');
    }
  });

  it('is stable across repeated calls for the same game (no flicker)', () => {
    expect(liveGameLabels('game-99', 1)).toEqual(liveGameLabels('game-99', 1));
    expect(liveGameLabels('game-99', null)).toEqual(liveGameLabels('game-99', null));
  });

  it('never shows the real placeholder labels', () => {
    const all = [
      ...Object.values(liveGameLabels('g', 1)),
      ...Object.values(liveGameLabels('g', 2)),
      ...Object.values(liveGameLabels('g', null)),
    ];
    for (const placeholder of ['GREEDY BOT', 'AI', 'Opponent', 'Agent', 'Player 1', 'Player 2']) {
      expect(all).not.toContain(placeholder);
    }
  });
});
