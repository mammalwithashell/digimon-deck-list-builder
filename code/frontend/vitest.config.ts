import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: false,
    setupFiles: ['./src/testSetup.ts'],
    // Unit tests only — Playwright specs under e2e/ run via `npx playwright
    // test`, and vitest's default include would otherwise collect them.
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
