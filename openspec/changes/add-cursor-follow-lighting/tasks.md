## 1. Component

- [ ] 1.1 Create `code/frontend/src/components/layout/CursorLight.tsx`: a single `pointer-events: none` overlay div that fills the shell.
- [ ] 1.2 Attach a pointermove listener (on the shell root or window) that stores the latest coords and applies them once per `requestAnimationFrame` as `--cursor-x`/`--cursor-y` CSS custom properties; clean up the listener and any pending rAF on unmount.
- [ ] 1.3 Read the effective motion level (from the `add-effects-and-motion-settings` accessor); do not attach the listener or render the layer when motion ≠ `full`.

## 2. Styles

- [ ] 2.1 Add CSS for `.cursor-light`: a radial gradient positioned at `var(--cursor-x) var(--cursor-y)`, layered above the shell background but below content (`z-index`), `pointer-events: none`.
- [ ] 2.2 Prefer a compositor-friendly implementation (translate a fixed gradient layer via `transform`) over animating gradient center/`background-position`; add `will-change` as needed.
- [ ] 2.3 Define the dark-theme tint (`--accent`→`--player` low-alpha halo) and the light-theme sheen under `[data-theme="dark"]` / `[data-theme="light"]`.
- [ ] 2.4 Gate the layer in CSS so it only paints under `[data-motion="full"]`.

## 3. Wiring

- [ ] 3.1 Mount `<CursorLight/>` inside `MenuShell` so it covers all menu routes with one instance and naturally excludes the full-bleed board route.
- [ ] 3.2 Confirm z-order: light sits above `.menu-shell` background, below `.menu-shell__content`, and below any modal/overlay layers.

## 4. Tests

- [ ] 4.1 Render test: at motion `full` the overlay mounts with `pointer-events: none`; at `reduced`/`off` it is absent.
- [ ] 4.2 Theme test: the tint class/vars switch with `data-theme`.
- [ ] 4.3 Pass-through test: a click on a control beneath the overlay still fires.

## 5. Verification

- [ ] 5.1 Manual pass per theme at motion `full`: light tracks smoothly, reads as on-theme, text stays legible.
- [ ] 5.2 Confirm the board route shows no cursor light, and `reduced`/`off` shows none.
- [ ] 5.3 Dev perf check: no per-move React renders; paint cost acceptable on the Tauri webview.
- [ ] 5.4 Typecheck + frontend tests green; `openspec validate add-cursor-follow-lighting`.
