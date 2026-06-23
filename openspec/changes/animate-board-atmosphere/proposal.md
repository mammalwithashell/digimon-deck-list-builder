## Why

The in-game board already renders the right ingredients for a "digital" feel — a grid
mat, corner binary text, a horizon glow, scanlines, and a vignette — but they are a
still photograph: nothing moves. Now that the menu atmosphere engine exists
(`add-live-theme-atmosphere`) and a motion gate exists
(`add-effects-and-motion-settings`), we can make the board breathe during a match
without building anything new — by animating the layers that are already there and
reusing the same engine. The hard constraint is subtlety: the board must feel alive
but never compete with the game.

## What Changes

- Animate the board's existing static atmosphere layers in `GameBoard`:
  - the corner binary (`__binary` / `BinaryWallpaper`) becomes actual calm digital
    rain via the reused `LiveAtmosphere` engine (board surface variant);
  - the scanlines (`__scanlines`) gain a slow roll;
  - the grid mat (`__mat`) gains a slow drift.
- Tuned **subtler than menus**: lower intensity, strictly behind cards/chrome and
  behind game-event VFX, so it never distracts during play.
- Gated by `add-effects-and-motion-settings`: animates only when effective
  live-background is on; at `reduced`/`off` or toggle-off the board renders its current
  static look (no regression).
- Performance-bounded for the fixed 1920×1080 internal canvas: the rain canvas is
  sized to the internal board, frame-rate capped, and paused when the document is
  hidden (inherited from the atmosphere engine).
- Does not change the existing event-driven VFX (digivolve/battle/security/phase) or
  the board layout.

No engine, gameplay, or network changes. Frontend-only, desktop-targeted.

## Capabilities

### New Capabilities
- `board-atmosphere-animation`: the in-game board's pre-existing atmosphere layers
  (binary rain, scanlines, grid mat) animate subtly when effective live-background is
  on, reusing the shared atmosphere engine, strictly behind gameplay and within a
  fixed-canvas performance budget.

### Modified Capabilities
<!-- None at the requirement level. The board's event-driven VFX and layout are
     unchanged; this animates currently-static decorative layers. -->

## Impact

- **Depends on**: `add-effects-and-motion-settings` (gate) and
  `add-live-theme-atmosphere` (reused engine / board surface variant).
- **UI**: `code/frontend/src/components/board/GameBoard.tsx` — replace/augment the
  static `BinaryWallpaper` with the reused atmosphere engine (board surface);
  `code/frontend/src/index.css` — add gated roll/drift animations to `.ib-board__scanlines`
  and `.ib-board__mat`.
- **Z-order**: keep atmosphere on the existing negative `z-index` band so it stays
  behind permanents, chrome, the memory gauge, and event VFX.
- **Performance**: canvas sized to the internal 1920×1080 board (not the scaled
  window); FPS cap + hidden-tab pause from the engine.
- **Tests**: board atmosphere animates when gated on; static when off; does not alter
  board layout or VFX layering.
