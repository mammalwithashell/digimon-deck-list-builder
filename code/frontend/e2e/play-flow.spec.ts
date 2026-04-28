import { expect, test } from '@playwright/test';

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
});
