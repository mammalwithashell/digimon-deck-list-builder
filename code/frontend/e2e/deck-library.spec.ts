import { test, expect } from '@playwright/test';

test.describe('Deck Library', () => {
  test('renders saved decks, filters, pins, and opens the builder route', async ({ page }) => {
    let decks = [
      {
        id: 'deck-1',
        name: 'Blue Flare',
        description: 'Hybrid aggro notes',
        game_mode: 'standard',
        is_valid: true,
        is_public: false,
        is_pinned: false,
        folder_id: 'folder-1',
        card_count: 54,
        main_count: 50,
        egg_count: 4,
        tags: ['locals'],
        meta_tier: 'meta',
        meta_archetype: 'Hybrid Aggro',
        colors: [],
        highest_level: null,
        created_at: '2026-04-20T00:00:00Z',
        updated_at: '2026-04-26T00:00:00Z',
      },
      {
        id: 'deck-2',
        name: 'Green Bugs',
        description: '',
        game_mode: 'standard',
        is_valid: false,
        is_public: false,
        is_pinned: true,
        folder_id: null,
        card_count: 43,
        main_count: 39,
        egg_count: 4,
        tags: [],
        meta_tier: null,
        meta_archetype: 'Insect Tribal',
        colors: [],
        highest_level: null,
        created_at: '2026-04-19T00:00:00Z',
        updated_at: '2026-04-25T00:00:00Z',
      },
    ];
    const folders = [
      {
        id: 'folder-1',
        owner_id: 'u1',
        name: 'Tournament',
        sort_order: 0,
        created_at: '2026-04-20T00:00:00Z',
        updated_at: '2026-04-20T00:00:00Z',
      },
    ];

    await page.addInitScript(() => {
      localStorage.setItem('access_token', 'test-token');
    });
    await page.route('**/api/users/me', (route) =>
      route.fulfill({
        json: { id: 'u1', username: 'tester', email: 't@example.com', roles: [] },
      }),
    );
    await page.route('**/api/decks/folders', (route) => route.fulfill({ json: folders }));
    await page.route('**/api/decks', (route) => {
      if (route.request().method() === 'GET') return route.fulfill({ json: decks });
      return route.fallback();
    });
    await page.route('**/api/decks/deck-1', (route) =>
      route.fulfill({
        json: {
          ...decks[0],
          owner_id: 'u1',
          main_deck: ['BT1-001', 'BT1-002'],
          egg_deck: ['BT1-003'],
          main_deck_alt_arts: [],
          egg_deck_alt_arts: [],
          commander_id: null,
          validation_errors: [],
        },
      }),
    );
    await page.route('**/api/decks/deck-2', (route) =>
      route.fulfill({
        json: {
          ...decks[1],
          owner_id: 'u1',
          main_deck: ['BT1-004'],
          egg_deck: [],
          main_deck_alt_arts: [],
          egg_deck_alt_arts: [],
          commander_id: null,
          validation_errors: [],
        },
      }),
    );
    await page.route('**/api/decks/*/library', async (route) => {
      const id = route.request().url().match(/\/decks\/([^/]+)\/library/)?.[1];
      const body = route.request().postDataJSON() as { is_pinned?: boolean };
      decks = decks.map((deck) =>
        deck.id === id && typeof body.is_pinned === 'boolean'
          ? { ...deck, is_pinned: body.is_pinned }
          : deck,
      );
      return route.fulfill({
        json: {
          ...decks.find((deck) => deck.id === id),
          owner_id: 'u1',
          main_deck: [],
          egg_deck: [],
          main_deck_alt_arts: [],
          egg_deck_alt_arts: [],
          commander_id: null,
          validation_errors: [],
        },
      });
    });
    await page.route('https://digimoncard.io/**', (route) =>
      route.fulfill({
        json: [
          {
            id: 'BT1-001',
            name: 'Test Card',
            type: 'Digimon',
            color: 'Blue',
            level: 3,
            play_cost: 3,
            rarity: 'C',
          },
        ],
      }),
    );

    await page.goto('/deckbuilder');
    await expect(page.getByRole('heading', { name: 'Blue Flare' })).toBeVisible();
    await expect(page.getByText('Green Bugs')).toBeVisible();

    await page.getByPlaceholder('Decks, archetypes, tags').fill('hybrid');
    await expect(page.getByText('Blue Flare').first()).toBeVisible();
    await expect(page.getByText('Green Bugs')).toHaveCount(0);

    await page.getByPlaceholder('Decks, archetypes, tags').fill('');
    await page.locator('button[title="Pin deck"]').first().click();
    await expect(page.getByText('Pinned').first()).toBeVisible();

    await page.getByRole('button', { name: 'Edit' }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/deck-1$/);
  });
});
