import { test, expect } from '@playwright/test';
import type { GameState, PlayerState, TriggerChoice } from '../src/types/game';
import { GamePhase } from '../src/types/game';

const choices: TriggerChoice[] = [
  { triggerId: '1', actionId: 30, cardId: 'ST6-03', cardName: 'Gabumon', effectText: 'Draw 1 card.', timing: 'WhenAttacking', sourceLabel: 'Inherited · battle area 1', isOptional: false, isInherited: true, activationPrompt: false },
  { triggerId: '2', actionId: 31, cardId: 'ST6-06', cardName: 'Garurumon', effectText: 'You may gain 1 memory.', timing: 'WhenAttacking', sourceLabel: 'Effect · battle area 2', isOptional: true, isInherited: false, activationPrompt: false },
  { triggerId: '3', actionId: 32, cardId: 'ST6-03', cardName: 'Gabumon', effectText: 'Draw 1 card.', timing: 'WhenAttacking', sourceLabel: 'Inherited · battle area 3', isOptional: false, isInherited: true, activationPrompt: false },
];
const player = (id: number): PlayerState => ({ id, memory: 0, handCount: 0, handIds: [], handCards: [], securityCount: 5, securityIds: [], deckCount: 35, eggDeckCount: 4, battleAreaCount: 0, battleArea: [], breedingArea: null, trashIds: [] });

for (const theme of ['dark', 'light']) {
test(`ordered trigger plan uses current action IDs and reopens on new triggers (${theme})`, async ({ page }, testInfo) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript((theme) => {
    localStorage.setItem('access_token', 'test-token');
    localStorage.setItem('desktop.theme', theme);
  }, theme);
  await page.route('**/users/me', (route) => route.fulfill({ json: { id: 'test', username: 'Tester', email: '', roles: [] } }));
  await page.route('**/decks', (route) => route.fulfill({ json: [] }));
  await page.route('**/decks/folders', (route) => route.fulfill({ json: [] }));
  let state: GameState = {
    turnCount: 2, currentPhase: GamePhase.SelectEffectChoice, currentPlayer: 1, firstPlayer: 0,
    memoryGauge: 0, isGameOver: false, winner: null, player1: player(1), player2: player(2), revealedCards: [], pendingAttack: null,
    pendingSelection: { phase: GamePhase.SelectEffectChoice, kind: 'TriggerOrder', selectingPlayer: 1, validIndices: [30, 31, 32], isOptional: false, prompt: 'Choose next trigger', triggerChoices: choices },
  };
  const mask = () => {
    const result = new Array(2192).fill(0);
    for (const id of state.pendingSelection?.validIndices ?? []) result[id] = 1;
    if (state.pendingSelection?.isOptional) result[62] = 1;
    return result;
  };
  const response = () => ({ state, action_mask: mask(), logs: [], events: [], is_game_over: false, is_human_turn: true, agent_pending: false });
  const actions: number[] = [];
  await page.route('**/games/trigger-plan/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith('/state')) return route.fulfill({ json: state });
    if (path.endsWith('/action-mask')) return route.fulfill({ json: { action_mask: mask() } });
    if (path.endsWith('/decoded-actions')) return route.fulfill({ json: [] });
    if (path.endsWith('/actions')) {
      actions.push(route.request().postDataJSON().action);
      if (actions.length === 1) {
        state = { ...state, pendingSelection: { ...state.pendingSelection!, kind: 'Replacement', isOptional: true, validIndices: [59], triggerChoices: [{ ...choices[1]!, actionId: 59, activationPrompt: true }] } };
      } else {
        const next = actions.length === 2
          ? [choices[0]!, { ...choices[2]!, actionId: 31 }]
          : [{ ...choices[0]!, actionId: 30 }, { ...choices[1]!, triggerId: 'new', actionId: 31 }];
        state = { ...state, pendingSelection: { ...state.pendingSelection!, kind: 'TriggerOrder', isOptional: false, validIndices: [30, 31], triggerChoices: next } };
      }
    }
    return route.fulfill({ json: response() });
  });
  await page.goto('/game/trigger-plan');
  const dialog = page.getByRole('dialog', { name: 'Choose trigger resolution order' });
  await expect(dialog).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('trigger-picker-wide.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(dialog.getByRole('button', { name: 'Resolve in this order' })).toBeInViewport();
  await page.screenshot({ path: testInfo.outputPath('trigger-picker-narrow.png') });
  await page.setViewportSize({ width: 1280, height: 900 });
  await dialog.getByRole('group', { name: 'Activate Garurumon' }).getByRole('button', { name: 'No', exact: true }).click();
  await dialog.getByRole('button', { name: 'Reorder item 1' }).focus();
  await page.keyboard.press('ArrowDown');
  await dialog.getByRole('button', { name: 'Reorder item 2' }).focus();
  await page.keyboard.press('ArrowDown');
  await dialog.getByRole('button', { name: 'Resolve in this order' }).click();
  await expect.poll(() => actions).toEqual([31, 62, 31]);
  await expect(dialog.getByRole('list', { name: 'Pending trigger resolution order' })).toBeVisible();
  await expect(dialog.getByRole('button', { name: /Order Garurumon/ })).toBeVisible();
  expect(errors).toEqual([]);
});
}
