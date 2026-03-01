import { test as base, type Page } from '@playwright/test';

const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

export async function loginViaApi(page: Page): Promise<{ access_token: string; refresh_token: string }> {
  const resp = await page.request.post(`${API_BASE}/auth/login`, {
    data: { username: 'mammal', password: 'testlogin' },
  });
  if (!resp.ok()) throw new Error(`Login failed: ${resp.status()}`);
  return resp.json();
}

export const test = base.extend<{ authedPage: Page }>({
  authedPage: async ({ page }, use) => {
    const tokens = await loginViaApi(page);
    await page.addInitScript((t) => {
      localStorage.setItem('access_token', t.access_token);
      localStorage.setItem('refresh_token', t.refresh_token);
    }, tokens);
    await use(page);
  },
});

export { expect } from '@playwright/test';
