# Landing-page desktop screenshots + companion skill — design

**Date:** 2026-06-22
**Status:** Approved (brainstorming) — ready for implementation plan
**Companion to:** `cut-desktop-release` (run around release time so screenshots track the shipped build)

## Problem

The landing page (`code/landing/index.html`, deployed as-is to GitHub Pages by
`.github/workflows/landing-page.yml`) is text-only — it sells a visual desktop
app with no pictures of it. We want a screenshot gallery of the app's mainstay
pages, and a repeatable skill that recaptures and republishes those shots.

The shots must come from the **real desktop app**, not a plain browser. A
capture trial (this session) confirmed why: the Tauri webview loads the same
`dev:desktop` frontend a browser can, but only the real app populates
backend-fed content (decks, models, a live game) through Tauri `invoke()`. A
Playwright shot of the Launcher rendered the correct chrome but an empty deck
panel (`:8000/desktop-decks` refused); the real window showed five real decks.

## Goals

- A committed screenshot gallery on the landing page, even 2-column grid, each
  shot in a CRT "monitor" frame, with a working light/dark toggle.
- A `update-landing-screenshots` skill that launches the real app, drives it
  through each mainstay page in both themes, captures the window, writes the
  assets, and commits + pushes (triggering the Pages deploy).
- Capture the **real Tauri window**, scripted and clean — no flaky computer-use.

## Non-goals

- No changes to how the landing page deploys (`landing-page.yml` stays as-is;
  it already ships `code/landing/` verbatim).
- Not auto-run inside `cut-desktop-release` — a separate, on-demand skill
  (release skill will merely *suggest* running it).
- No new MCP surface — the skill talks to the existing debug bridge over HTTP.
- Not the hosted web build's `HomePage`/`WebRoutes`; desktop route tree only.

## Decisions (from brainstorming)

| Decision | Choice | Why |
|---|---|---|
| Capture mechanism | **A — real Tauri window** via PowerShell `PrintWindow(PW_RENDERFULLCONTENT)` | Proven this session: clean full-window PNG, works even unfocused, no computer-use. Populated, genuine. |
| Page navigation | **Dev-only bridge hook** (`/navigate {route, theme}`) | App uses client-side routing — no external URL. Bridge already exists + is feature-gated/dev-only. Deterministic, scriptable. |
| Pages | launcher, game board, deck builder, deck library, AI models | The mainstay set the user picked. |
| Themes | **both** dark + light | Each page ×2 — also showcases the theming. Dark is the app default (matches the page). |
| Gallery layout | **Even 2-col grid** | User pick over hero+filmstrip. |
| Publish | **Auto commit + push** | Push to `main` triggers `landing-page.yml`. |

## Architecture

Three units, each independently understandable/testable:

### Unit 1 — Dev-only navigation/theme hook (enabling change)

- **Rust** (`code/src-tauri/src/debug_bridge.rs`): add a `POST /navigate` route
  to the existing axum router. Body `{ "route": "/deckbuilder", "theme":
  "dark" }` (theme optional). Handler emits a `debug:navigate` window event
  carrying the payload. The whole module is already `#![cfg(feature =
  "debug-bridge")]` and the server only starts under `DIGIMON_DEBUG_BRIDGE=1`,
  so this adds **zero** release surface. ~15 lines, mirrors existing handlers.
- **Frontend** (`code/frontend/src/components/desktop/DebugBridgeNav.tsx`, new):
  a render-null component mounted inside `<BrowserRouter>` (in `App.tsx`, beside
  `UpdaterBridge`) **only when** `import.meta.env.DEV && IS_DESKTOP`. It
  `listen()`s (`@tauri-apps/api/event`) for `debug:navigate` and calls
  `useNavigate()(route)` + `useThemeStore.getState().setTheme(theme)`. Gated by
  `import.meta.env.DEV`, so production desktop builds (`vite build --mode
  desktop`) tree-shake it out. Events only originate from the dev bridge, so it
  is doubly inert in release.
  - Interface in: `debug:navigate` window event `{route, theme?}`.
  - Depends on: react-router `useNavigate`, `useThemeStore` (`setTheme`,
    themes are exactly `'dark' | 'light'`).

### Unit 2 — Capture toolkit (skill scripts)

`.claude/skills/update-landing-screenshots/scripts/`

- **`capture_window.ps1`** — by process name (`digimon-tcg`) find the main
  window, `SetProcessDPIAware`, capture via `PrintWindow(hwnd, hdc, 0x2)`
  (PW_RENDERFULLCONTENT — verified to capture the WebView2 content), save PNG.
  Param: `-OutPath`. Prototyped this session.
- **`to_webp.ps1`** — crop the native OS title bar + any letterbox margins to
  the app canvas region, then convert PNG → WebP. Param: in/out + crop box.
  (Crop box tuned during implementation; capture client-rect to minimize it.)
- Interface: PNG/WebP files on disk. Pure, no app knowledge.

### Unit 3 — The skill recipe (`SKILL.md`) + fixture

