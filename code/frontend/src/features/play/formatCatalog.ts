import type { QueueType } from '@/api/matchmaking';
import { listFormats } from '@/api/deckApi';
import type { DeckSummary } from '@/types/deck';

// Format ids match the engine's `game_mode` registry ids (see
// data/deck_formats.json). The play catalog mirrors exactly the registry's
// playable formats — no concept placeholders.
export type PlayFormatId =
  | 'standard'
  | 'no_restriction'
  | 'pauper'
  | 'eden'
  | 'eden_singleton'
  | 'starter';
export type OpponentMode = 'quick' | 'room' | 'bot' | 'ai_starter';

export interface PlayFormat {
  id: PlayFormatId;
  name: string;
  tagline: string;
  description: string;
  deckLabel: string;
  populationPct: number;
  enabled: boolean;
  disabledReason?: string;
}

export const ENGINE_STANDARD_ONLY_REASON = 'Not yet supported by the engine in this build';

// Purely-presentational metadata (tagline, rough population %) keyed by id.
// Names / descriptions / enabled-ness come from the engine registry via
// `loadPlayFormats()`; this map only supplies flavour the engine doesn't carry.
const PRESENTATION: Record<PlayFormatId, { tagline: string; populationPct: number }> = {
  standard: { tagline: 'The official ruleset', populationPct: 84 },
  no_restriction: { tagline: 'Every card. Every printing.', populationPct: 23 },
  pauper: { tagline: 'Commons and uncommons only', populationPct: 18 },
  eden: { tagline: 'Rotation-light, anomaly-gated', populationPct: 27 },
  eden_singleton: { tagline: 'EDEN, highlander', populationPct: 12 },
  starter: { tagline: 'Six official starter decks', populationPct: 0 },
};

// Static fallback used before the engine list resolves (and in environments
// where the engine call fails). Kept aligned with the registry; the live
// source of truth is `loadPlayFormats()`.
const STANDARD_FORMAT: PlayFormat = {
  id: 'standard',
  name: 'STANDARD',
  tagline: PRESENTATION.standard.tagline,
  description: '50-card decks, current banlist, mirrored memory gauge.',
  deckLabel: '50 cards',
  populationPct: PRESENTATION.standard.populationPct,
  enabled: true,
};

export const PLAY_FORMATS: PlayFormat[] = [
  STANDARD_FORMAT,
  {
    id: 'no_restriction',
    name: 'NO BANLIST',
    tagline: PRESENTATION.no_restriction.tagline,
    description: 'Standard shape with every card legal — no banned or restricted list.',
    deckLabel: '50 cards',
    populationPct: PRESENTATION.no_restriction.populationPct,
    enabled: true,
  },
  {
    id: 'pauper',
    name: 'PAUPER',
    tagline: PRESENTATION.pauper.tagline,
    description: 'Common and uncommon cards only, on the standard banlist.',
    deckLabel: '50 cards',
    populationPct: PRESENTATION.pauper.populationPct,
    enabled: true,
  },
  {
    id: 'eden',
    name: 'EDEN',
    tagline: PRESENTATION.eden.tagline,
    description: 'Common/uncommon plus the EDEN Anomaly Protocol, on EDEN’s banlist.',
    deckLabel: '50 cards',
    populationPct: PRESENTATION.eden.populationPct,
    enabled: true,
  },
  {
    id: 'eden_singleton',
    name: 'EDEN SINGLETON',
    tagline: PRESENTATION.eden_singleton.tagline,
    description: 'EDEN’s pool and banlist played highlander — one copy of any card.',
    deckLabel: '50 singleton',
    populationPct: PRESENTATION.eden_singleton.populationPct,
    enabled: true,
  },
];

/** Live play-format catalog sourced from the engine registry (`list_formats()`)
 *  — the same source the deck builder uses. Every registry format is playable;
 *  presentational flavour (tagline / population) is overlaid from `PRESENTATION`.
 *  No concept placeholders are appended. Falls back to the static `PLAY_FORMATS`
 *  if the engine call fails. */
export async function loadPlayFormats(): Promise<PlayFormat[]> {
  try {
    const engine = await listFormats();
    return engine.map((f) => ({
      id: f.id as PlayFormatId,
      name: f.name.toUpperCase(),
      tagline: PRESENTATION[f.id as PlayFormatId]?.tagline ?? '',
      description: f.description,
      deckLabel: f.singleton ? `${f.deck_size} singleton` : `${f.deck_size} cards`,
      populationPct: PRESENTATION[f.id as PlayFormatId]?.populationPct ?? 0,
      enabled: f.playable,
    }));
  } catch {
    return PLAY_FORMATS;
  }
}

export function getPlayFormat(formatId: string | null | undefined): PlayFormat {
  return PLAY_FORMATS.find((format) => format.id === formatId) ?? STANDARD_FORMAT;
}

export function canUseDeckForFormat(
  deck: DeckSummary,
  formatId: PlayFormatId,
): { ok: true } | { ok: false; reason: string } {
  const format = getPlayFormat(formatId);
  if (!format.enabled) {
    return { ok: false, reason: format.disabledReason ?? ENGINE_STANDARD_ONLY_REASON };
  }
  // A deck belongs to the format it was built and validated under. A deck
  // built for another format isn't legal here even if its card list happens
  // to fit — its `is_valid` was computed against a different ruleset.
  if (deck.game_mode !== formatId) {
    return { ok: false, reason: `Built for ${deck.game_mode}, not ${format.name}.` };
  }
  // Within the right format, legality (banlist / rarity / singleton) is
  // enforced by deck validation and reflected in `deck.is_valid`.
  if (!deck.is_valid) return { ok: false, reason: 'Deck must pass validation before queueing.' };
  if (deck.main_count !== 50 || deck.egg_count < 0 || deck.egg_count > 5) {
    return { ok: false, reason: 'Requires 50 main cards and 0-5 eggs.' };
  }
  return { ok: true };
}

export function formatToQueueType(_formatId: PlayFormatId): QueueType {
  return 'casual';
}
