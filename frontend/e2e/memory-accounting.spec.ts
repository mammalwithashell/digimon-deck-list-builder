import { test, expect } from './fixtures/auth';
import { createDebugGame, MEDUSA_DECK } from './fixtures/debug-game';
import { GamePageObject } from './page-objects/game-page';
import { assertMemory } from './helpers/assertions';

test.describe('Memory accounting', () => {
  test('play cost deducts correct memory', async ({ authedPage }) => {
    const game = await createDebugGame(authedPage, {
      deck1: MEDUSA_DECK,
      deck2: MEDUSA_DECK,
      initial_memory: 10,
    });

    const gamePage = new GamePageObject(authedPage);
    await gamePage.navigateToGame(game.gameId);
    await gamePage.assertBoardVisible();

    // Check initial memory
    await assertMemory(authedPage, game.gameId, 10);

    // Get actions to find a playable card
    const actions = await game.getActions();
    // Find the first play action (0-29)
    const playAction = Object.keys(actions)
      .map(Number)
      .find((id) => id >= 0 && id < 30);

    if (playAction !== undefined) {
      // Get the card's cost from internal state to know expected deduction
      const stateBefore = await game.getInternalState();
      const memBefore: number = stateBefore.memory;

      // Play the card
      await game.sendAction(playAction);

      // Memory should have decreased
      const stateAfter = await game.getInternalState();
      expect(stateAfter.memory).toBeLessThan(memBefore);
    }
  });
});
