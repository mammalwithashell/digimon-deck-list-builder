import { test, expect, type Page } from '@playwright/test';

async function openDesktopLauncher(page: Page) {
  await page.goto('/');
  const launcherHeading = page.getByRole('heading', { name: /PICK UP WHERE YOU LEFT OFF/i });
  const isLauncherVisible = await launcherHeading
    .waitFor({ state: 'visible', timeout: 1000 })
    .then(() => true)
    .catch(() => false);
  if (isLauncherVisible) {
    return launcherHeading;
  }

  const webHomeHeading = page.getByRole('heading', { name: 'Digimon TCG Simulator' });
  const isWebHomeVisible = await webHomeHeading
    .waitFor({ state: 'visible', timeout: 1000 })
    .then(() => true)
    .catch(() => false);
  test.skip(isWebHomeVisible, 'desktop launcher route is only available in desktop build');

  await expect(launcherHeading).toBeVisible();
  return launcherHeading;
}

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
    await page.addInitScript(() => {
      localStorage.setItem('access_token', 'guest-token');
      localStorage.setItem('guest_access_token', 'guest-token');
      localStorage.setItem('guest_user_id', 'guest_abc');
      localStorage.setItem('guest_display_name', 'Guest-ABCD');
    });
  });

  test('renders live server state and launcher actions', async ({ page }) => {
    const launcherHeading = await openDesktopLauncher(page);

    await expect(launcherHeading).toBeVisible();
    await expect(page.getByText('CONNECTED')).toBeVisible();
    await expect(page.getByRole('link', { name: /PRIMARY ACTION\s+PLAY/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /LIBRARY\s+MY DECKS/i })).toBeVisible();
    await expect(page.getByText('Launcher polish')).toBeVisible();
  });

  test('navigates launcher actions into existing app routes', async ({ page }) => {
    await openDesktopLauncher(page);
    await page.getByRole('link', { name: /PRIMARY ACTION\s+PLAY/i }).click();
    await expect(page).toHaveURL(/\/play/);
    await expect(page.getByRole('heading', { name: /CHOOSE YOUR\s+FORMAT/i })).toBeVisible();

    await page.goto('/');
    await openDesktopLauncher(page);
    await page.getByRole('link', { name: /Deck builder/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder/);
  });

  test('opens deck panel shortcuts into the new builder routes', async ({ page }) => {
    await openDesktopLauncher(page);

    await page.locator('.launcher-panel-foot').getByRole('link', { name: /\+ New Deck/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/new$/);
    await expect(page.getByText('CARD POOL')).toBeVisible();

    await page.goto('/');
    await openDesktopLauncher(page);
    await page.locator('.launcher-panel-foot').getByRole('link', { name: /Import/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/new\?import=1/);
    await expect(page.getByText('Import / Export Deck')).toBeVisible();
  });
});

test.describe('Desktop launcher offline routing', () => {
  test('desktop offline guest can navigate launcher mock routes without seeded auth', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /PICK UP WHERE YOU LEFT OFF/i })).toBeVisible();

    await page.getByRole('link', { name: /PRIMARY ACTION\s+PLAY/i }).click();
    await expect(page).toHaveURL(/\/play$/);
    await expect(page.getByRole('heading', { name: /CHOOSE YOUR\s+FORMAT/i })).toBeVisible();

    await page.goto('/');
    await page.getByRole('link', { name: /Deck builder/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/new$/);
    await expect(page.getByRole('button', { name: 'VALIDATE' })).toBeVisible();

    await page.goto('/');
    await page.locator('nav[aria-label="Tools"] a[href="/deckbuilder/new?import=1"]').click();
    await expect(page).toHaveURL(/\/deckbuilder\/new\?import=1/);
    await expect(page.getByText('Import / Export Deck')).toBeVisible();
  });
});
