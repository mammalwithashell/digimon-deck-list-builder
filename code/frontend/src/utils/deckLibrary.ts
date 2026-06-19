import type { DigimonCardData } from '@/types/cards';
import type { DeckFormat, DeckResponse, DeckSummary } from '@/types/deck';

export type DeckSortKey = 'recent' | 'name' | 'validity' | 'count';

export interface LibraryFilters {
  activeFolder: string;
  search: string;
  sort: DeckSortKey;
  /** Selected format (`game_mode`) to narrow to, or `'all'` / undefined for no
   *  format filtering. */
  format?: string;
}

/** One entry in the library's format filter (sidebar + toolbar). */
export interface FormatBucket {
  id: string;
  label: string;
  count: number;
}

export interface DeckAnalytics {
  colors: { name: string; count: number; pct: number }[];
  levelCurve: number[];
  averagePlayCost: string;
  highestLevel: number | null;
}

export function filterAndSortDecks(decks: DeckSummary[], filters: LibraryFilters): DeckSummary[] {
  const search = filters.search.trim().toLowerCase();
  const format = filters.format ?? 'all';
  let out = decks;

  if (filters.activeFolder === 'pinned') {
    out = out.filter((deck) => deck.is_pinned);
  } else if (filters.activeFolder !== 'all') {
    out = out.filter((deck) => deck.folder_id === filters.activeFolder);
  }

  if (format !== 'all') {
    out = out.filter((deck) => deck.game_mode === format);
  }

  if (search) {
    out = out.filter((deck) => {
      const haystack = [
        deck.name,
        deck.description,
        deck.meta_archetype,
        deck.meta_tier,
        deck.game_mode,
        ...(deck.tags ?? []),
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();
      return haystack.includes(search);
    });
  }

  return [...out].sort((a, b) => {
    if (filters.sort === 'name') return a.name.localeCompare(b.name);
    if (filters.sort === 'validity') return Number(b.is_valid) - Number(a.is_valid);
    if (filters.sort === 'count') return b.card_count - a.card_count;
    return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
  });
}

export function formatRelativeTime(value: string): string {
  const timestamp = new Date(value).getTime();
  if (Number.isNaN(timestamp)) return 'unknown';
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d`;
  const months = Math.floor(days / 30);
  return `${months}mo`;
}

/** Whole-library deck counts keyed by `game_mode` (folder-independent). */
export function countByFormat(decks: DeckSummary[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const deck of decks) {
    counts.set(deck.game_mode, (counts.get(deck.game_mode) ?? 0) + 1);
  }
  return counts;
}

/** Resolve a `game_mode` to its engine-registry display name, falling back to
 *  the raw id (uppercased) for legacy/unknown modes. */
export function formatLabel(gameMode: string, formats: DeckFormat[]): string {
  return formats.find((format) => format.id === gameMode)?.name ?? gameMode.toUpperCase();
}

/** Ordered format buckets for the library filter: registry formats that have at
 *  least one deck (in registry order) first, then any non-registry `game_mode`
 *  present among the decks (sorted). Each carries its display label and count. */
export function deriveFormatBuckets(decks: DeckSummary[], formats: DeckFormat[]): FormatBucket[] {
  const counts = countByFormat(decks);
  const buckets: FormatBucket[] = [];
  const seen = new Set<string>();

  for (const format of formats) {
    const count = counts.get(format.id) ?? 0;
    if (count > 0) {
      buckets.push({ id: format.id, label: format.name, count });
      seen.add(format.id);
    }
  }

  for (const id of Array.from(counts.keys()).filter((mode) => !seen.has(mode)).sort()) {
    buckets.push({ id, label: formatLabel(id, formats), count: counts.get(id) ?? 0 });
  }

  return buckets;
}

export function groupCardIds(ids: string[], altArts: boolean[] = []): Map<string, number> {
  const counts = new Map<string, number>();
  ids.forEach((cardId, index) => {
    const key = `${cardId}|${altArts[index] ? '1' : '0'}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  });
  return counts;
}

export function deckToExportText(deck: DeckResponse, cards: Map<string, DigimonCardData>): string {
  const lines: string[] = [];
  for (const [key, count] of groupCardIds(deck.egg_deck, deck.egg_deck_alt_arts).entries()) {
    const cardId = key.split('|')[0]!;
    lines.push(`${count} ${cardId} ${cards.get(cardId)?.name ?? ''}`.trim());
  }
  for (const [key, count] of groupCardIds(deck.main_deck, deck.main_deck_alt_arts).entries()) {
    const cardId = key.split('|')[0]!;
    lines.push(`${count} ${cardId} ${cards.get(cardId)?.name ?? ''}`.trim());
  }
  return lines.join('\n');
}

export function buildDeckAnalytics(deck: DeckResponse | null, cards: Map<string, DigimonCardData>): DeckAnalytics {
  if (!deck) {
    return { colors: [], levelCurve: Array(8).fill(0), averagePlayCost: '-', highestLevel: null };
  }

  const colorCounts = new Map<string, number>();
  const levelCurve = Array(8).fill(0) as number[];
  let totalCardsWithCost = 0;
  let totalCost = 0;
  let highestLevel: number | null = null;

  for (const cardId of [...deck.main_deck, ...deck.egg_deck]) {
    const card = cards.get(cardId);
    if (!card) continue;

    const colors = [card.color, card.color2].filter(Boolean) as string[];
    for (const color of colors.length ? colors : ['Unknown']) {
      colorCounts.set(color, (colorCounts.get(color) ?? 0) + 1);
    }

    const level = Number.parseInt(card.level, 10);
    if (!Number.isNaN(level)) {
      highestLevel = highestLevel === null ? level : Math.max(highestLevel, level);
      const bucket = Math.max(0, Math.min(7, level));
      levelCurve[bucket] = (levelCurve[bucket] ?? 0) + 1;
    }

    const playCost = Number.parseInt(card.play_cost, 10);
    if (!Number.isNaN(playCost)) {
      totalCardsWithCost += 1;
      totalCost += playCost;
    }
  }

  const colorTotal = Array.from(colorCounts.values()).reduce((sum, count) => sum + count, 0) || 1;
  const colors = Array.from(colorCounts.entries())
    .map(([name, count]) => ({ name, count, pct: Math.round((count / colorTotal) * 100) }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));

  return {
    colors,
    levelCurve,
    averagePlayCost: totalCardsWithCost ? (totalCost / totalCardsWithCost).toFixed(1) : '-',
    highestLevel,
  };
}

export function deckInitials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => part[0])
    .join('')
    .slice(0, 2)
    .toUpperCase();
}
