## Why

The in-game board positions its vertical regions with hardcoded absolute pixel
offsets (opponent field `top:78px`, memory gauge `top:50%`, player field
`bottom:154px`, hand `bottom:8px`, raising-area `translateY(56px)`). These
offsets assume the board fills the full 1920×1080 design canvas, but the NavBar
(56px) and the footer bars (BotSpeedControl + Seed readout + ActionBar) shrink
the board container below that — and because the offsets are fixed, the lost
height lands disproportionately on the **bottom (player) band**. The result is
two reported bugs with one root cause:

- The player's half of the board looks **smushed under the memory gauge** at
  1920×1080.
- When the window is **maximized, the action bar is pushed below the viewport /
  taskbar** and becomes unreachable.

Separately, a developer **tensor-summary badge** (`STANDARD_LITE_DECK_V2 P0
T8850 A2192 L3 …`) renders unconditionally in the bottom-right of every game,
overlapping the hand-count chip — debug telemetry that should not ship and, when
kept for dev, should not collide with gameplay chrome.

## What Changes

- Replace the absolute-positioned board bands (opponent field / memory gauge /
  player field / hand) with a **flex or grid vertical layout** so the regions
  share the available height proportionally and adapt as the board container
  shrinks (NavBar + footer bars + any preset). This fixes both the player-half
  compression and the maximized action-bar clipping.
- Guarantee the **action bar (and required gameplay chrome) is always within the
  viewport** at every supported resolution preset and when maximized.
- **Dev-gate** the tensor-summary badge behind `import.meta.env.DEV` so it never
  appears in production/desktop release builds.
- Give dev/debug overlays a **dedicated region that does not overlap gameplay
  chrome** (currently the tensor badge and hand-count chip both sit at
  `right:18px` and collide).
- Preserve all geometry-dependent behavior unchanged (attack arrows, FLIP card
  transitions, full-screen overlays, drag-and-drop, the CanvasScaler design
  canvas + uniform scale model, z-index layering).

## Capabilities

### New Capabilities
- `ingame-board-layout`: How the in-game board allocates vertical space across
  its regions (opponent field, memory gauge, player field, hand, footer/action
  bar) so the layout is balanced and fully visible across resolutions and window
  states, without relying on hardcoded pixel offsets.
- `ingame-dev-overlays`: How developer-only debug overlays (tensor summary, seed
  readout) are gated to dev builds and placed so they never overlap gameplay
  chrome.

### Modified Capabilities
<!-- None. No existing capability defines the in-game board layout or debug overlays. -->

## Impact

- **Frontend (shared by web + desktop builds):**
  - `code/frontend/src/components/board/GameBoard.tsx` — band structure + dev-badge mount.
  - `code/frontend/src/index.css` — `.ib-board*` band positioning rules (the absolute offsets) and dev-chrome rules.
  - `code/frontend/src/pages/GamePage.tsx` — the flex column wrapping board + footer bars (BotSpeedControl, Seed, ActionBar).
  - `code/frontend/src/components/board/TensorDebugBadge.tsx` — gating + placement.
- **No engine changes required** for the layout fix. (Optional follow-up: stop the desktop layer computing a tensor summary per action in `code/src-tauri/src/engine_commands.rs` once the badge is dev-gated — out of scope here.)
- **Must not regress:** `AttackArrow` and `usePositionTransitions` (both measure live `getBoundingClientRect`), `SecurityRevealOverlay`/`BattleEffect`/`DigivolveBanner` (`absolute inset-0`), the `CanvasScaler` fixed 1920×1080 canvas + `canvasScale` DnD pointer compensation, and the existing z-index stack.
- **Builds:** desktop (`VITE_BUILD_TARGET=desktop`) is the primary target; the web build shares `GameBoard` and must render correctly too.
