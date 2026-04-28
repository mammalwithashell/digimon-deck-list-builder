import { describe, expect, it } from 'vitest';
import {
  buildDeckRows,
  countDraftDecks,
  formatCardCount,
  formatRelativeEdit,
  summarizeLatestRelease,
} from './launcherData';
import type { DeckSummary } from '@/types/deck';
import type { PatchNotesResponse } from '@/api/patchNotesApi';

const decks: DeckSummary[] = [
  {
    id: 'deck-valid',
    name: 'Red Hybrid',
    description: '',
    game_mode: 'standard',
    is_valid: true,
    is_public: false,
    is_pinned: false,
    folder_id: null,
    card_count: 50,
    main_count: 50,
    egg_count: 0,
    tags: [],
    meta_tier: 'L6',
    meta_archetype: 'Red Aggro',
    created_at: '2026-04-20T12:00:00.000Z',
    updated_at: '2026-04-26T16:00:00.000Z',
  },
  {
    id: 'deck-draft',
    name: 'Green Insect Rush',
    description: '',
    game_mode: 'standard',
    is_valid: false,
    is_public: false,
    is_pinned: false,
    folder_id: null,
    card_count: 43,
    main_count: 43,
    egg_count: 0,
    tags: [],
    meta_tier: null,
    meta_archetype: null,
    created_at: '2026-04-10T12:00:00.000Z',
    updated_at: '2026-04-21T17:00:00.000Z',
  },
];

describe('launcherData', () => {
  it('builds deck rows with legal/draft labels and deckbuilder links', () => {
    const rows = buildDeckRows(decks, new Date('2026-04-26T18:00:00.000Z'));

    expect(rows).toEqual([
      {
        id: 'deck-valid',
        name: 'RED HYBRID',
        href: '/deckbuilder/deck-valid',
        countLabel: '50/50',
        statusLabel: 'BO3 LEGAL',
        statusKind: 'legal',
        levelLabel: 'L6',
        metaLabel: 'RED AGGRO',
        editedLabel: 'EDIT 2H AGO',
      },
      {
        id: 'deck-draft',
        name: 'GREEN INSECT RUSH',
        href: '/deckbuilder/deck-draft',
        countLabel: '43/50',
        statusLabel: 'DRAFT',
        statusKind: 'draft',
        levelLabel: 'L?',
        metaLabel: 'UNCLASSIFIED',
        editedLabel: 'EDIT 5D AGO',
      },
    ]);
  });

  it('counts draft decks from invalid or incomplete summaries', () => {
    expect(countDraftDecks(decks)).toBe(1);
  });

  it('formats card counts with thousands separators', () => {
    expect(formatCardCount(4127)).toBe('4,127');
    expect(formatCardCount(null)).toBe('—');
  });

  it('formats relative edit labels for recent, daily, and weekly edits', () => {
    const now = new Date('2026-04-26T18:00:00.000Z');
    expect(formatRelativeEdit('2026-04-26T17:54:00.000Z', now)).toBe('EDIT JUST NOW');
    expect(formatRelativeEdit('2026-04-26T14:00:00.000Z', now)).toBe('EDIT 4H AGO');
    expect(formatRelativeEdit('2026-04-24T18:00:00.000Z', now)).toBe('EDIT 2D AGO');
    expect(formatRelativeEdit('2026-04-10T18:00:00.000Z', now)).toBe('EDIT 2W AGO');
  });

  it('summarizes the latest release from patch notes', () => {
    const patchNotes: PatchNotesResponse = {
      known_issues: [],
      releases: [
        {
          id: 'old',
          version: '0.4.1',
          release_date: '2026-04-20',
          title: 'Old build',
          added: ['Old feature'],
          changed: [],
          fixed: [],
          created_at: '2026-04-20T00:00:00.000Z',
          updated_at: '2026-04-20T00:00:00.000Z',
        },
        {
          id: 'new',
          version: '0.4.2',
          release_date: '2026-04-24',
          title: 'Launcher polish',
          added: ['Desktop launcher'],
          changed: ['Guest boot flow'],
          fixed: [],
          created_at: '2026-04-24T00:00:00.000Z',
          updated_at: '2026-04-24T00:00:00.000Z',
        },
      ],
    };

    expect(summarizeLatestRelease(patchNotes, new Date('2026-04-26T18:00:00.000Z'))).toEqual({
      title: 'Launcher polish',
      versionLabel: 'v0.4.2 · 2D AGO',
      bullets: ['Desktop launcher', 'Guest boot flow'],
    });
  });
});
