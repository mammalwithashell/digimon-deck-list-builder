## Context

The theme is on `data-theme`; the motion + live-background preferences (from
`add-effects-and-motion-settings`) resolve to an effective live-background boolean
(`motion === 'full' && liveBackground`). `MenuShell` renders `.menu-shell` →
`.menu-shell__content` (`<Outlet/>`); it does **not** currently mount `.ds-backdrop`,
which only appears in the style guide. The board (`.ib-board`) has its own static
atmosphere layers (`__mat`, `__binary`, `__scanlines`, `__horizon`, `__vignette`) and
is full-bleed (not in `MenuShell`). This change owns the menu atmosphere and produces
the reusable engine the board change will consume.

## Goals / Non-Goals

**Goals:**
- Two distinct, on-theme live backgrounds for the menu shell.
- Calm by default (slow/sparse rain) with speed/density as named tunables.
- One reusable component the board change can drive.
- Strict gating (effective live-background) + static fallback + hidden-tab pause.

**Non-Goals:**
- The in-game board wiring itself (separate change; this only provides the engine).
- Cursor-follow lighting (separate change).
- New tokens or theme additions.
- Mobile/touch tuning (desktop target).

## Decisions

### Decision: One `LiveAtmosphere` component, two theme variants, a `surface` prop
A single component renders the correct variant from `data-theme` and accepts a
`surface` ('menu' | 'board') so the board change reuses it with board-appropriate
sizing/intensity. Centralizing avoids two divergent rain implementations. Alternative
considered: separate menu and board components — rejected (duplicate logic, drift).

### Decision: Canvas for the dark rain, CSS for the light idle scene
Digital rain (many moving glyph columns) is cheapest and smoothest on a `<canvas>`
with a capped frame rate; the light "idle desktop" (breathing gradient, parallax
dots, blink cursor, sweep) is well expressed with CSS animations/transforms and needs
no canvas. The component picks the renderer by variant. Alternative considered:
all-CSS rain (column elements) — rejected as heavier in the DOM and harder to keep
calm/performant; all-canvas light scene — unnecessary.

### Decision: Calm is the default; speed/density are explicit constants
Per direct feedback, the rain must feel like ambient weather, not a screensaver. Fall
speed and column density are named constants (e.g. `RAIN_SPEED`, `RAIN_DENSITY`) tuned
slow/sparse by default, with headroom to lift into a future "effects intensity"
setting. The demo's fast rain is explicitly not the target.

### Decision: Gate + static fallback, single mechanism
When effective live-background is on, the live renderer runs; otherwise the component
renders a static version of the same scene (no animation) so the look is consistent
across motion levels. Gating reads the effective boolean from the foundation change.
The existing `.ds-backdrop` static atmosphere is reconciled so the live layer
supersedes it when active and the static fallback matches it when not — no
double-rendered scanlines/grid.

### Decision: Pause when not visible
Subscribe to `visibilitychange`; stop the rAF loop / canvas draw when the document is
hidden and resume on return. This keeps idle menus from burning cycles in the
background.

### Decision: Behind content, non-interactive
The atmosphere is `pointer-events: none` and sits behind `.menu-shell__content`
(z-order below content, above the base surface), like the cursor light but lower.

## Risks / Trade-offs

- [Canvas rain cost on the Tauri webview] → cap FPS (calm rain needs ~15–24fps),
  size the canvas to the shell (not full desktop), redraw with a trailing-fade fill,
  and pause when hidden.
- [Double atmosphere if `.ds-backdrop` and `LiveAtmosphere` both render] → explicitly
  reconcile: the live component is the single source of menu atmosphere; the static
  `.ds-backdrop` styles become the fallback the component renders when gated off.
- [Light "idle desktop" reading as busy/distracting] → keep motions slow and
  low-contrast; the sweep is occasional (long interval), the blink is a single cursor.
- [Reuse coupling with the board change] → keep the `surface` prop the only board-
  specific knob; the board change owns its own mounting and intensity, not this one.
- [Flash of wrong/again-static background on load] → gate in CSS where possible and
  render the static fallback as the default so the first paint is never empty.

## Open Questions

- Should the light "analyzer sweep" be tied to anything (route change, periodic) or
  purely ambient on a long timer? Default: ambient long timer.
- Exact default `RAIN_SPEED`/`RAIN_DENSITY` values — tuned during implementation
  against the "ambient weather" bar; capture the chosen constants in the component.
- Whether to expose intensity now or wait for a future settings change. Lean: constants
  now, settings later.
