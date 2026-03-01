import { test, expect } from './fixtures/auth';
import { createDebugGame, MEDUSA_DECK } from './fixtures/debug-game';
import { GamePageObject } from './page-objects/game-page';

test.describe('Game loads', () => {
  test('debug game loads in browser', async ({ authedPage }) => {
    const game = await createDebugGame(authedPage, {
      deck1: MEDUSA_DECK,
      deck2: MEDUSA_DECK,
      initial_memory: 5,
    });

    const gamePage = new GamePageObject(authedPage);
    await gamePage.navigateToGame(game.gameId);
    await gamePage.assertBoardVisible();
    await gamePage.assertPhaseIndicatorVisible();
  });
});
