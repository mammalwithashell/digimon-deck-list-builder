## 1. Component scaffold

- [x] 1.1 Create a reusable `LiveAtmosphere` component with a `surface` prop (`'menu' | 'board'`) and theme detection via `data-theme`.
- [x] 1.2 Read the effective live-background boolean and motion level from the `add-effects-and-motion-settings` accessor; switch between the live renderer and the static fallback accordingly.
- [x] 1.3 Make the layer non-interactive (`pointer-events: none`) and position it behind content.

## 2. Dark variant — digital rain

- [x] 2.1 Implement a `<canvas>` digital-rain renderer (glyph columns + trailing-fade fill) with a capped frame rate.
- [x] 2.2 Add named tunables `RAIN_SPEED` / `RAIN_DENSITY`, defaulted slow/sparse (ambient weather, not screensaver); add a faint drifting grid behind the rain.
- [x] 2.3 Tint the rain with dark-theme tokens (`--accent` body, occasional `--player` head glyph).
- [x] 2.4 Provide a static fallback frame (one rendered state, no loop) for when the gate is off.

## 3. Light variant — idle desktop

- [x] 3.1 Implement the light scene with CSS: a slowly breathing teal gradient (`--surface-bg`) and a gently parallaxing dot-grid (`--grid-line`).
- [x] 3.2 Add a blinking terminal cursor and an occasional analyzer sweep line on a long interval.
- [x] 3.3 Provide the static fallback (gradient + dot-grid, no animation) for when the gate is off.

## 4. Lifecycle + performance

- [x] 4.1 Drive animation with a single rAF loop (canvas) / CSS animations (light); clean up on unmount.
- [x] 4.2 Subscribe to `visibilitychange`: stop the loop when hidden, resume when visible.
- [x] 4.3 Size the canvas to the shell (not the full desktop); handle resize.

## 5. Wiring + reconciliation

- [x] 5.1 Mount `<LiveAtmosphere surface="menu" />` behind `.menu-shell__content` in `MenuShell`.
- [x] 5.2 Reconcile with the existing `.ds-backdrop` static atmosphere in `components.css` so the live layer is the single source of menu atmosphere (no doubled scanlines/grid); make the static styles the fallback the component renders when gated off.

## 6. Tests

- [x] 6.1 Renders the live layer when effective live-background is on; renders the static fallback when off.
- [x] 6.2 Switches variant with `data-theme`.
- [x] 6.3 Pauses on `visibilitychange` (hidden) and resumes on visible (mock the canvas/loop).

## 7. Verification

- [ ] 7.1 Manual pass: dark rain reads as calm/ambient at the default constants; light idle desktop reads slow and non-distracting.
- [x] 7.2 Confirm gating: `reduced`/`off` or toggle-off shows static; full + toggle-on shows live; no double atmosphere.
- [ ] 7.3 Dev perf check on the Tauri webview (capped FPS, paused when hidden).
- [x] 7.4 Typecheck + frontend tests green; `openspec validate add-live-theme-atmosphere`.
