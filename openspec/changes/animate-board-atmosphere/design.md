## Context

`GameBoard` (`code/frontend/src/components/board/GameBoard.tsx`) renders, on a negative
`z-index` band, `__mat` (grid + dot matrix), `BinaryWallpaper` → `__binary` (static
corner binary text), `__horizon` (glow line), `__scanlines` (static), and `__vignette`
— all defined in `.ib-board` in `code/frontend/src/index.css`. The board route is
full-bleed (not wrapped by `MenuShell`) and renders inside a fixed 1920×1080 internal
canvas scaled by `CanvasScaler`. Event-driven VFX (digivolve/battle/security/phase)
sit on higher `z-index` bands and are driven by `store.events` (rule 15). The shared
`LiveAtmosphere` engine and the effective live-background gate already exist from the
prior two changes.

## Goals / Non-Goals

**Goals:**
- Make the board's existing static atmosphere move, subtly, during a match.
- Reuse the `LiveAtmosphere` engine (board surface) rather than a second rain.
- Stay strictly behind gameplay and within the fixed-canvas perf budget.
- No regression: gated off ⇒ exactly today's static board.

**Non-Goals:**
- New gameplay VFX or changes to digivolve/battle/security/phase effects.
- Board layout changes.
- Menu atmosphere (owned by the prior change).
- Audio.

## Decisions

### Decision: Reuse `LiveAtmosphere surface="board"` for the rain
Replace the static `BinaryWallpaper` with the shared engine's board variant so there is
one rain implementation. The board variant uses board-appropriate sizing/intensity
(lower density, sized to the internal canvas). Alternative considered: keep
`BinaryWallpaper` and animate it in place — rejected (a second rain implementation to
maintain; the whole point of the engine's `surface` prop was board reuse).

### Decision: Scanline roll and grid drift are CSS animations on the existing layers
`__scanlines` gains a slow vertical roll and `__mat` a slow background-position drift,
both as CSS keyframes gated by `data-motion`/effective live-background. These are
cheap, compositor-friendly, and need no canvas. Alternative considered: fold them into
the canvas — rejected (unnecessary; CSS is cheaper and keeps the layers independently
tunable).

### Decision: Subtler than menus, strictly behind gameplay
Board atmosphere intensity is dialed below the menu defaults (lower rain density,
lower scanline/drift amplitude). It stays on the existing negative `z-index` band so
permanents, chrome, the memory gauge, and all event VFX render above it. Legibility of
the board state is the priority; atmosphere is texture, not focus.

### Decision: Single gate, current look as the fallback
Animate only when effective live-background is on. At `reduced`/`off` or toggle-off the
board renders exactly its current static layers — the existing look is the fallback,
so there is no regression and nothing new to design for the gated-off path.

### Decision: Canvas sized to the internal board, not the window
The rain canvas matches the 1920×1080 internal coordinate space (then scaled by
`CanvasScaler` with everything else), so it never reflows and the draw cost is fixed
regardless of window size. FPS cap and hidden-tab pause come from the engine.

## Risks / Trade-offs

- [Atmosphere distracts during play] → intensity tuned well below menus; behind all
  gameplay layers; validated in real matches at both themes.
- [Canvas draw cost on top of an active game on the Tauri webview] → fixed internal
  size + capped FPS (calm rain ~15–24fps) + pause when hidden; profile during a live
  game, not just an idle board.
- [Z-order regressions hiding cards or VFX] → keep the existing negative `z-index`
  band; add a test/asserted ordering that permanents and event VFX render above
  atmosphere.
- [Double rain if both `BinaryWallpaper` and the engine mount] → remove/disable
  `BinaryWallpaper` when the engine renders the board rain; the engine is the single
  source.
- [Scaling interaction with `CanvasScaler`] → the canvas lives inside the scaled board
  subtree so it scales uniformly; verify it tracks the transform like other layers.

## Open Questions

- Keep `BinaryWallpaper` as the gated-off static fallback, or let the engine render its
  own static frame for the board too? Lean: engine renders the static fallback for
  consistency, retire `BinaryWallpaper` once parity is confirmed.
- Final board intensity constants — set during implementation against the "texture not
  focus" bar, in a real match.
