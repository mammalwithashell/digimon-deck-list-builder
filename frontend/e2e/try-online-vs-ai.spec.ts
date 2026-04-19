import { test, expect } from '@playwright/test';
import { MEDUSA_DECK } from './fixtures/debug-game';

const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

interface ManifestModel {
  id: string;
  name: string;
}

interface ManifestResponse {
  generated_at: string;
  models: ManifestModel[];
}

interface GuestResponse {
  access_token: string;
  user_id: string;
  display_name: string;
}

// Eggs live at the front of MEDUSA_DECK (first four entries).
const EGG_DECK = MEDUSA_DECK.slice(0, 4);
const MAIN_DECK = MEDUSA_DECK.slice(4);

test.describe('Try Online vs AI', () => {
  test('anonymous guest can start, play, and concede a vs-AI game', async ({ page }) => {
    // Pre-req: at least one AI model must be published. If none, skip — the
    // alpha publish flow lives behind admin auth and is covered elsewhere.
    const manifestResp = await page.request.get(`${API_BASE}/models/manifest.json`);
    expect(manifestResp.ok(), `GET /models/manifest.json (${manifestResp.status()})`).toBeTruthy();
    const manifest = (await manifestResp.json()) as ManifestResponse;
    test.skip(
      !manifest.models || manifest.models.length === 0,
      'No AI models published — run tools/export_null_agent.py and admin-publish first',
    );

    // Mint an anonymous guest session. The hosted API issues a long-lived
    // JWT; the UI treats this token the same as a logged-in user's.
    const guestResp = await page.request.post(`${API_BASE}/auth/guest`);
    expect(guestResp.ok(), `POST /auth/guest (${guestResp.status()})`).toBeTruthy();
    const guest = (await guestResp.json()) as GuestResponse;

    // Create a deck owned by the guest so the ModelsPage deck-picker is
    // non-empty. Must hit the decks API with the guest's bearer token —
    // ModelsPage won't enable the "Try online" button without a deck.
    const deckResp = await page.request.post(`${API_BASE}/decks`, {
      headers: { Authorization: `Bearer ${guest.access_token}` },
      data: {
        name: 'E2E Medusa',
        game_mode: 'standard',
        main_deck: MAIN_DECK,
        egg_deck: EGG_DECK,
      },
    });
    expect(deckResp.ok(), `POST /decks (${deckResp.status()}): ${await deckResp.text().catch(() => '')}`).toBeTruthy();

    // Install the guest token in localStorage before any page load, so the
    // axios interceptor picks it up for /decks, /models/{id}/prepare, /games.
    await page.addInitScript((t: string) => {
      localStorage.setItem('access_token', t);
    }, guest.access_token);

    // Navigate to the ModelsPage and click the first "Try online" button.
    await page.goto('/models');
    const tryOnline = page.getByRole('button', { name: /Try online/i }).first();
    await expect(tryOnline).toBeVisible({ timeout: 20_000 });
    await tryOnline.click();

    // Server prepares the ONNX blob then creates a vs-AI game; the page
    // navigates to /game/:id?mode=vsai&player=1.
    await expect(page).toHaveURL(/\/game\/[^/?#]+\?.*mode=vsai/, { timeout: 20_000 });
    await expect(page.getByTestId('game-board')).toBeVisible({ timeout: 30_000 });

    const match = page.url().match(/\/game\/([^/?#]+)/);
    expect(match).not.toBeNull();
    const gameId = match![1]!;

    // Submit at least one legal action. Use the REST endpoint — the mask
    // reflects the server-side runner state and is authoritative.
    const maskResp = await page.request.get(`${API_BASE}/games/${gameId}/action-mask`);
    expect(maskResp.ok()).toBeTruthy();
    const { action_mask: mask } = (await maskResp.json()) as { action_mask: number[] };
    const firstLegal = mask.findIndex((v) => v === 1);
    expect(firstLegal, `Mask should expose at least one legal action: ${mask.slice(0, 20)}…`).toBeGreaterThanOrEqual(0);

    const actResp = await page.request.post(`${API_BASE}/games/${gameId}/actions`, {
      data: { action: firstLegal },
    });
    expect(actResp.ok(), `POST /games/${gameId}/actions (${actResp.status()})`).toBeTruthy();

    // Concede to end the game deterministically.
    const surrResp = await page.request.post(`${API_BASE}/games/${gameId}/surrender`, {
      data: { player_id: 1 },
    });
    expect(surrResp.ok(), `POST /games/${gameId}/surrender (${surrResp.status()})`).toBeTruthy();
    const surr = (await surrResp.json()) as { is_game_over: boolean };
    expect(surr.is_game_over).toBe(true);

    // Return to home. Actions/surrender went via HTTP (the UI is on the
    // WebSocket path in vs-AI mode and won't auto-reflect them), so we
    // navigate explicitly and assert the home screen is restored.
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Digimon TCG Simulator/i })).toBeVisible();
  });
});
