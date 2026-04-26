import { test, expect } from '@playwright/test';

const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

test.describe('Guest onboarding', () => {
  test('guest session grants access to home and deck builder', async ({ page }) => {
    // The desktop build mints a guest session on first launch via
    // `bootstrap/guest.ts → POST /auth/guest`. There is no UI affordance:
    // the mint happens automatically inside `authStore.hydrate()` when
    // `VITE_BUILD_TARGET === 'desktop'`. First, prove the endpoint the
    // bootstrapper hits actually works.
    const mint = await page.request.post(`${API_BASE}/auth/guest`);
    expect(mint.status(), 'POST /auth/guest returns 201 Created').toBe(201);
    const session = (await mint.json()) as {
      access_token: string;
      user_id: string;
      display_name: string;
    };
    expect(session.access_token).toBeTruthy();
    expect(session.user_id).toMatch(/^guest_/);
    expect(session.display_name).toMatch(/^Guest-/);

    // The Playwright suite runs against the web-mode dev server
    // (`npm run dev`), which does not enter the desktop-only mint path
    // during hydrate. Seed localStorage with the exact keys
    // `ensureGuestSession` + `authStore.hydrate` write after a successful
    // mint on desktop so the downstream UI-flow assertions cover the
    // same state the boot path produces.
    await page.addInitScript((s) => {
      localStorage.setItem('guest_access_token', s.access_token);
      localStorage.setItem('guest_user_id', s.user_id);
      localStorage.setItem('guest_display_name', s.display_name);
      localStorage.setItem('access_token', s.access_token);
    }, session);

    await page.goto('/');
    await expect(
      page.getByRole('heading', { name: /PICK UP WHERE YOU LEFT OFF/i }),
    ).toBeVisible();

    // `/deckbuilder` sits under `<AuthGuard>`. Reaching it (rather than
    // being redirected to `/login`) proves the seeded guest session is
    // accepted by the auth store.
    await page.getByRole('link', { name: /Deck builder/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder/);
    await expect(page.getByRole('button', { name: 'Validate' })).toBeVisible();
  });
});
