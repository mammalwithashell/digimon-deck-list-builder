import type { DeckResponse, DeckSummary } from '@/types/deck';
import { canUseDeckForFormat, type PlayFormatId } from './formatCatalog';

export type BotOpponentDeckCandidate =
  | {
      id: string;
      name: string;
      source: 'starter';
      deck: DeckResponse;
    }
  | {
      id: string;
      name: string;
      source: 'saved';
      deckId: string;
    };

interface StarterDeckSeed {
  id: string;
  name: string;
  egg_deck: string[];
  main_deck: string[];
}

const STARTER_SEEDS: StarterDeckSeed[] = [
  {
    id: 'starter-st1',
    name: 'ST-1 Gaia Red',
    egg_deck: ['ST1-01', 'ST1-01', 'ST1-01', 'ST1-01'],
    main_deck: ['ST1-02', 'ST1-02', 'ST1-02', 'ST1-02', 'ST1-03', 'ST1-03', 'ST1-03', 'ST1-03', 'ST1-04', 'ST1-04', 'ST1-04', 'ST1-04', 'ST1-05', 'ST1-05', 'ST1-05', 'ST1-05', 'ST1-06', 'ST1-06', 'ST1-06', 'ST1-06', 'ST1-07', 'ST1-07', 'ST1-08', 'ST1-08', 'ST1-08', 'ST1-08', 'ST1-09', 'ST1-09', 'ST1-09', 'ST1-09', 'ST1-10', 'ST1-10', 'ST1-11', 'ST1-11', 'ST1-12', 'ST1-12', 'ST1-12', 'ST1-12', 'ST1-13', 'ST1-13', 'ST1-13', 'ST1-13', 'ST1-14', 'ST1-14', 'ST1-14', 'ST1-14', 'ST1-15', 'ST1-15', 'ST1-16', 'ST1-16'],
  },
  {
    id: 'starter-st2',
    name: 'ST-2 Cocytus Blue',
    egg_deck: ['ST2-01', 'ST2-01', 'ST2-01', 'ST2-01'],
    main_deck: ['ST2-02', 'ST2-02', 'ST2-02', 'ST2-02', 'ST2-03', 'ST2-03', 'ST2-03', 'ST2-03', 'ST2-04', 'ST2-04', 'ST2-04', 'ST2-04', 'ST2-05', 'ST2-05', 'ST2-05', 'ST2-05', 'ST2-06', 'ST2-06', 'ST2-07', 'ST2-07', 'ST2-07', 'ST2-07', 'ST2-08', 'ST2-08', 'ST2-08', 'ST2-08', 'ST2-09', 'ST2-09', 'ST2-09', 'ST2-09', 'ST2-10', 'ST2-10', 'ST2-11', 'ST2-11', 'ST2-12', 'ST2-12', 'ST2-12', 'ST2-12', 'ST2-13', 'ST2-13', 'ST2-13', 'ST2-13', 'ST2-14', 'ST2-14', 'ST2-14', 'ST2-14', 'ST2-15', 'ST2-15', 'ST2-16', 'ST2-16'],
  },
  {
    id: 'starter-st3',
    name: "ST-3 Heaven's Yellow",
    egg_deck: ['ST3-01', 'ST3-01', 'ST3-01', 'ST3-01'],
    main_deck: ['ST3-02', 'ST3-02', 'ST3-02', 'ST3-02', 'ST3-03', 'ST3-03', 'ST3-03', 'ST3-03', 'ST3-04', 'ST3-04', 'ST3-04', 'ST3-04', 'ST3-05', 'ST3-05', 'ST3-06', 'ST3-06', 'ST3-06', 'ST3-06', 'ST3-07', 'ST3-07', 'ST3-07', 'ST3-07', 'ST3-08', 'ST3-08', 'ST3-08', 'ST3-08', 'ST3-09', 'ST3-09', 'ST3-09', 'ST3-09', 'ST3-10', 'ST3-10', 'ST3-11', 'ST3-11', 'ST3-12', 'ST3-12', 'ST3-12', 'ST3-12', 'ST3-13', 'ST3-13', 'ST3-13', 'ST3-13', 'ST3-14', 'ST3-14', 'ST3-15', 'ST3-15', 'ST3-15', 'ST3-15', 'ST3-16', 'ST3-16'],
  },
  {
    id: 'starter-st4',
    name: 'ST-4 Giga Green',
    egg_deck: ['ST4-01', 'ST4-01', 'ST4-01', 'ST4-01'],
    main_deck: ['ST4-02', 'ST4-02', 'ST4-02', 'ST4-02', 'ST4-03', 'ST4-03', 'ST4-03', 'ST4-03', 'ST4-04', 'ST4-04', 'ST4-04', 'ST4-04', 'ST4-05', 'ST4-05', 'ST4-05', 'ST4-05', 'ST4-06', 'ST4-06', 'ST4-06', 'ST4-06', 'ST4-07', 'ST4-07', 'ST4-07', 'ST4-07', 'ST4-08', 'ST4-08', 'ST4-09', 'ST4-09', 'ST4-09', 'ST4-09', 'ST4-10', 'ST4-10', 'ST4-10', 'ST4-10', 'ST4-11', 'ST4-11', 'ST4-12', 'ST4-12', 'ST4-13', 'ST4-13', 'ST4-14', 'ST4-14', 'ST4-14', 'ST4-14', 'ST4-15', 'ST4-15', 'ST4-15', 'ST4-15', 'ST4-16', 'ST4-16'],
  },
  {
    id: 'starter-st5',
    name: 'ST-5 Machine Black',
    egg_deck: ['ST5-01', 'ST5-01', 'ST5-01', 'ST5-01'],
    main_deck: ['ST5-02', 'ST5-02', 'ST5-02', 'ST5-02', 'ST5-03', 'ST5-03', 'ST5-03', 'ST5-03', 'ST5-04', 'ST5-04', 'ST5-04', 'ST5-04', 'ST5-05', 'ST5-05', 'ST5-05', 'ST5-05', 'ST5-06', 'ST5-06', 'ST5-06', 'ST5-06', 'ST5-07', 'ST5-07', 'ST5-07', 'ST5-07', 'ST5-08', 'ST5-08', 'ST5-09', 'ST5-09', 'ST5-09', 'ST5-09', 'ST5-10', 'ST5-10', 'ST5-10', 'ST5-10', 'ST5-11', 'ST5-11', 'ST5-12', 'ST5-12', 'ST5-13', 'ST5-13', 'ST5-14', 'ST5-14', 'ST5-14', 'ST5-14', 'ST5-15', 'ST5-15', 'ST5-15', 'ST5-15', 'ST5-16', 'ST5-16'],
  },
  {
    id: 'starter-st6',
    name: 'ST-6 Venomous Violet',
    egg_deck: ['ST6-01', 'ST6-01', 'ST6-01', 'ST6-01'],
    main_deck: ['ST6-02', 'ST6-02', 'ST6-02', 'ST6-02', 'ST6-03', 'ST6-03', 'ST6-03', 'ST6-03', 'ST6-04', 'ST6-04', 'ST6-04', 'ST6-04', 'ST6-05', 'ST6-05', 'ST6-06', 'ST6-06', 'ST6-06', 'ST6-06', 'ST6-07', 'ST6-07', 'ST6-07', 'ST6-07', 'ST6-08', 'ST6-08', 'ST6-08', 'ST6-08', 'ST6-09', 'ST6-09', 'ST6-09', 'ST6-09', 'ST6-10', 'ST6-10', 'ST6-11', 'ST6-11', 'ST6-11', 'ST6-11', 'ST6-12', 'ST6-12', 'ST6-13', 'ST6-13', 'ST6-14', 'ST6-14', 'ST6-14', 'ST6-14', 'ST6-15', 'ST6-15', 'ST6-15', 'ST6-15', 'ST6-16', 'ST6-16'],
  },
];

