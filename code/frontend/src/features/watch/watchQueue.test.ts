import { describe, expect, it } from 'vitest';
import {
  isMirrorMatch,
  playableGames,
  watchGameUrl,
  watchPlaylist,
  type WatchGameRow,
  type WatchQueueIndex,
} from './watchQueue';

function row(id: string, a: string, b: string, replayable?: boolean): WatchGameRow {
  return {
    id,
    block: 0,
    archetypes: [a, b],
    seat_labels: ['CP8 (closed list)', 'CP8 (knows list)'],
    first_player: 0,
    winner: 0,
    terminal_reason: null,
    moves: 10,
    searched: 5,
    replayable,
  };
}

const index: WatchQueueIndex = {
  title: 't',
  source: 's',
  games: [
    row('mirror', 'Examon', 'Examon', true),
    row('ok', 'Examon', 'Glowing Dawn', true),
    row('diverged', 'Examon', 'Chronicle', false),
    row('unverified', 'Chronicle', 'Glowing Dawn'),
  ],
};

describe('watch queue playlist', () => {
  it('detects same-archetype mirrors', () => {
    expect(isMirrorMatch(row('m', 'SW', 'SW'))).toBe(true);
    expect(isMirrorMatch(row('n', 'SW', 'Examon'))).toBe(false);
  });

  it('drops mirrors and games the engine no longer reproduces, keeps unverified ones', () => {
    expect(playableGames(index).map((g) => g.id)).toEqual(['ok', 'unverified']);
  });

  it('filters by an archetype on either side', () => {
    expect(watchPlaylist(index, 'Glowing Dawn').map((g) => g.id)).toEqual(['ok', 'unverified']);
    expect(watchPlaylist(index, 'Chronicle').map((g) => g.id)).toEqual(['unverified']);
    expect(watchPlaylist(index, '').map((g) => g.id)).toEqual(['ok', 'unverified']);
  });
});

describe('watchGameUrl', () => {
  it('carries queue and position, and filter / move only when set', () => {
    expect(watchGameUrl('g1', 'cp8-mirror', 3, '')).toBe('/game/g1?watch=cp8-mirror&pos=3');
    expect(watchGameUrl('g1', 'cp8-mirror', 3, 'SW', 26)).toBe(
      '/game/g1?watch=cp8-mirror&pos=3&filter=SW&at=26',
    );
    expect(watchGameUrl('g1', 'q', 0, '', 0)).toBe('/game/g1?watch=q&pos=0');
  });
});
