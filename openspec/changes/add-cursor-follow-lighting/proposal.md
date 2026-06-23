## Why

DigiLab's UI feels alive partly because a soft light follows the cursor. Our menus are
currently static surfaces. A pointer-tracked light is the single highest
payoff-to-effort effect we can add: it makes the whole app feel responsive and
"digital" the moment the user moves the mouse, with no new assets and a small,
self-contained surface. It is the natural first feature on top of the motion
foundation (`add-effects-and-motion-settings`).

## What Changes

- Add a cursor-follow light: a single `pointer-events: none` overlay in the desktop
  menu shell that tracks the pointer and renders a soft radial glow around it.
- Per-theme tint: dark "Digi-OS" gets an electric green→orange halo; light
  "Adventure '99" gets a soft white/teal screen-sheen (like glare on the laptop).
- Pointer tracking is `requestAnimationFrame`-throttled and writes CSS custom
  properties (`--cursor-x`/`--cursor-y`) consumed by a CSS radial gradient — no React
  re-render per move.
- The effect is gated by the motion preference from
  `add-effects-and-motion-settings`: it renders only at motion `full`, and is absent
  at `reduced`/`off`.
- Scope is the **menu shell only** (Home, Play, Decks, Patch Notes, Graphics, Models).
  The full-bleed in-game board route is intentionally excluded in this change.

No engine, gameplay, or network changes. Frontend-only, desktop-targeted.

## Capabilities

### New Capabilities
- `cursor-follow-lighting`: a pointer-tracked, theme-tinted, motion-gated light overlay
  in the desktop menu shell that brightens the area around the cursor without
  intercepting input.

### Modified Capabilities
<!-- None. This change reads the motion gate introduced by
     `add-effects-and-motion-settings` but does not change its requirements. -->

## Impact

- **Depends on**: `add-effects-and-motion-settings` (reads `data-motion` / the
  effective-motion accessor).
- **UI**: new component (e.g. `code/frontend/src/components/layout/CursorLight.tsx`)
  mounted inside `code/frontend/src/components/layout/MenuShell.tsx`; the in-game
  `GamePage`/`GameBoard` are deliberately untouched.
- **Styles**: a small CSS block (in `MenuShell.css` or a dedicated file) defining the
  radial-gradient layer and the two per-theme tints, keyed on `data-theme` and
  `data-motion`.
- **Tokens**: reuses existing role tokens (`--accent`, `--accent-glow`, `--player`,
  theme-stable hues) — no new tokens.
- **Tests**: a render/behavior test (overlay mounts at motion `full`, absent at
  `reduced`/`off`, `pointer-events: none`).