function starterToDeck(seed: StarterDeckSeed): DeckResponse {
  return {
    id: seed.id,
    owner_id: 'builtin',
    folder_id: null,
    name: seed.name,
    description: 'Built-in starter deck for local bot practice.',
    game_mode: 'standard',
    main_deck: seed.main_deck,
    egg_deck: seed.egg_deck,
    main_deck_alt_arts: [],
    egg_deck_alt_arts: [],
    commander_id: null,
    is_valid: true,
    validation_errors: [],
    is_public: false,
    is_pinned: false,
    tags: ['starter', 'bot'],
    meta_tier: null,
    meta_archetype: seed.name,
    created_at: '2026-06-15T00:00:00.000Z',
    updated_at: '2026-06-15T00:00:00.000Z',
  };
}

export const BUILT_IN_STARTER_BOT_DECKS: DeckResponse[] = STARTER_SEEDS.map(starterToDeck);

export function buildBotOpponentDeckPool(
  savedDecks: DeckSummary[],
  selectedDeckId: string | null | undefined,
  formatId: PlayFormatId,
): BotOpponentDeckCandidate[] {
  const starterCandidates: BotOpponentDeckCandidate[] = BUILT_IN_STARTER_BOT_DECKS.map((deck) => ({
    id: deck.id,
    name: deck.name,
    source: 'starter',
    deck,
  }));
  const savedCandidates: BotOpponentDeckCandidate[] = savedDecks
    .filter((deck) => deck.id !== selectedDeckId && canUseDeckForFormat(deck, formatId).ok)
    .map((deck) => ({
      id: deck.id,
      name: deck.name,
      source: 'saved',
      deckId: deck.id,
    }));
  return [...starterCandidates, ...savedCandidates];
}

export function chooseBotOpponentCandidate(
  candidates: BotOpponentDeckCandidate[],
  random: () => number = Math.random,
): BotOpponentDeckCandidate {
  if (candidates.length === 0) {
    throw new Error('No bot opponent decks are available.');
  }
  const index = Math.min(candidates.length - 1, Math.floor(random() * candidates.length));
  const candidate = candidates[index];
  if (!candidate) {
    throw new Error('No bot opponent decks are available.');
  }
  return candidate;
}

export async function chooseBotOpponentDeck({
  savedDecks,
  selectedDeckId,
  formatId,
  getDeck,
  random,
}: {
  savedDecks: DeckSummary[];
  selectedDeckId: string | null | undefined;
  formatId: PlayFormatId;
  getDeck: (deckId: string) => Promise<DeckResponse>;
  random?: () => number;
}): Promise<DeckResponse> {
  const candidate = chooseBotOpponentCandidate(
    buildBotOpponentDeckPool(savedDecks, selectedDeckId, formatId),
    random,
  );
  if (candidate.source === 'starter') return candidate.deck;
  return getDeck(candidate.deckId);
}
