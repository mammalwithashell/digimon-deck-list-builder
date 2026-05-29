import { test, expect } from './fixtures/auth';
import { createDebugGame, MEDUSA_DECK } from './fixtures/debug-game';
import { GamePageObject } from './page-objects/game-page';

test.describe('Digivolution mechanics', () => {
  test('digivolution draws 1 card and costs evo cost', async ({ authedPage }) => {
    // Create game, hatch an egg, move to field, then digivolve
    const game = await createDebugGame(authedPage, {
      deck1: MEDUSA_DECK,
      deck2: MEDUSA_DECK,
      initial_memory: 10,
    });

    const gamePage = new GamePageObject(authedPage);
    await gamePage.navigateToGame(game.gameId);
    await gamePage.assertBoardVisible();

    // Step 1: Hatch (action 60)
    await game.sendAction(60);

    // Step 2: Move to field (action 61)
    await game.sendAction(61);

    // Check state - we should have a Lv.2 on the field
    let state = await game.getInternalState();
    const handCountBefore: number = state.hand_p1.length;
    const memBefore: number = state.memory;

    // Step 3: Check if we can digivolve - get actions
    const actions = await game.getActions();
    // Find a digivolve action (400+)
    const digiAction = actions.find((id) => id >= 400 && id < 1000);

    if (digiAction !== undefined) {
      await game.sendAction(digiAction);

      state = await game.getInternalState();
      // Should have drawn 1 card (digivolution bonus)
      expect(state.hand_p1.length).toBe(handCountBefore + 1);
      // Memory should have decreased by evo cost (not play cost)
      expect(state.memory).toBeLessThan(memBefore);
    }
  });
});
