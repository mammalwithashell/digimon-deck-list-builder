## Why

DCGO's menus feel "digital" because the background is alive. Ours are static. Each of
our two themes already maps to a distinct fictional computer — dark "Digi-OS" (inside
the machine: terminal/CRT/digital rain) and light "Adventure '99" (the kids' laptop
OS: teal desktop, parchment windows). Bringing those backgrounds to life is the
biggest single step toward the app feeling alive, and it gives us a reusable
atmosphere engine the in-game board change can share.

Note: the existing `.ds-backdrop` atmosphere (scanlines / dot-grid + CRT scan) is only
mounted in the style guide today — the real menu shell renders no atmosphere. This
change adds a live atmosphere to the actual shell.

## What Changes

- Add a reusable `LiveAtmosphere` component (a non-interactive, behind-content layer)
  with two theme variants, mounted behind the desktop menu shell content.
- **Dark "Digi-OS" variant**: a calm, slow digital rain plus a faint drifting grid —
  ambient weather, not a screensaver. Rain speed and density are explicit tunables and
  default to slow/sparse.
- **Light "Adventure '99" variant**: an idle laptop desktop — a slowly breathing teal
  gradient, a gently parallaxing dot-grid, a blinking terminal cursor, and an
  occasional "analyzer" sweep line (DA28/ZT21 reference).
- Gate the live rendering on `add-effects-and-motion-settings`: it animates only when
  effective live-background is on (motion `full` **and** the Live-background toggle
  on); otherwise a static fallback renders the same scene without motion.
- Pause animation when the document/tab is hidden (performance).
- Make the component reusable by a `surface` distinction so the later
  `animate-board-atmosphere` change can drive the in-game board's atmosphere layers
  from the same engine.

No engine, gameplay, or network changes. Frontend-only, desktop-targeted.

## Capabilities

### New Capabilities
- `live-theme-atmosphere`: a reusable, theme-variant, motion-gated animated background
  for the menu shell — calm digital rain for dark, an idle laptop-desktop scene for
  light — with a static fallback and a hidden-tab pause.

### Modified Capabilities
<!-- None. Reads the motion / live-background gate from
     `add-effects-and-motion-settings` without changing its requirements. -->

## Impact

- **Depends on**: `add-effects-and-motion-settings` (effective live-background gate +
  motion accessor).
- **UI**: new `LiveAtmosphere` component (likely `code/frontend/src/design/components/`
  or `components/layout/`), mounted behind content in
  `code/frontend/src/components/layout/MenuShell.tsx`.
- **Styles**: reconcile with the existing `.ds-backdrop` static atmosphere
  (`code/frontend/src/design/components/components.css`) so the live layer supersedes
  the static one when active and falls back to it when not — no double-rendered
  scanlines.
- **Tokens**: reuses theme role tokens (`--surface-bg`, `--accent`, `--grid-line`,
  `--screen-*`); no new tokens expected.
- **Reuse**: the component/engine is consumed by `animate-board-atmosphere` for the
  in-game board.
- **Tests**: renders live layer when gate on; renders static fallback when off; pauses
  on `visibilitychange`.
