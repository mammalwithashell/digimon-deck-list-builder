/**
 * Two-client room-match flow (add-room-match-pvp).
 *
 * Drives two independent browser contexts (two distinct guests) through the
 * full DCGO-parity room flow against a REAL local server: create room with a
 * 5-digit code → join by code → both lock decks → host picks first player →
 * host starts → both clients auto-enter the game as their seat and the Rust
 * game streams over WebSocket.
 *
 * Pre-reqs (same as the other live-server specs):
 *   - uvicorn server.api:app on :8000 (sqlite default DB is fine)
 *   - vite dev server on :5174 (BASE_URL) proxying /api → :8000 with ws:true
 *
 * The deck library is mocked per-page (the room flow under test is the
 * lobby + WS path, not deck persistence); the deck CONTENTS are real legal
 * cards so the server-side Rust game construction succeeds.
 */
import { expect, test, type Page } from '@playwright/test';

// 50 distinct-but-repeated BT14 Digimon — passes the standard ban list and
// constructs a real Rust game (mirrors code/tests/api/test_lobby_rooms.py).
function legalDeck(): string[] {
  const ids = Array.from({ length: 13 }, (_, i) => `BT14-${String(i + 10).padStart(3, '0')}`);
  const deck: string[] = [];
  for (const cid of ids) {
    for (let n = 0; n < 4 && deck.length < 50; n += 1) deck.push(cid);
  }
  return deck;
}

// Direct API origin (page.request is Node-side — no CORS). Same pattern as
// try-online-vs-ai.spec.ts so the spec works with or without the vite proxy.
const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

/** Mint a real guest on the local server and install its token before any
 * page load (web-mode hydrate doesn't mint guests; AuthGuard would bounce
 * to /login). Same pattern as try-online-vs-ai.spec.ts. */
async function installGuestSession(page: Page) {
  const resp = await page.request.post(`${API_BASE}/auth/guest`);
  if (!resp.ok()) throw new Error(`POST /auth/guest failed (${resp.status()})`);
  const guest = (await resp.json()) as { access_token: string };
  await page.addInitScript((token: string) => {
    localStorage.setItem('access_token', token);
  }, guest.access_token);
}

async function mockDeckLibrary(page: Page) {
  const summary = {
    id: 'deck-1',
    name: 'Ember Vanguard',
    description: '',
    game_mode: 'standard',
    is_valid: true,
    is_public: false,
    is_pinned: false,
    folder_id: null,
    card_count: 50,
    main_count: 50,
    egg_count: 0,
    tags: [],
    meta_tier: 'rogue',
    meta_archetype: 'Red Aggro',
    colors: ['Red'],
    highest_level: 6,
    created_at: '2026-04-27T00:00:00.000Z',
    updated_at: '2026-04-27T00:00:00.000Z',
  };
  await page.route('**/decks/folders', (route) => route.fulfill({ json: [] }));
  await page.route('**/decks', (route) => route.fulfill({ json: [summary] }));
  await page.route('**/decks/deck-1', (route) =>
    route.fulfill({
      json: {
        ...summary,
        owner_id: 'guest_e2e',
        main_deck: legalDeck(),
        egg_deck: [],
        main_deck_alt_arts: [],
        egg_deck_alt_arts: [],
        commander_id: null,
        validation_errors: [],
      },
    }),
  );
}

test.describe('Room match (two clients)', () => {
  test('host and guest complete the room flow into a live PvP game', async ({ browser }) => {
    test.setTimeout(120_000);

    const hostContext = await browser.newContext();
    const guestContext = await browser.newContext();
    const host = await hostContext.newPage();
    const guest = await guestContext.newPage();
    await installGuestSession(host);
    await installGuestSession(guest);
    await mockDeckLibrary(host);
    await mockDeckLibrary(guest);

    try {
      // ── Host: create a room from the chooser ──
      await host.goto('/play/room');
      await host.getByRole('button', { name: 'CREATE ROOM' }).click();
      await host.waitForURL(/\/play\/room\/(?!new)[\w-]+/, { timeout: 15_000 });

      // Room code renders as 5 digits
      const codeEl = host.locator('.room-code-card strong');
      await expect(codeEl).toHaveText(/^\d{5}$/, { timeout: 15_000 });
      const code = (await codeEl.textContent())!.trim();

      // Host deck auto-locks (first legal deck is auto-selected)
      await expect(host.locator('.room-player').first().locator('.ready')).toHaveText('READY', {
        timeout: 15_000,
      });

      // START disabled while waiting for the opponent
      const startButton = host.getByRole('button', { name: 'START GAME' });
      await expect(startButton).toBeDisabled();

      // ── Guest: join via the 5-digit code ──
      await guest.goto('/play/room');
      await guest.getByLabel('Room code').fill(code);
      await guest.getByRole('button', { name: 'JOIN', exact: true }).click();
      await guest.waitForURL(/\/play\/room\/[\w-]+/, { timeout: 15_000 });

      // Guest sees host presence and readies up (deck auto-locks)
      await expect(guest.locator('.room-actions .ready')).toHaveText(
        /WAITING FOR HOST TO START/,
        { timeout: 15_000 },
      );

      // ── Host: sees the guest arrive + ready, picks first player, starts ──
      await expect(
        host.locator('.room-player').nth(1).locator('.ready'),
      ).toHaveText('READY', { timeout: 15_000 });

      await host.locator('.room-first-player button', { hasText: '1' }).first().click();
      await expect(startButton).toBeEnabled({ timeout: 10_000 });
      await startButton.click();

      // ── Both clients auto-enter the game as their seat ──
      await host.waitForURL(/\/game\/[\w-]+\?mode=pvp&player=1/, { timeout: 20_000 });
      await guest.waitForURL(/\/game\/[\w-]+\?mode=pvp&player=2/, { timeout: 20_000 });

      // The live Rust game streams over WS — the board renders for both.
      await expect(host.getByTestId('game-board')).toBeVisible({ timeout: 20_000 });
      await expect(guest.getByTestId('game-board')).toBeVisible({ timeout: 20_000 });
    } finally {
      await hostContext.close();
      await guestContext.close();
    }
  });

  test('joining an unknown code surfaces a friendly error', async ({ page }) => {
    await installGuestSession(page);
    await mockDeckLibrary(page);
    await page.goto('/play/room');
    await page.getByLabel('Room code').fill('00000');
    await page.getByRole('button', { name: 'JOIN', exact: true }).click();
    await expect(page.locator('.room-notice')).toHaveText(/No room with that code/i, {
      timeout: 15_000,
    });
  });
});
