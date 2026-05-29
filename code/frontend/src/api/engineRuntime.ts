/**
 * Runtime detection + small HTTP client for the engine-only API
 * (`/games`, `/games/{id}/...`).
 *
 * Why this exists
 * ---------------
 * The desktop build's `gameApi.ts` was historically Tauri-`invoke()`-only.
 * Running the same bundle in a plain browser (the dev/testing workflow
 * `npm run dev:desktop` + Playwright) requires an HTTP fallback so the
 * frontend can talk to the local FastAPI server (`code/server`) which
 * exposes the same Rust engine via PyO3.
 *
 * The detection is purely runtime: presence of `window.__TAURI_INTERNALS__`
 * (the global Tauri injects into its webview). This means a *single*
 * build of the frontend can run unchanged in either context — desktop
 * shell or browser — and pick the right transport on its own.
 */

/** True iff this page is loaded inside a Tauri webview that has the
 *  internals global injected. False in plain browsers (Chromium opened
 *  by Playwright, the Vite dev server preview, etc). */
export function isInTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  return Boolean(
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__,
  );
}

/** Base URL for the hosted engine API in browser-dev mode. Override
 *  via `VITE_ENGINE_API_URL` if the FastAPI server isn't on
 *  `http://localhost:8000` (e.g. when developing against a remote
 *  hosted-API instance). */
export function engineApiBaseUrl(): string {
  return import.meta.env.VITE_ENGINE_API_URL ?? 'http://localhost:8000';
}

/** Lightweight `fetch` wrapper that raises on non-2xx and decodes
 *  JSON. Mirrors the error shape of Tauri `invoke` (throws on failure).
 *  Keeps the surface minimal so the rest of `gameApi.ts` can call it
 *  without importing axios or another HTTP client. */
export async function httpJson<T>(
  path: string,
  init: { method?: string; body?: unknown } = {},
): Promise<T> {
  const method = init.method ?? 'GET';
  const url = `${engineApiBaseUrl()}${path}`;
  const headers: Record<string, string> = {};
  let body: BodyInit | undefined;
  if (init.body !== undefined) {
    headers['Content-Type'] = 'application/json';
    body = JSON.stringify(init.body);
  }
  const resp = await fetch(url, { method, headers, body });
  if (!resp.ok) {
    let detail: string;
    try {
      const j = await resp.json();
      detail = j.detail ?? JSON.stringify(j);
    } catch {
      detail = await resp.text();
    }
    throw new Error(`engine API ${method} ${path} failed: ${resp.status} ${detail}`);
  }
  return (await resp.json()) as T;
}
