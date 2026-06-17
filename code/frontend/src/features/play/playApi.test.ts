import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DeckFormat } from '@/types/deck';

const listDeckFormatsMock = vi.fn<() => Promise<DeckFormat[]>>();
const clientGetMock = vi.fn();

vi.mock('@/api/deckApi', () => ({
  listFormats: listDeckFormatsMock,
}));

vi.mock('@/api/client', () => ({
  default: {
    get: clientGetMock,
  },
}));

const registryFormats: DeckFormat[] = [
  {
    id: 'standard',
    name: 'Standard',
    description: '50-card decks, current banlist, mirrored memory gauge.',
    deck_size: 50,
    egg_max: 5,
    rarity_policy: 'all',
    singleton: false,
    default_max_copies: 4,
    playable: true,
  },
  {
    id: 'no_restriction',
    name: 'No Banlist',
    description: 'Standard shape with every card legal.',
    deck_size: 50,
    egg_max: 5,
    rarity_policy: 'all',
    singleton: false,
    default_max_copies: 4,
    playable: true,
  },
  {
    id: 'pauper',
    name: 'Pauper',
    description: 'Common and uncommon cards only.',
    deck_size: 50,
    egg_max: 5,
    rarity_policy: 'common_uncommon',
    singleton: false,
    default_max_copies: 4,
    playable: true,
  },
  {
    id: 'eden',
    name: 'EDEN',
    description: 'Common/uncommon plus the EDEN Anomaly Protocol.',
    deck_size: 50,
    egg_max: 5,
    rarity_policy: 'eden_anomaly',
    singleton: false,
    default_max_copies: 4,
    playable: true,
  },
  {
    id: 'eden_singleton',
    name: 'EDEN Singleton',
    description: 'EDEN pool and banlist, one copy of any card.',
    deck_size: 50,
    egg_max: 5,
    rarity_policy: 'eden_anomaly',
    singleton: true,
    default_max_copies: 1,
    playable: true,
  },
];

describe('playApi.listFormats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listDeckFormatsMock.mockResolvedValue(registryFormats);
    clientGetMock.mockResolvedValue({
      data: [
        {
          id: 'standard',
          name: 'STANDARD',
          tagline: 'The official ruleset',
          description: '50-card decks, current banlist, mirrored memory gauge.',
          deck_label: '50 cards',
          population_pct: 84,
          enabled: true,
        },
      ],
    });
  });

  it('uses the engine registry-backed deck format list for the play queue', async () => {
    const { listFormats } = await import('./playApi');

    const formats = await listFormats();

    expect(listDeckFormatsMock).toHaveBeenCalledOnce();
    expect(clientGetMock).not.toHaveBeenCalledWith('/formats');
    expect(formats.filter((format) => format.enabled).map((format) => format.id)).toEqual([
      'standard',
      'no_restriction',
      'pauper',
      'eden',
      'eden_singleton',
    ]);
    expect(formats.find((format) => format.id === 'eden')).toMatchObject({
      name: 'EDEN',
      tagline: 'Rotation-light, anomaly-gated',
      deckLabel: '50 cards',
      enabled: true,
    });
    expect(formats.find((format) => format.id === 'eden_singleton')).toMatchObject({
      deckLabel: '50 singleton',
      enabled: true,
    });
  });
});
