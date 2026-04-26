import { test, expect } from '@playwright/test';

test.describe('Desktop launcher', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/health', async (route) => {
      await route.fulfill({ json: { status: 'ok' } });
    });
    await page.route('**/patch-notes', async (route) => {
      await route.fulfill({
        json: {
          known_issues: [],
          releases: [
            {
              id: 'release-1',
              version: '0.4.2',
              release_date: '2026-04-24',
              title: 'Launcher polish',
              added: ['Desktop launcher'],
              changed: ['Guest boot flow'],
              fixed: [],
              created_at: '2026-04-24T00:00:00.000Z',
              updated_at: '2026-04-24T00:00:00.000Z',
            },
          ],
        },
      });
    });
    await page.route('**/decks/tested-cards', async (route) => {
      await route.fulfill({ json: { card_ids: ['BT1-001', 'BT1-002'], card_count: 2 } });
    });
    await page.addInitScript(() => {
      localStorage.setItem('access_token', 'guest-token');
      localStorage.setItem('guest_access_token', 'guest-token');
      localStorage.setItem('guest_user_id', 'guest_abc');
      localStorage.setItem('guest_display_name', 'Guest-ABCD');
    });
  });

  test('renders live server state and launcher actions', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: /PICK UP WHERE YOU LEFT OFF/i })).toBeVisible();
    await expect(page.getByText('CONNECTED')).toBeVisible();
    await expect(page.getByRole('link', { name: /PRIMARY ACTION\s+PLAY/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /LIBRARY\s+MY DECKS/i })).toBeVisible();
    await expect(page.getByText('Launcher polish')).toBeVisible();
  });

  test('navigates launcher actions into existing app routes', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: /Deck builder/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder/);
  });
});
