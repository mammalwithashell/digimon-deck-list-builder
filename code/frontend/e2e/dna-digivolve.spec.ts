import { test, expect } from '@playwright/test';
import { loadScenario, stageScenario } from './fixtures/debug-game';

/**
 * DNA digivolve — rendered-UI e2e (add-ui-scenario-test-substrate).
 *
 * This is the DOM-level half of the harness: it stages the board via the
 * `/debug` router, then drives the ACTUAL rendered game board in the
 * browser and asserts the player can reach DNA digivolve.
 *
 * Reproduces the reported bug: Paildramon (AD1-011) in hand with ExVeemon
 * (Blue Lv.4) + Stingmon (Green Lv.4) on the field. The engine offers both
 * a regular digivolve (over one material) and a DNA digivolve (combining
 * both); the UI must surface the *choice* rather than forcing the regular
 * path. Requires the frontend (desktop-mode Vite, default :5173) and
 * FastAPI (:8000) running.
 */

const FRONTEND = process.env.FRONTEND_URL ?? 'http://localhost:5173';
const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

test.describe('DNA digivolve — rendered UI', () => {
  test('Paildramon offers DNA Digivolve over ExVeemon + Stingmon', async ({ page }) => {
    const fixture = loadScenario('dna-paildramon-hand');
    const game = await stageScenario(page, fixture);
    try {
      // ── Engine layer ──────────────────────────────────────────────
      // DNA digivolve (action 63) AND regular digivolve (400) both legal.
      const eng = await game.evaluate(fixture.assertions.engine ?? []);
      expect(eng.all_passed, JSON.stringify(eng.results, null, 2)).toBe(true);

      // ── UI layer ──────────────────────────────────────────────────
      // Provision a guest token and seed it so AuthGuard passes on first
      // render. `authStore` initializes `isAuthenticated` synchronously from
      // `localStorage.access_token`, so seeding it before navigation avoids
      // the hydrate redirect race.
      const guest = await page.request.post(`${API_BASE}/auth/guest`);
      const { access_token: token } = await guest.json();
      await page.addInitScript((tok) => {
        localStorage.setItem('access_token', tok as string);
      }, token);
      await page.goto(`${FRONTEND}/game/${game.gameId}`);

      // The staged board renders; Paildramon is the player's hand card.
      const handCard = page.getByRole('button', { name: /AD1-011/ });
      await expect(handCard).toBeVisible();
      await handCard.click();

      // The method-choice modal must offer DNA Digivolve. (The bug was the
      // UI dispatching the regular-digivolve range and never offering DNA.)
      const dnaBtn = page.getByRole('button', { name: 'DNA Digivolve', exact: true });
      await expect(dnaBtn).toBeVisible();
      // Regular digivolve is also offered — the user gets the choice.
      await expect(page.getByRole('button', { name: 'Digivolve', exact: true })).toBeVisible();

      await dnaBtn.click();

      // Choosing DNA enters material selection — DCGO's "Select ... DNA
      // material" prompt, driven by the engine's pending_selection.
      await expect(page.getByText(/first DNA material/i)).toBeVisible();

      // Complete the pick by CLICKING the two materials on the board
      // (ExVeemon at p1 slot 0, Stingmon at p1 slot 1). This exercises the
      // SelectMaterial board-click wiring (raw battle_area index dispatch).
      await page.getByTestId('field-slot-p1-0').click();
      await expect(page.getByText(/second DNA material/i)).toBeVisible();
      // Wait for the second-stage mask to settle in the store (action 1 =
      // the remaining material is legal; action 0 no longer is) so the next
      // click isn't fired mid-transition.
      await page.waitForFunction(() => {
        const s = (window as unknown as { __dev__?: { useGameStore: { getState: () => { actionMask?: number[] } } } }).__dev__?.useGameStore.getState();
        const m = s?.actionMask ?? [];
        return m[1] === 1 && m[0] !== 1;
      });
      await page.getByTestId('field-slot-p1-1').click();

      // The second material pick dispatches asynchronously (HTTP). Wait for
      // the merge to land before asserting — `.click()` resolves on the DOM
      // event, not on the engine round-trip.
      await page.waitForFunction(() => {
        const s = (window as unknown as { __dev__?: { useGameStore: { getState: () => { player1?: { battleArea?: unknown[] } } } } }).__dev__?.useGameStore.getState();
        return (s?.player1?.battleArea?.length ?? -1) === 1;
      });

      // The two Lv.4 materials merge into a single own-field permanent:
      // Paildramon on top of a 3-card stack (Paildramon + ExVeemon +
      // Stingmon). (The hand isn't asserted — DNA digivolve grants a draw,
      // so the consumed Paildramon is replaced by a drawn card.)
      const merged = await page.evaluate(() => {
        const s = (window as unknown as { __dev__?: { useGameStore: { getState: () => Record<string, unknown> } } }).__dev__?.useGameStore.getState() as Record<string, unknown>;
        const p1 = (s?.player1 ?? {}) as { battleArea?: Array<{ topCardName?: string; sourceCount?: number }> };
        return {
          fieldCount: p1.battleArea?.length ?? -1,
          top: p1.battleArea?.[0]?.topCardName ?? null,
          sources: p1.battleArea?.[0]?.sourceCount ?? -1,
        };
      });
      expect(merged.fieldCount, JSON.stringify(merged)).toBe(1);
      expect(merged.top).toContain('Paildramon');
      // `sourceCount` counts digivolution cards only (the top card is not
      // one): Paildramon on top of ExVeemon + Stingmon → 2 sources.
      expect(merged.sources).toBe(2);
    } finally {
      await game.delete();
    }
  });
});
