import { expect, test, type Page } from '@playwright/test';

async function mockDeckLibrary(page: Page) {
  await page.route('**/decks/folders', (route) => route.fulfill({ json: [] }));
  await page.route('**/decks', (route) =>
    route.fulfill({
      json: [
        {
          id: 'deck-1',
          name: 'Ember Vanguard',
          description: '',
          game_mode: 'standard',
          is_valid: true,
          is_public: false,
          is_pinned: false,
          folder_id: null,
          card_count: 54,
          main_count: 50,
          egg_count: 4,
          tags: [],
          meta_tier: 'rogue',
          meta_archetype: 'Red Aggro',
          colors: ['Red'],
          highest_level: 6,
          created_at: '2026-04-27T00:00:00.000Z',
          updated_at: '2026-04-27T00:00:00.000Z',
        },
      ],
    }),
  );
  await page.route('**/decks/deck-1', (route) =>
    route.fulfill({
      json: {
        id: 'deck-1',
        owner_id: 'guest_abc',
        folder_id: null,
        name: 'Ember Vanguard',
        description: '',
        game_mode: 'standard',
        main_deck: Array(50).fill('BT1-001'),
        egg_deck: Array(4).fill('BT1-002'),
        main_deck_alt_arts: [],
        egg_deck_alt_arts: [],
        commander_id: null,
        is_valid: true,
        validation_errors: [],
        is_public: false,
        is_pinned: false,
        tags: [],
        meta_tier: 'rogue',
        meta_archetype: 'Red Aggro',
        created_at: '2026-04-27T00:00:00.000Z',
        updated_at: '2026-04-27T00:00:00.000Z',
      },
    }),
  );
}

test.describe('In Between play flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('access_token', 'guest-token');
      localStorage.setItem('guest_access_token', 'guest-token');
      localStorage.setItem('guest_user_id', 'guest_abc');
      localStorage.setItem('guest_display_name', 'Guest-ABCD');
    });
    await page.route('**/api/users/me', (route) =>
      route.fulfill({
        json: { id: 'guest_abc', username: 'Guest-ABCD', email: null, roles: [] },
      }),
    );
    await page.route('**/formats', (route) =>
      route.fulfill({
        json: [
          {
            id: 'standard',
            name: 'STANDARD',
            tagline: 'The official ruleset',
            description: '50-card decks, current banlist, mirrored memory gauge.',
            deck_label: '50 cards',
            population_pct: 84,
            enabled: true,
            disabled_reason: null,
          },
        ],
      }),
    );
  });

  test('opens format selection from launcher play', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: /PRIMARY ACTION\s+PLAY/i }).click();
    await expect(page).toHaveURL(/\/play$/);
    await expect(page.getByRole('heading', { name: /CHOOSE YOUR\s+FORMAT/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /QUICK MATCH/i })).toBeVisible();
  });

  test('chooses quick match standard and advances to deck select', async ({ page }) => {
    await page.goto('/play');
    await page.getByRole('button', { name: /QUICK MATCH/i }).click();
    await page.getByRole('button', { name: /STANDARD/i }).click();
    await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
    await expect(page).toHaveURL(/\/play\/deck/);
  });

  test('selects a legal deck and advances to matching', async ({ page }) => {
    await mockDeckLibrary(page);
    await page.goto('/play');
    await page.getByRole('button', { name: /STANDARD/i }).click();
    await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
    await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
    await page.getByRole('button', { name: /USE THIS DECK/i }).click();
    await expect(page).toHaveURL(/\/play\/matching/);
  });

  test('queues selected deck for quick match', async ({ page }) => {
    await mockDeckLibrary(page);
    let queuePayload: unknown = null;
    await page.route('**/matchmaking/queue', async (route) => {
      queuePayload = route.request().postDataJSON();
      await route.fulfill({ json: { status: 'waiting', ticket_id: 'ticket-1' } });
    });
    await page.goto('/play');
    await page.getByRole('button', { name: /STANDARD/i }).click();
    await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
    await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
    await page.getByRole('button', { name: /USE THIS DECK/i }).click();
    await expect(page.getByRole('heading', { name: /SEARCHING\s+FOR AN OPPONENT/i })).toBeVisible();
    await expect.poll(() => queuePayload).not.toBeNull();
    expect(queuePayload).toMatchObject({
      queue_type: 'casual',
      game_mode: 'standard',
    });
  });

  test('creates a room immediately and locks a deck inside the room', async ({ page }) => {
    await mockDeckLibrary(page);
    let createPayload: Record<string, unknown> | null = null;
    let deckPayload: Record<string, unknown> | null = null;
    await page.route('**/lobby/create', (route) => {
      createPayload = route.request().postDataJSON();
      return route.fulfill({ json: { game_id: 'game-1', join_code: 'ABC123' } });
    });
    await page.route('**/lobby/game-1/deck', (route) => {
      deckPayload = route.request().postDataJSON();
      return route.fulfill({
        json: {
          game_id: 'game-1',
          join_code: 'ABC123',
          host_display_name: 'Guest-ABCD',
          host_deck_ready: true,
          joiner_deck_ready: false,
          started: false,
        },
      });
    });
    await page.goto('/play');
    await page.getByRole('button', { name: /ROOM MATCH/i }).click();
    await page.getByRole('button', { name: /STANDARD/i }).click();
    await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
    await expect(page).toHaveURL(/\/play\/room\/new/);
    await expect(page.getByText('ABC123', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: /EMBER VANGUARD/i })).toBeVisible();
    expect(createPayload).toMatchObject({
      is_public: false,
      allow_spectators: true,
      spectator_mode: 'hidden',
    });
    expect(createPayload).not.toHaveProperty('deck');
    await expect.poll(() => deckPayload).not.toBeNull();
    expect((deckPayload?.deck as string[]) ?? []).toContain('BT1-001');
    expect((deckPayload?.deck as string[]) ?? []).toContain('BT1-002');
  });

  test('bot match starts local game route from deck selection', async ({ page }) => {
    await mockDeckLibrary(page);
    await page.route('**/games', (route) =>
      route.fulfill({
        json: {
          game_id: 'game-bot',
          state: {
            turn_count: 1,
            current_phase: 'Main',
            memory: 0,
            game_over: false,
            winner: null,
            players: [],
          },
          action_mask: [],
        },
      }),
    );
    await page.goto('/play');
    await page.getByRole('button', { name: /BOT MATCH/i }).click();
    await page.getByRole('button', { name: /STANDARD/i }).click();
    await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
    await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
    await page.getByRole('button', { name: /USE THIS DECK/i }).click();
    await expect(page).toHaveURL(/\/game\/game-bot/);
  });
});
