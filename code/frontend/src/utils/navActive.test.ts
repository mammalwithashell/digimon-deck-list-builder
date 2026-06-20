import { describe, expect, it } from 'vitest';
import { isActivePath } from './navActive';

describe('isActivePath', () => {
  it('matches an exact item only on the exact path', () => {
    expect(isActivePath('/', { to: '/', exact: true })).toBe(true);
    expect(isActivePath('/play', { to: '/', exact: true })).toBe(false);
  });

  it('matches a prefix item on the path and its nested routes', () => {
    expect(isActivePath('/play', { to: '/play' })).toBe(true);
    expect(isActivePath('/play/deck', { to: '/play' })).toBe(true);
    expect(isActivePath('/deckbuilder/new', { to: '/deckbuilder' })).toBe(true);
  });

  it('does not match a sibling that merely shares a string prefix', () => {
    // `/play-area` must not light up the `/play` item.
    expect(isActivePath('/play-area', { to: '/play' })).toBe(false);
  });

  it('does not match an unrelated path', () => {
    expect(isActivePath('/deckbuilder', { to: '/play' })).toBe(false);
  });
});
