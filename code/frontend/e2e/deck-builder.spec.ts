import { expect, test, type Page } from '@playwright/test';

const apiCards = [
  {
    id: 'BT1-001',
    name: 'Gaomon',
    type: 'Digimon',
    color: 'Blue',
    level: 3,
    play_cost: 3,
    rarity: 'C',
    main_effect: 'A small blue Digimon for testing.',
    source_effect: 'Inherited test effect.',
    set_name: 'BT-01 Booster',
  },
  {
    id: 'BT1-002',
    name: 'Bukamon',
    type: 'Digi-Egg',
    color: 'Blue',
    level: 2,
    play_cost: null,
    rarity: 'U',
    main_effect: '',
    source_effect: 'Draw 1.',
    set_name: 'BT-01 Booster',
  },
];

const savedDeck = {
  id: 'deck-1',
  owner_id: 'u1',
  name: 'Blue Flare',
  description: '',
  game_mode: 'standard',
  is_valid: true,
  is_public: false,
  is_pinned: false,
  folder_id: null,
  card_count: 3,
  main_count: 2,
  egg_count: 1,
  tags: [],
  meta_tier: null,
  meta_archetype: null,
  colors: ['Blue'],
  highest_level: 3,
  main_deck: ['BT1-001', 'BT1-001'],
  egg_deck: ['BT1-002'],
  main_deck_alt_arts: [false, false],
  egg_deck_alt_arts: [false],
  commander_id: null,
  validation_errors: [],
  created_at: '2026-04-20T00:00:00Z',
  updated_at: '2026-04-26T00:00:00Z',
};

async function mockDeckBuilderRoutes(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem('access_token', 'test-token');
    localStorage.setItem('guest_access_token', 'test-token');
    localStorage.setItem('guest_user_id', 'u1');
    localStorage.setItem('guest_display_name', 'tester');
  });

  await page.route('**/auth/guest', (route) =>
    route.fulfill({
      json: { access_token: 'test-token', user_id: 'u1', display_name: 'tester' },
    }),
  );
  await page.route('**/users/me', (route) =>
    route.fulfill({
      json: { id: 'u1', username: 'tester', email: 't@example.com', roles: [] },
    }),
  );
  await page.route('**/decks/tested-cards', (route) =>
    route.fulfill({
      json: { card_ids: apiCards.map((card) => card.id), card_count: apiCards.length },
    }),
  );
  await page.route('**/decks/validate', (route) =>
    route.fulfill({ json: { is_valid: true, errors: [], warnings: [] } }),
  );
  await page.route('**/decks/deck-1', (route) => {
    if (route.request().method() === 'GET') return route.fulfill({ json: savedDeck });
    if (route.request().method() === 'PUT') return route.fulfill({ json: savedDeck });
    return route.fallback();
  });
  await page.route('**/decks', async (route) => {
    if (route.request().method() === 'POST') {
      const body = route.request().postDataJSON() as { name?: string };
      return route.fulfill({ json: { ...savedDeck, name: body.name ?? 'New Deck' } });
    }
    if (route.request().method() === 'GET') {
      return route.fulfill({ json: [savedDeck] });
    }
    return route.fallback();
  });
  await page.route('https://digimoncard.io/**', (route) => {
    const url = new URL(route.request().url());
    const cardId = url.searchParams.get('card');
    const cards = cardId ? apiCards.filter((card) => card.id === cardId) : apiCards;
    return route.fulfill({ json: cards });
  });
}

test.describe('Deck Builder', () => {
  test.beforeEach(async ({ page }) => {
    await mockDeckBuilderRoutes(page);
  });

  test('renders new builder chrome and uses the mocked card pool', async ({ page }) => {
    await page.goto('/deckbuilder/new');

    await expect(page.getByText('CARD POOL')).toBeVisible();
    await expect(page.getByText('DECK CONTENTS')).toBeVisible();
    await expect(page.getByLabel('Deck name')).toHaveValue('New Deck');
    await expect(page.getByRole('button', { name: 'Gaomon BT1-001' })).toBeVisible();
  });

  test('adds and removes a card from the deck with click and right-click', async ({ page }) => {
    await page.goto('/deckbuilder/new');

    const cardTile = page.getByRole('button', { name: 'Gaomon BT1-001' });
    const deckPane = page.locator('.bld-deck');

    await cardTile.click();
    await expect(cardTile.locator('.ct')).toHaveText('x1');
    await expect(deckPane.locator('.bld-section-head .ct')).toHaveText('1 / 50');

    await cardTile.click({ button: 'right' });
    await expect(cardTile.locator('.ct')).toHaveCount(0);
    await expect(deckPane.locator('.bld-section-head .ct')).toHaveText('0 / 50');
  });

  test('saves a new deck and loads an existing saved deck', async ({ page }) => {
    await page.goto('/deckbuilder/new');
    await page.getByRole('button', { name: 'Gaomon BT1-001' }).click();

    await page.getByRole('button', { name: 'SAVE' }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/deck-1$/);

    await page.goto('/deckbuilder/deck-1');
    await expect(page.getByLabel('Deck name')).toHaveValue('Blue Flare');
    await expect(page.getByText('CARD POOL')).toBeVisible();
    await expect(page.getByText('DECK CONTENTS')).toBeVisible();
    await expect(page.locator('.bld-deck .bld-section-head .ct')).toHaveText('2 / 50');
  });

  test('opens import modal from the new-deck query parameter', async ({ page }) => {
    await page.goto('/deckbuilder/new?import=1');

    await expect(page.getByText('Import / Export Deck')).toBeVisible();
  });

  test('returns to play deck selection after saving from the play flow', async ({ page }) => {
    await page.goto('/deckbuilder/new?returnTo=play');

    await expect(page.getByRole('button', { name: 'BACK TO PLAY' })).toBeVisible();
    await page.getByRole('button', { name: 'Gaomon BT1-001' }).click();
    await page.getByRole('button', { name: 'SAVE' }).click();

    await expect(page).toHaveURL(/\/play\/deck$/);
  });
});
