import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'
import { readFileSync } from 'fs'

// Single source of truth for the app version: the Tauri config, which the
// release pipeline bumps per release. Injected at build time so the
// launcher's BUILD label (and the web build) show the real version instead
// of a hardcoded fallback. On desktop this is also re-confirmed at runtime
// via Tauri's getVersion() (see useAppVersion), so the label stays correct
// even if a build ever ships with a stale injected value.
const appVersion = (() => {
  try {
    const conf = JSON.parse(
      readFileSync(path.resolve(__dirname, '../src-tauri/tauri.conf.json'), 'utf-8'),
    ) as { version?: string }
    return typeof conf.version === 'string' ? conf.version : '0.0.0'
  } catch {
    return '0.0.0'
  }
})()

export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    'import.meta.env.VITE_APP_VERSION': JSON.stringify(appVersion),
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true,
        // PvP/spectator game traffic upgrades to WebSocket on the same
        // /api prefix (useWebSocketGame builds ws://host/api/ws/...).
        ws: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
    },
  },
})
