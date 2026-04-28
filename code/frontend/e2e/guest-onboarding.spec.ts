import { test, expect } from '@playwright/test';

test.describe('Guest onboarding', () => {
  test('guest session grants access to home and deck builder', async ({ page }) => {
    const session = {
      access_token: 'guest-e2e-token',
      user_id: 'guest_e2e',
      display_name: 'Guest-E2E',
    };

    // The Playwright suite can run against either web-mode or desktop-mode
    // Vite. Seed localStorage with the exact keys `ensureGuestSession` +
    // `authStore.hydrate` use so the downstream UI-flow assertions cover
    // authenticated guest state without depending on a live backend.
    await page.addInitScript((s) => {
      localStorage.setItem('guest_access_token', s.access_token);
      localStorage.setItem('guest_user_id', s.user_id);
      localStorage.setItem('guest_display_name', s.display_name);
      localStorage.setItem('access_token', s.access_token);
    }, session);

    await page.goto('/');
    const desktopLauncherHeading = page.getByRole('heading', {
      name: /PICK UP WHERE YOU LEFT OFF/i,
    });
    const webHomeHeading = page.getByRole('heading', { name: 'Digimon TCG Simulator' });
    await expect(desktopLauncherHeading.or(webHomeHeading)).toBeVisible();

    // `/deckbuilder` sits under `<AuthGuard>`. Reaching it (rather than
    // being redirected to `/login`) proves the seeded guest session is
    // accepted by the auth store.
    await page.getByRole('link', { name: /Deck builder/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder/);
    await expect(page.getByRole('button', { name: 'Validate' })).toBeVisible();
  });
});
