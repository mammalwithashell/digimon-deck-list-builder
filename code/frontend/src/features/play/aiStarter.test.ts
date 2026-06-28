import { describe, expect, it, vi, beforeEach } from 'vitest';

// Mock the gameApi + http client so we observe what createAiStarterGame sends.
const createGame = vi.fn((_arg: unknown) => Promise.resolve({ game_id: 'g1', seed: '7' }));
vi.mock('@/api/gameApi', () => ({
  createGame: (arg: unknown) => createGame(arg),
  normalizeSeedInput: (s: string | null) => s,
}));
vi.mock('@/api/client', () => ({ default: { post: vi.fn() } }));
// Force the Tauri-desktop path.
vi.stubGlobal('isTauri', true);

import { starterIndexFromSeed, createAiStarterGame } from './playApi';
import { STARTER_DECKS } from './starterDecks.generated';

beforeEach(() => createGame.mockClear());

describe('starterIndexFromSeed', () => {
  it('is deterministic for a given seed', () => {
    expect(starterIndexFromSeed('42', 6)).toBe(starterIndexFromSeed('42', 6));
  });
  it('stays within range', () => {
    for (const s of ['', '1', 'abc', '999999']) {
      const i = starterIndexFromSeed(s || null, 6);
      expect(i).toBeGreaterThanOrEqual(0);
      expect(i).toBeLessThan(6);
    }
  });
});

describe('createAiStarterGame', () => {
  it('sends player + a starter AI deck and falls back to greedy when no model', async () => {
    const res = await createAiStarterGame({
      deck: STARTER_DECKS[0]!,
      starterDecks: STARTER_DECKS,
      seed: '42',
    });
    expect(res.game_id).toBe('g1');
    expect(createGame).toHaveBeenCalledTimes(1);
    const arg = (createGame.mock.calls[0]?.[0] as unknown) as {
      deck1: string[];
      deck2: string[];
      player_kinds: string[];
      player_model_ids: (string | null)[];
    };
    // Player 1 is the chosen deck; player 2 is a (seed-derived) starter deck.
    expect(arg.deck1.length).toBe(54);
    expect(arg.deck2.length).toBe(54);
    // No model published in tests -> greedy CPU.
    expect(arg.player_kinds).toEqual(['human', 'greedy']);
    expect(arg.player_model_ids).toEqual([null, null]);
  });

  it('honors a specific opponent: AI plays the chosen starter deck', async () => {
    const aiPick = STARTER_DECKS[2]!; // a specific specialist deck
    const res = await createAiStarterGame({
      deck: STARTER_DECKS[0]!,
      starterDecks: STARTER_DECKS,
      seed: '42',
      opponent: aiPick.id,
    });
    expect(res.aiDeckName).toBe(aiPick.name);
    const arg = (createGame.mock.calls[0]?.[0] as unknown) as { deck2: string[] };
    // AI deck2 is the chosen deck, NOT the seed-derived one.
    expect(arg.deck2).toEqual([...aiPick.egg_deck, ...aiPick.main_deck]);
  });

  it('falls back to seed-derived deck when opponent id is unknown', async () => {
    const seedDeck = STARTER_DECKS[starterIndexFromSeed('42', STARTER_DECKS.length)]!;
    const res = await createAiStarterGame({
      deck: STARTER_DECKS[0]!,
      starterDecks: STARTER_DECKS,
      seed: '42',
      opponent: 'not_a_real_deck_id',
    });
    expect(res.aiDeckName).toBe(seedDeck.name);
  });
});
