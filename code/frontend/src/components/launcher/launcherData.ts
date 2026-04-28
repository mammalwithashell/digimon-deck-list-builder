import type { PatchNotesResponse, Release } from '@/api/patchNotesApi';
import type { DeckSummary } from '@/types/deck';

export interface LauncherDeckRow {
  id: string;
  name: string;
  href: string;
  countLabel: string;
  statusLabel: string;
  statusKind: 'legal' | 'draft';
  levelLabel: string;
  metaLabel: string;
  editedLabel: string;
}

export interface LauncherReleaseSummary {
  title: string;
  versionLabel: string;
  bullets: string[];
}

export function formatRelativeEdit(isoDate: string, now = new Date()): string {
  const edited = new Date(isoDate);
  const diffMs = Math.max(0, now.getTime() - edited.getTime());
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 10) return 'EDIT JUST NOW';
  if (minutes < 60) return `EDIT ${minutes}M AGO`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `EDIT ${hours}H AGO`;
  const days = Math.floor(hours / 24);
  if (days < 14) return `EDIT ${days}D AGO`;
  const weeks = Math.floor(days / 7);
  return `EDIT ${weeks}W AGO`;
}

export function formatCardCount(count: number | null | undefined): string {
  return typeof count === 'number' ? new Intl.NumberFormat('en-US').format(count) : '—';
}

export function countDraftDecks(decks: DeckSummary[]): number {
  return decks.filter((deck) => !deck.is_valid || deck.card_count < 50).length;
}

export function buildDeckRows(decks: DeckSummary[], now = new Date()): LauncherDeckRow[] {
  return decks
    .slice()
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 5)
    .map((deck) => {
      const legal = deck.is_valid && deck.card_count === 50;
      return {
        id: deck.id,
        name: deck.name.toUpperCase(),
        href: `/deckbuilder/${deck.id}`,
        countLabel: `${deck.card_count}/50`,
        statusLabel: legal ? 'BO3 LEGAL' : 'DRAFT',
        statusKind: legal ? 'legal' : 'draft',
        levelLabel: deck.meta_tier?.toUpperCase() ?? 'L?',
        metaLabel: deck.meta_archetype?.toUpperCase() ?? 'UNCLASSIFIED',
        editedLabel: formatRelativeEdit(deck.updated_at, now),
      };
    });
}

function latestRelease(releases: Release[]): Release | null {
  return (
    releases
      .slice()
      .sort((a, b) => new Date(b.release_date).getTime() - new Date(a.release_date).getTime())[0] ??
    null
  );
}

export function summarizeLatestRelease(
  patchNotes: PatchNotesResponse | null,
  now = new Date(),
): LauncherReleaseSummary {
  const release = patchNotes ? latestRelease(patchNotes.releases) : null;
  if (!release) {
    return {
      title: 'No release notes published',
      versionLabel: 'NEWS UNAVAILABLE',
      bullets: ['Server release feed is not reachable.'],
    };
  }
  const bullets = [...release.added, ...release.changed, ...release.fixed].slice(0, 3);
  return {
    title: release.title ?? `Version ${release.version}`,
    versionLabel: `v${release.version} · ${formatRelativeEdit(release.release_date, now).replace('EDIT ', '')}`,
    bullets: bullets.length > 0 ? bullets : ['Release notes are available in Patch Notes.'],
  };
}
