import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { type Page } from '@playwright/test';
import { test, expect } from './fixtures/auth';
import { createDebugGame } from './fixtures/debug-game';
import { GamePageObject } from './page-objects/game-page';

/**
 * Visual capture of the in-game board on a staged mid-game scenario, for
 * each board style (classic / arena) and theme (dark / light): the board at
 * rest, the stack inspector on your own and on an opponent permanent, a hand
 * card hovered, and a hand card mid-drag over its "Play here" landing spot.
 *
 * Not a pixel-diff gate — it writes PNGs for a human to review:
 *   SCREENSHOT_DIR=/some/dir npx playwright test e2e/board-visual.spec.ts
 * (defaults to test-results/board-visual). Needs the hosted API on
 * API_BASE and the Vite dev server on BASE_URL, like the other e2e specs.
 */

const OUT_DIR = process.env.SCREENSHOT_DIR ?? join('test-results', 'board-visual');

const ST1_RED = [
  ...Array(4).fill('ST1-01'),
  ...['02', '03', '04', '05', '06', '08', '09', '12', '13', '14'].flatMap((n) => Array(4).fill(`ST1-${n}`)),
  ...['07', '10', '11', '15', '16'].flatMap((n) => Array(2).fill(`ST1-${n}`)),
];
const ST2_BLUE = [
  ...Array(4).fill('ST2-01'),
  ...['02', '03', '04', '05', '07', '08', '09', '12', '13', '14'].flatMap((n) => Array(4).fill(`ST2-${n}`)),
  ...['06', '10', '11', '15', '16'].flatMap((n) => Array(2).fill(`ST2-${n}`)),
];

/** Mid-game, your Main phase: a five-card WarGreymon stack, a Blocker, a
 *  suspended Digimon and a Tamer against a WereGarurumon line. */
const MIDGAME = {
  deck1: ST1_RED,
  deck2: ST2_BLUE,
  seed: 7,
  initial_memory: 3,
  phase: 'Main',
  turn: 6,
  first_player: 1,
  zones: {
    '1': {
      hand: ['ST1-05', 'ST1-08', 'ST1-13', 'ST1-15', 'ST1-10', 'ST1-16'],
      security: ['ST1-02', 'ST1-03', 'ST1-04', 'ST1-05'],
      trash: ['ST1-14', 'ST1-02', 'ST1-13'],
      field: [
        { stack: ['ST1-01', 'ST1-03', 'ST1-07', 'ST1-09', 'ST1-11'], is_suspended: false, turn_played: 0 },
        { stack: ['ST1-04', 'ST1-06'], is_suspended: false, turn_played: 0 },
        { stack: ['ST1-02'], is_suspended: true, turn_played: 0 },
        { stack: ['ST1-12'], is_suspended: false, turn_played: 0 },
      ],
      breeding: ['ST1-01'],
    },
    '2': {
      hand: ['ST2-03', 'ST2-05', 'ST2-13', 'ST2-14', 'ST2-09'],
      security: ['ST2-02', 'ST2-04', 'ST2-05'],
      trash: ['ST2-13', 'ST2-14'],
      field: [
        { stack: ['ST2-01', 'ST2-03', 'ST2-06', 'ST2-08'], is_suspended: false, turn_played: 0 },
        { stack: ['ST2-07'], is_suspended: false, turn_played: 0 },
        { stack: ['ST2-12'], is_suspended: false, turn_played: 0 },
      ],
      breeding: ['ST2-01'],
    },
  },
};

const COMBOS = [
  { style: 'classic', theme: 'dark' },
  { style: 'classic', theme: 'light' },
  { style: 'arena', theme: 'dark' },
  { style: 'arena', theme: 'light' },
] as const;

async function settle(page: Page) {
  // Card art comes from the CDN; wait until every image has loaded or failed.
  await page.waitForFunction(() => [...document.images].every((img) => img.complete), null, {
    timeout: 15_000,
  });
  await page.waitForTimeout(400);
}

test.describe('Board visual capture', () => {
  test.use({ viewport: { width: 1920, height: 1080 } });

  for (const { style, theme } of COMBOS) {
    test(`${style} / ${theme}`, async ({ authedPage: page }) => {
      mkdirSync(OUT_DIR, { recursive: true });
      const shot = (name: string) => join(OUT_DIR, `${style}-${theme}-${name}.png`);

      await page.addInitScript(
        ([s, t]) => {
          localStorage.setItem('gameplay.boardStyle', s);
          localStorage.setItem('desktop.theme', t);
        },
        [style, theme],
      );
      const game = await createDebugGame(page, MIDGAME);
      await new GamePageObject(page).navigateToGame(game.gameId);
      const board = page.getByTestId('game-board');
      await expect(board).toHaveAttribute('data-board-style', style);
      await expect(page.getByTestId('field-slot-p1-0')).toBeVisible();
      await settle(page);
      await page.screenshot({ path: shot('board') });

      // Stack inspector on your WarGreymon: opens over the opponent's half,
      // with Attack available.
      await page.getByTestId('field-slot-p1-0').locator('.ib-permanent-slot').click({ button: 'right' });
      await expect(page.getByRole('dialog')).toBeVisible();
      await expect(page.getByTestId('inspector-action-attack')).toBeVisible();
      await settle(page);
      await page.screenshot({ path: shot('inspect-own') });
      await page.keyboard.press('Escape');

      // Stack inspector on the opponent's WereGarurumon: opens over your half.
      await page.getByTestId('field-slot-p2-0').locator('.ib-permanent-slot').click({ button: 'right' });
      await expect(page.getByRole('dialog')).toHaveAttribute('data-half', 'lower');
      await settle(page);
      await page.screenshot({ path: shot('inspect-opponent') });
      await page.keyboard.press('Escape');

      // Hover a hand card: it lifts out of the fan.
      const handCards = page.locator('.ib-hand-zone--you .ib-hand-card');
      const hovered = await handCards.nth(2).boundingBox();
      expect(hovered).not.toBeNull();
      await page.mouse.move(hovered!.x + hovered!.width / 2, hovered!.y + 30);
      await page.waitForTimeout(300);
      await page.screenshot({ path: shot('hand-hover') });

      // Drag Birdramon toward the field: the "Play here" landing spot appears.
      const dragged = await handCards.nth(0).boundingBox();
      await page.mouse.move(dragged!.x + 30, dragged!.y + 40);
      await page.mouse.down();
      for (let i = 1; i <= 15; i++) {
        await page.mouse.move(dragged!.x + 30 + i * 12, dragged!.y + 40 - i * 22);
      }
      await expect(page.locator('.ib-battle-slot--landing')).toBeVisible();
      await page.waitForTimeout(200);
      await page.screenshot({ path: shot('drag-landing') });
      await page.mouse.up();
    });
  }
});
