import type { QueueType } from '@/api/matchmaking';
import type { DeckSummary } from '@/types/deck';

export type PlayFormatId = 'standard' | 'titan' | 'edh' | 'nobanlist' | 'draft' | 'tutorial';
export type OpponentMode = 'quick' | 'room' | 'bot';

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

export const ENGINE_STANDARD_ONLY_REASON = 'Engine supports Standard only in this build';

export const PLAY_FORMATS: PlayFormat[] = [
  {
    id: 'standard',
    name: 'STANDARD',
    tagline: 'The official ruleset',
    description: '50-card decks, current banlist, mirrored memory gauge.',
    deckLabel: '50 cards',
    populationPct: 84,
    enabled: true,
  },
  {
    id: 'titan',
    name: 'TITAN',
    tagline: 'Bigger gauges. Bigger threats.',
    description: '75-card deck concept from the mock; disabled until Rules support lands.',
    deckLabel: '75 cards',
    populationPct: 42,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'edh',
    name: 'EDH',
    tagline: 'One herald, one of each, four players',
    description: '100-card singleton concept from the mock; disabled until multiplayer Rules support lands.',
    deckLabel: '100 singleton',
    populationPct: 67,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'nobanlist',
    name: 'NO BANLIST',
    tagline: 'Every card. Every printing.',
    description: 'Standard shape without restrictions; disabled until validator support lands.',
    deckLabel: '50 cards',
    populationPct: 23,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'draft',
    name: 'DRAFT',
    tagline: 'Build from a pod',
    description: 'Limited mode concept from the mock; disabled until draft pool support lands.',
    deckLabel: '40 cards',
    populationPct: 12,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'tutorial',
    name: 'TUTORIAL',
    tagline: 'Practice the board',
    description: 'Guided game concept from the mock; disabled until scripted tutorial support lands.',
    deckLabel: 'Starter',
    populationPct: 9,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
];

export function getPlayFormat(formatId: string | null | undefined): PlayFormat {
  return PLAY_FORMATS.find((format) => format.id === formatId) ?? PLAY_FORMATS[0];
}

export function canUseDeckForFormat(
  deck: DeckSummary,
  formatId: PlayFormatId,
): { ok: true } | { ok: false; reason: string } {
  const format = getPlayFormat(formatId);
  if (!format.enabled) {
    return { ok: false, reason: format.disabledReason ?? ENGINE_STANDARD_ONLY_REASON };
  }
  if (!deck.is_valid) return { ok: false, reason: 'Deck must pass validation before queueing.' };
  if (deck.main_count !== 50 || deck.egg_count < 0 || deck.egg_count > 5) {
    return { ok: false, reason: 'Standard requires 50 main cards and 0-5 eggs.' };
  }
  return { ok: true };
}

export function formatToQueueType(formatId: PlayFormatId): QueueType {
  if (formatId === 'standard') return 'casual';
  return 'casual';
}