A guided recipe (agent executes with judgment — staging a good board, verifying
crops). Orchestrates Units 1 & 2 and the existing bridge `/stage`.

Flow:
1. **Preconditions** — on `main`'s tip; `gh` auth; frontend deps present.
2. **Launch** (reuse `run-desktop`): `dev:desktop` on :5173, then
   `cargo tauri dev --features debug-bridge` + `DIGIMON_DEBUG_BRIDGE=1` +
   `--config '{"build":{"beforeDevCommand":""}}'`. Read bridge port from
   `<data_dir>/digimon-tcg/debug_bridge.json`.
3. **Capture loop** — for `theme in [dark, light]`, for each page:
   - `POST /navigate {route, theme}`.
   - Game board: also `POST /stage` the committed `fixtures/hero-board.json`.
   - Wait for render (poll bridge `/state` or fixed settle), run
     `capture_window.ps1`, then `to_webp.ps1`.
4. **Write** → `code/landing/assets/screenshots/<page>-<theme>.webp`
   (fixed filenames: `launcher`, `game-board`, `deck-builder`, `deck-library`,
   `ai-models` × `dark|light` = 10 assets).
5. **Wire `index.html`** — insert the gallery section if absent (idempotent);
   recaptures only overwrite WebPs.
6. **Commit + push** to `main` → `landing-page.yml` deploys.
7. **Teardown** — stop Tauri + dev server.

`fixtures/hero-board.json` — a curated, legal mid-match board (digivolved
Digimon, interesting memory, some security) for an alive hero shot. Same schema
the scenario-MCP / bridge `/stage` consume (`decks` + `state` + `zones`).

### Unit 4 — Landing-page gallery (`code/landing/index.html`)

New `<section>`: `>> visual feed`. Even 2-col grid (`auto-fit`/explicit 2-col),
1-col under the existing 640px breakpoint. Each tile:
- a phosphor-bezel + scanline frame (reuse the page's `--line`/`--phosphor`
  palette and `repeating-linear-gradient` scanlines), `aspect-ratio:1280/768`;
- two `<img loading="lazy">` (dark + light WebP), one shown via the grid's
  `data-skin` attribute;
- a `// <page>.png` caption.

One light/dark switch in the section header toggles `data-skin` on the grid
(vanilla JS, ~5 lines). Respects `prefers-reduced-motion` (already handled
globally). No build step — consistent with the page's hand-authored style.

## Data flow

```
skill recipe ──HTTP──▶ debug bridge :5174 ──event──▶ DebugBridgeNav ──▶ router + theme
     │                      │ /stage
     │                      └──────────▶ engine world.game ──event──▶ GamePage renders board
     ▼
capture_window.ps1 ──PrintWindow──▶ PNG ──to_webp.ps1──▶ code/landing/assets/screenshots/*.webp
     ▼
git commit + push main ──▶ landing-page.yml ──▶ GitHub Pages
```

## Open implementation risks (verify in plan)

1. **Staged board → on-screen render.** Getting a board visible needs the
   frontend on a `/game` route bound to the bridge-installed `world.game`. The
   scenario-MCP already stages boards in the desktop window, so a working path
   exists — reuse/verify it (navigate to the game route, then `/stage`), rather
   than inventing one. Confirm `GamePage`'s load path against an
   externally-installed game.
2. **Crop fidelity.** The window includes a native title bar and the
   CanvasScaler may letterbox. Prefer capturing the webview **client rect**
   (`GetClientRect`/`ClientToScreen`) and tune a small crop in `to_webp.ps1`;
   on-theme margins are acceptable if cropping is fragile.
3. **DPI.** Capture machine is 150% DPI (window ~1936×1137 for a 1280×768
   logical window). `SetProcessDPIAware` handled it this session; downscale the
   final WebP to a sane gallery width (~640–960px).

## Verification

- Unit 1: `cargo test --manifest-path code/src-tauri/Cargo.toml --lib` (bridge
  handler unit test, like existing `stage_into` tests); frontend `tsc` +
  component mount gating check.
- Unit 2/3: run the skill end-to-end; `Read` each emitted WebP (the tool
  renders WebP) to self-verify content + theme + crop before committing.
- Unit 4: open `code/landing/index.html` locally; toggle light/dark; check
  1-col mobile reflow.

## Files

New:
- `.claude/skills/update-landing-screenshots/SKILL.md`
- `.claude/skills/update-landing-screenshots/scripts/capture_window.ps1`
- `.claude/skills/update-landing-screenshots/scripts/to_webp.ps1`
- `.claude/skills/update-landing-screenshots/fixtures/hero-board.json`
- `code/frontend/src/components/desktop/DebugBridgeNav.tsx`
- `code/landing/assets/screenshots/*.webp` (10, generated by the skill)

Edited:
- `code/src-tauri/src/debug_bridge.rs` (+`/navigate`)
- `code/frontend/src/App.tsx` (mount `DebugBridgeNav` in dev+desktop)
- `code/landing/index.html` (+ gallery section)
- `.claude/skills/cut-desktop-release/SKILL.md` (a one-line "consider running
  update-landing-screenshots" pointer)
