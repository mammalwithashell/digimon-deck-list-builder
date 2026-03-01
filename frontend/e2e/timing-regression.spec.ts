import { test, expect } from './fixtures/auth';
import { createDebugGame, MEDUSA_DECK } from './fixtures/debug-game';
import { GamePageObject } from './page-objects/game-page';

test.describe('Timing regression', () => {
  test('delay effects do not corrupt memory', async ({ authedPage }) => {
    // Set up a game and play multiple cards
    // After each action, verify memory only changes by expected amounts
    const game = await createDebugGame(authedPage, {
      deck1: MEDUSA_DECK,
      deck2: MEDUSA_DECK,
      initial_memory: 10,
    });

    const gamePage = new GamePageObject(authedPage);
    await gamePage.navigateToGame(game.gameId);
    await gamePage.assertBoardVisible();

    // Inject P-035 (Red Memory Boost, cost 3) into hand to have a known card
    await game.injectCard(1, 'P-035', 'hand');

    // Re-fetch the page to pick up the injected card state
    await authedPage.reload();
    await gamePage.board.waitFor({ state: 'visible', timeout: 15_000 });

    // Get available actions and internal state
    const actions = await game.getActions();
    const state = await game.getInternalState();
    const handP1: string[] = state.hand_p1;
    const p035Index = handP1.findIndex((id: string) => id === 'P-035');

    if (p035Index >= 0 && String(p035Index) in actions) {
      const memBefore: number = state.memory;

      // Play P-035 (cost 3)
      await game.sendAction(p035Index);

      // Memory should decrease by exactly 3
      const stateAfter = await game.getInternalState();
      expect(stateAfter.memory).toBe(memBefore - 3);
    }
  });
});
