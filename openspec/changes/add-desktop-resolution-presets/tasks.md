## 1. Frontend: Graphics Settings store + persistence

- [x] 1.1 Add `graphicsPreset: { width: number; height: number }` and `fullscreen: boolean` to `uiStore.ts` with localStorage-backed read/write (key: `desktop.graphicsPreset`, `desktop.fullscreen`)
- [x] 1.2 Define a `RESOLUTION_PRESETS` constant in `code/frontend/src/utils/constants.ts` listing the 8 DCGO preset {width,height} tuples in the order specified
- [x] 1.3 Define `DEFAULT_PRESET = { width: 1280, height: 720 }` and `DESIGN_CANVAS = { width: 1920, height: 1080 }` constants
- [x] 1.4 Unit-test the store actions (set preset → localStorage update, hydrate on mount picks up persisted value, falls back to default when absent) — `src/stores/uiStore.test.ts` (6 tests)

## 2. Frontend: CanvasScaler component

- [x] 2.1 Create `code/frontend/src/components/desktop/CanvasScaler.tsx` that wraps `children` in a 1920×1080 fixed-pixel inner box
- [x] 2.2 Compute `scale = min(window.innerWidth / 1920, window.innerHeight / 1080)` and apply via `transform: scale(...)` with `transform-origin: top left`
- [x] 2.3 Center the inner box horizontally and vertically inside the window (flex wrapper with `align-items: center; justify-content: center; background: #000`) so ultrawides letterbox
- [x] 2.4 Subscribe to `window.resize` and recompute scale on resize/fullscreen change
- [x] 2.5 Gate the entire wrapper behind `import.meta.env.VITE_BUILD_TARGET === 'desktop'` — non-desktop builds render children directly without scaling
- [x] 2.6 On mount, read the preset from `uiStore`, call `appWindow.setSize(new LogicalSize(w, h))` and `appWindow.setFullscreen(fs)` to apply the persisted choice
- [x] 2.7 Wire `<CanvasScaler>` into `App.tsx` (desktop layout root) above the page routes
- [x] 2.8 Unit-test scale calculation for each of the 8 presets + ultrawide letterbox math — `src/components/desktop/CanvasScaler.test.tsx` (12 tests)

## 3. Frontend: Graphics Settings page

- [x] 3.1 Create `code/frontend/src/pages/GraphicsSettingsPage.tsx` with an 8-button grid laid out 2 columns × 4 rows of presets and a fullscreen toggle above
- [x] 3.2 Wire each button's click to `appWindow.setSize(new LogicalSize(w, h))` + store update + localStorage persist
- [x] 3.3 Wire the fullscreen toggle to `appWindow.setFullscreen(bool)` + store update + localStorage persist
- [x] 3.4 Mark the active preset with a visual selected state (border / glow / chip)
- [x] 3.5 Add a route entry in `App.tsx` for `/settings/graphics` (desktop-only, behind VITE_BUILD_TARGET)
- [x] 3.6 Add a navigation entry to reach the page (Desktop ▸ Graphics Settings or similar)
- [x] 3.7 Component test: clicking a preset calls setSize with correct values and updates store — `src/pages/GraphicsSettingsPage.test.tsx` (4 tests)

## 4. Tauri window config

- [x] 4.1 Update `code/src-tauri/tauri.conf.json`: change default window to `width: 1280, height: 720`, set `resizable: false`, remove `minWidth`/`minHeight`
- [x] 4.2 Add `tauri-plugin-window-state` dependency to `code/src-tauri/Cargo.toml` (or leave as a follow-up if integration is non-trivial — design.md flags this as a risk)
- [x] 4.3 Initialize the window-state plugin in `code/src-tauri/src/main.rs` so window position is restored on launch
- [x] 4.4 Verify Tauri builds (`cargo build --bin digimon-tcg`) with the new config — `cargo test --lib` 36/36 pass

## 5. CSS: Battle area grid + media-query purge

- [x] 5.1 In `code/frontend/src/index.css` change `.ib-battle-area` to `grid-template-columns: repeat(7, minmax(96px, 1fr))` and `grid-template-rows: repeat(2, 1fr)`
- [x] 5.2 Adjust `.ib-battle-slot` sizing so 7 columns fit inside the canvas's battle-area width at the design resolution (1920×1080) — minmax(96px, 1fr) lets the grid auto-fit while keeping the per-slot floor
- [x] 5.3 Remove the `@media (max-width: ...)` block targeting `.ib-battle-area`, `.ib-battle-slot`, `.ib-raise-zone__empty`, etc. — board never reflows now
- [x] 5.4 Scan `index.css` for other game-board media queries and remove ones inside the canvas (keep ones that affect desktop chrome / settings page / login if any) — verified zero `@media` rules remain in index.css
- [ ] 5.5 Visual smoke test: run desktop at 1024×576, 1280×720, 1920×1080, 3440×1440, 3840×2160 and confirm battle area always shows 2×7 grid (**manual QA — user verification**)

## 6. Slot-shift animation

- [x] 6.1 Create `code/frontend/src/hooks/usePositionTransitions.ts` implementing FLIP animation for a keyed set of elements
- [x] 6.2 Integrate into `BattleArea.tsx` keyed by `(perm.topCardId, perm.turnPlayed, slotIndex)` — capture rects before render, animate translate after commit
- [x] 6.3 Ensure the new animation does not double-fire alongside `.animate-card-play-in` (FLIP applies only when a card existed in the previous render at a different slot) — hook explicitly skips when the node has `animate-card-play-in`
- [x] 6.4 Tune duration to ~250ms with `ease-out` easing — `cubic-bezier(0.2, 0.8, 0.2, 1)` over 250ms
- [x] 6.5 Behavioral test: simulate state where engine deletes middle permanent → render snapshot shows transform applied on survivors — `src/hooks/usePositionTransitions.test.tsx` (3 tests; jsdom limitation noted in test file)

## 7. Documentation + verification

- [x] 7.1 Update `docs/ARCHITECTURE.md` (desktop section) noting fixed-canvas scaling and resolution presets — added "Window sizing and canvas scaling" subsection plus rules 6–7
- [x] 7.2 Add a test plan to the change directory's verification notes — see `openspec/changes/add-desktop-resolution-presets/verification.md`
- [ ] 7.3 Manual QA: open Graphics Settings, cycle through all 8 presets, verify window resizes correctly and battle area always shows 2×7 (**manual QA — user verification**)
- [ ] 7.4 Manual QA: toggle fullscreen on/off at multiple presets; verify canvas scales and the toggle-off returns to the last preset (**manual QA — user verification**)
- [ ] 7.5 Manual QA: kill app at 2560×1440, relaunch — verify it restores to 2560×1440 (**manual QA — user verification**)
- [ ] 7.6 Manual QA: trigger a midfield deletion (e.g., attack a middle permanent); verify slot-shift animation plays smoothly (**manual QA — user verification**)
- [x] 7.7 Run `openspec validate add-desktop-resolution-presets --strict` and resolve any structural issues — passes
- [ ] 7.8 Verify the web build still works (no scaler applied, responsive layout intact) (**manual QA — user verification**)
