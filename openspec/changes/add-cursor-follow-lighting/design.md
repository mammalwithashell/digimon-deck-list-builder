## Context

`MenuShell` (`code/frontend/src/components/layout/MenuShell.tsx`) wraps every menu
route through `<Outlet/>`; the in-game board route is deliberately not wrapped and
renders full-bleed. The theme is on `data-theme`; the motion preference (from
`add-effects-and-motion-settings`) is on `data-motion` plus an effective-motion
accessor in `uiStore`. Role tokens (`--accent`, `--accent-glow`, `--player`) already
encode the per-theme palette.

## Goals / Non-Goals

**Goals:**
- A soft light that tracks the cursor across the menu shell, tinted per theme.
- Zero per-move React renders; cheap enough to leave on continuously.
- Honors the motion preference (only at `full`) and never harms text legibility or
  intercepts input.

**Non-Goals:**
- The in-game board (separate change, stricter perf budget).
- Per-element spotlights / hover-reactive card lighting (possible later; not now).
- Touch/no-pointer handling beyond gracefully doing nothing (desktop target).
- New design tokens.

## Decisions

### Decision: One overlay div driven by CSS custom properties, not React state
A single `pointer-events: none` div fills the shell. A pointermove listener on the
shell root updates `--cursor-x`/`--cursor-y` (in px) on that element; the div's
`background: radial-gradient(... at var(--cursor-x) var(--cursor-y) ...)` follows. The
handler is `requestAnimationFrame`-throttled (store last event, apply once per frame),
so there is no React re-render and at most one style write per frame. Alternative
considered: React state + inline style per move — rejected (re-renders the subtree on
every mouse move).

### Decision: Light sits behind content, not above it
The overlay is layered above the shell background but **below** the content
(`z-index` between them). It brightens the atmosphere near the cursor rather than
tinting text. This avoids `mix-blend-mode` washing out foreground text and keeps
WCAG contrast intact. The glow still reads because menu surfaces are semi-transparent
over the background. Alternative considered: overlay above content with
`mix-blend-mode: screen/soft-light` — rejected as a legibility risk on dense text
pages (deck builder, patch notes).

### Decision: Per-theme tint via `data-theme`, gated via `data-motion`
The gradient stops are theme-scoped: dark = `--accent`(green)→`--player`(orange) low-
alpha halo; light = soft white→teal sheen. The whole layer is rendered only when the
effective motion is `full` — implemented by gating in CSS
(`[data-motion="full"] .cursor-light { … }`) and/or by not mounting the listener
below `full`. CSS gating keeps it flash-free; the component additionally skips
attaching the listener when motion ≠ `full` to avoid wasted work.

### Decision: Scope to MenuShell, mount once
Mount `<CursorLight/>` inside `MenuShell` so it covers all menu routes with one
instance and naturally excludes the board (which isn't wrapped). No router-level
conditional needed.

## Risks / Trade-offs

- [Continuous pointer tracking burns CPU/GPU] → rAF throttle + single listener +
  `will-change: background-position` (or transform-based positioning); one layer only;
  disabled entirely below motion `full`.
- [Large radial gradient repaint each frame is expensive on the Tauri webview] →
  Prefer translating a fixed-size gradient layer via `transform` (compositor-friendly)
  over animating `background-position`/gradient center; validate paint cost in dev.
- [Listener leaks on route changes] → single listener owned by the `CursorLight`
  component with proper cleanup; shell mounts once for the menu lifetime.
- [Glow invisible in light theme] → tune alpha/blend per theme during implementation;
  the light sheen may use a very subtle `mix-blend-mode: soft-light` limited to the
  background layer (not content) if a plain gradient reads too weakly.

## Open Questions

- Does the light feel better centered behind content (ambient) or as a faint
  above-content sheen on interactive cards only? Start ambient/behind; revisit if it
  reads too subtle.
- Should the glow radius/intensity be a tunable (future "effects intensity")? Out of
  scope here; keep constants, leave room to lift into settings later.
