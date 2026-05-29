import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  retries: 0,
  reporter: [['html', { open: 'never' }], ['list']],
  use: {
    baseURL: process.env.BASE_URL ?? 'http://localhost:5174',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    // WATCH_E2E=1 → record a slowed-down video of the run so a human can
    // watch the flow play out (headed Chromium can't spawn in this env;
    // headless-shell + video capture is the watchable substitute).
    launchOptions: process.env.WATCH_E2E === '1' ? { slowMo: 600 } : {},
    video: process.env.WATCH_E2E === '1' ? 'on' : 'off',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});
