## Why

The desktop app's game board has spacing bugs at non-default window sizes — the
battle area's 14 slots are laid out in a `repeat(6, ...)` CSS grid, so they
wrap to 3 rows and the third row collides with the memory gauge at narrower
window widths. The current "resize responsively with media queries" model is
also fighting itself: every CSS breakpoint is one more layout to maintain, and
small windows produce broken layouts the user actually sees. DCGO solves this
the same way every shipping TCG client does — authors the board at a single
fixed resolution and uniformly scales the canvas to whatever window the user
picks from a preset list. Adopting the same shape eliminates the responsive
layout maintenance burden and lets us guarantee a correct board at every
supported resolution.

## What Changes

- **Add a Graphics Settings page** (desktop only) with a fullscreen toggle and
  8 resolution preset buttons matching DCGO exactly: 1024×576, 1280×720,
  1600×900, 1920×1080, 2560×1440, 3440×1440, 3840×2160, 5160×2160.
- **Persist the selected preset + fullscreen state** across launches via
  `localStorage` plus Tauri's window-state restoration so the user sees the
  same window on next launch.
- **Adopt a fixed 1920×1080 internal canvas.** Wrap the entire game UI in a
  scaler that applies `transform: scale(min(w/1920, h/1080))` and
  `transform-origin: top left`, centered in the window. Smaller windows
  shrink uniformly; larger windows grow uniformly.
- **Letterbox ultrawide presets.** The 3440×1440 preset (21.5:9) renders the
  16:9 canvas centered with side bars rather than stretching, matching DCGO.
- **Battle area becomes a strict 7-column × 2-row grid.** Change
  `grid-template-columns: repeat(6, ...)` to `repeat(7, ...)` so 14 slots
  always lay out as exactly 2 rows. Cards still pack left-to-right via the
  engine's `Vec` model (no engine changes — same behavior as DCGO).
- **Animate slot-shift on permanent removal.** When a permanent in the
  middle of the row dies and engine indices shift left, animate the
  remaining cards sliding to their new positions instead of teleporting.
  FLIP-style animation in the frontend; engine semantics unchanged.
- **Tauri window config update.** Drop `minWidth`/`minHeight` constraints
  that no longer apply, raise the default startup size to match the second
  preset (1280×720), and add resizable=false except when fullscreen is on
  (presets are the only way to change window size).
- **Purge media queries inside the game-board CSS** since the board now
  always renders at 1920×1080 internally. Drop the existing breakpoint at
  `@media (max-width: ...)` for `.ib-battle-area` and friends.

## Capabilities

### New Capabilities

- `desktop-graphics-settings`: Resolution-preset selector, fullscreen toggle,
  fixed-canvas scaling model, and the rendering contract for the game board
  inside that canvas (2-row battle grid, slot-shift animation).

### Modified Capabilities

<!-- None. No existing spec governs desktop window sizing or board layout. -->

## Impact

- **Frontend (`code/frontend/src/`)** — new `<CanvasScaler>` wrapper at the
  root of the game UI; new `GraphicsSettingsPage` route; updates to
  `index.css` (battle grid columns, removal of board-internal media
  queries); FLIP animation utility for permanent slot shifts; settings
  persistence via `uiStore.ts` + localStorage.
- **Tauri (`code/src-tauri/`)** — `tauri.conf.json` window defaults and
  resizable flag; possibly a `set_window_preset` command if we want
  centralized preset application (or rely entirely on the JS-side window
  API from `@tauri-apps/api/window`).
- **No engine changes.** The Rust engine, action space, tensor encoding,
  and `field_index` semantics are unaffected. Cards still pack left-to-right
  in the engine; the layout decoupling stays in the frontend.
- **Browser/web build** — `<CanvasScaler>` should be gated by
  `VITE_BUILD_TARGET === 'desktop'` since this is dev tooling for the
  desktop client; the browser build can remain responsive for now.
- **No backend, RL, or DB impact.** This change is confined to the desktop
  shell + frontend.
