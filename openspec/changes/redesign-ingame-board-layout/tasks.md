## 1. Pre-work — map the current absolute layout

- [x] 1.1 Inventory every `.ib-board*` rule in `code/frontend/src/index.css` that uses absolute positioning / fixed offsets / transforms (opponent `top:78`, gauge `top:50%`, player `bottom:154`, hand `bottom:8`, raise `translateY(56)`, `min-height:620`), noting which are layout bands vs overlays (arrows/banners/chips).
- [x] 1.2 Confirm the geometry consumers are rect-based and need no change: `AttackArrow.tsx` (`getBoundingClientRect`), `usePositionTransitions.ts` (FLIP), `SecurityRevealOverlay`/`BattleEffect`/`DigivolveBanner` (`absolute inset-0`), DnD `canvasScale` compensation.
- [ ] 1.3 Capture baseline screenshots at each `RESOLUTION_PRESET` (1024×576 … 1920×1080), maximized, and in the web build, for before/after comparison. — NOT done (deferred; only spot-checked dev launcher/menus at maximized 1920×1080).

## 2. Outer column — keep the action bar on-screen (D2)

- [x] 2.1 In `GamePage.tsx`, board container is the only `flex-1 min-h-0` element; footer panels (action choice, digivolve, BotSpeedControl, Seed) + `ActionBar` are `shrink-0` (ActionBar wrapped in a `shrink-0` div).
- [x] 2.2 `.ib-board` `min-height:620px` → `min-height:0` so the column can compress without overflowing.
- [ ] 2.3 Verify the action bar is fully visible/interactive when maximized at 1920×1080 and at the smallest preset. — code complete; LIVE VERIFY PENDING (dev Tauri build crashed on game launch — pre-existing intermittent native crash, unrelated to these CSS/JSX changes).

## 3. Board bands — proportional flex layout (D1, D6)

- [x] 3.1 Added `.ib-board__stage` (absolute `top:74 bottom:146 left:16 right:16`, `display:flex; flex-direction:column`) wrapping opponent field / gauge / player field; the two `.ib-board__side` are `flex:1 1 0; min-height:0`, the gauge is `flex:0 0 auto`. (GameBoard.tsx JSX wraps the three bands in `.ib-board__stage`.)
- [x] 3.2 Deleted the absolute band offsets (`top:78`, player `bottom:154`, gauge `top:50%`+translate, raise `translateY(56)`).
- [ ] 3.3 Verify at 1920×1080 the player and opponent halves are equivalent and the gauge is clear of both card rows. — code complete; LIVE VERIFY PENDING (see 2.3).

## 4. Hand band (D4 — implemented via the simpler reserved-space variant)

- [x] 4.1 DEVIATION FROM DESIGN: rather than moving the hand into flow, the hand (`.ib-board__hand`) stays `absolute bottom:8px` and the new flex stage reserves its space via `bottom:146px`. This preserves the hand's overlap aesthetic exactly with zero risk, and still removes the absolute-offset fight that smushed the player field. (Lower-risk than D4's negative-margin-in-flow; design.md D4 should be annotated to reflect this choice.)
- [ ] 4.2 Verify the hand renders correctly (overlap, hover/raise, drag start) and drop targets remain accurate. — LIVE VERIFY PENDING (see 2.3).

## 5. Re-anchor remaining absolute overlays (D3)

- [x] 5.1 Audited absolute `.ib-board__*` overlays/chips: opponent-hand, top-chrome, player tags, revealed zone, hand, hand-count chip, and decorative layers remain absolute (anchored to `.ib-board` edges/center) and did not reference the removed band offsets — no re-anchoring needed. The revealed zone was moved out of the flex stage in the JSX (it overlays the gauge area).
- [ ] 5.2 Verify attack arrows connect correctly, FLIP transitions animate with no spurious slide on game start, and full-screen overlays still cover the board. — LIVE VERIFY PENDING (rect-based, low risk; see 2.3).

## 6. Dev overlays — gate + non-overlapping placement (D5)

- [x] 6.1 `TensorDebugBadge` mount gated behind `import.meta.env.DEV` in `GameBoard.tsx` — absent in production/desktop release builds.
- [x] 6.2 `.ib-tensor-badge` moved from bottom-right (`right:18 bottom:54`, colliding with the hand-count chip) to bottom-left (`left:18 bottom:8`); hand-count chip stays the bottom-right gameplay element.
- [ ] 6.3 Condense/relocate the Seed footer chrome. — DEFERRED: Seed left in place but now `shrink-0` in the column (no longer steals from the board). Revisit if footer is still tight.
- [ ] 6.4 Verify in a dev build the tensor badge is visible + clear of gameplay chrome; in a production build it is absent. — code complete; LIVE VERIFY PENDING (see 2.3).

## 7. Validation matrix (D6)

- [ ] 7.1 Re-test the full matrix (each preset, maximized, web build). — PENDING (blocked by the dev-build game-launch crash this session; do in a fresh session or on the next release build).
- [x] 7.2 Frontend checks pass: `tsc --noEmit` clean; `vitest run` 148/148 passed across 29 files. No test regressions.
