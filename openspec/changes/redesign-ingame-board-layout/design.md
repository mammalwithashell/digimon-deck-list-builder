## Context

The in-game board (`code/frontend/src/components/board/GameBoard.tsx` +
`.ib-board*` rules in `code/frontend/src/index.css`) lays out its major regions
with **absolute pixel offsets** measured from the board container's edges:

- opponent field `top: 78px`
- memory gauge `top: 50%` + `translate(-50%, -50%)` (centered)
- player field `bottom: 154px`, with the player raising-area additionally shoved
  down by `transform: translateY(56px)`
- hand `bottom: 8px`

These offsets implicitly assume `.ib-board` is the full height of the 1920×1080
design canvas. It isn't: the NavBar (~56px) and the footer bars
(`BotSpeedControl` + Seed readout + `ActionBar`, ~60–90px), which live in
`GamePage`'s flex column *below* the board, shrink `.ib-board` well under 1080px.
Because the band offsets are fixed pixels, the lost height is absorbed entirely
by the **bottom band** — the player field gets a much smaller slice than the
opponent field (the gauge is pinned at the geometric 50% of a now-shorter box),
producing the "smushed under the gauge" look. The same height pressure pushes
the action bar below the viewport when maximized.

The desktop build wraps everything in `CanvasScaler`
(`code/frontend/src/components/desktop/CanvasScaler.tsx`): a fixed 1920×1080
design canvas uniformly scaled by `min(w/1920, h/1080)`. At a 1920×1080 window
the scale is exactly 1.0, so the compression is **not** a scaling artifact — it
is the absolute-offset CSS itself. The web build does not use CanvasScaler but
shares the same `GameBoard`.

**Key de-risking fact (verified during exploration):** the systems that care
about element geometry already measure *live* rects, not the CSS offsets:
- `AttackArrow` uses `getBoundingClientRect()` on the elements and container
  (`AttackArrow.tsx:23`) and renders as an `absolute inset-0` SVG.
- `usePositionTransitions` (FLIP card-move animations) uses
  `node.getBoundingClientRect()` (`usePositionTransitions.ts:47`).
- `SecurityRevealOverlay` / `BattleEffect` / `DigivolveBanner` render
  `absolute inset-0` full-screen, independent of band layout.
- Drag-and-drop compensates pointer deltas via `uiStore.canvasScale`, which is a
  property of the outer scale transform, not the band layout.

So converting the bands from absolute offsets to flow does not require touching
arrows, FLIP, overlays, DnD, or the scale model — they follow wherever the slots
render.

## Goals / Non-Goals

**Goals:**
- Bands share vertical space proportionally so the player and opponent halves are
  balanced, and shrink gracefully as the board container shrinks (NavBar +
  footer + any preset).
- The action bar (and required gameplay chrome) is always within the viewport,
  including maximized and the smallest preset.
- The memory gauge occupies its own band and never overlaps the field rows.
- The tensor debug badge is dev-only and dev/debug overlays never overlap
  gameplay chrome.
- Zero regression to arrows, FLIP transitions, overlays, DnD, the CanvasScaler
  model, or z-index layering, on both desktop and web builds.

**Non-Goals:**
- Changing the CanvasScaler fixed-canvas + uniform-scale model (kept as-is).
- Reworking the engine event/animation pipeline (separate exploration).
- Removing tensor-summary computation in the Rust desktop layer
  (`engine_commands.rs`) — an optional follow-up once the badge is dev-gated; out
  of scope here.
- A visual redesign of the cards, gauge, or chrome beyond what the layout change
  requires.

## Decisions

### D1 — Board regions become a flex column, not absolute offsets

Restructure `.ib-board` as a `display: flex; flex-direction: column` container
with three in-flow bands:

```
.ib-board (flex column, height: 100%, position: relative)
  ├─ opponent field   flex: 1 1 0   (min-h-0)
  ├─ memory gauge     flex: 0 0 auto
  └─ player field     flex: 1 1 0   (min-h-0)
```

The two field halves get equal `flex: 1 1 0`, so they always split the
non-gauge height symmetrically regardless of how tall `.ib-board` is. The gauge
is `flex: 0 0 auto` (its natural height) and sits between them — structurally
unable to overlap a field's rows (satisfies the gauge-overlap requirement).

Rationale: flex with `flex: 1 1 0` + `min-height: 0` is the standard,
robust way to get two equal panes that shrink together; it removes every
hardcoded offset (`top:78`, `bottom:154`, `translateY(56)`, gauge `top:50%`).
Grid (`grid-template-rows: 1fr auto 1fr`) is an equivalent alternative and was
considered — flex is chosen for closer alignment with the existing Tailwind
flex usage in `GamePage` and simpler `min-h-0` handling, but the spec is
satisfied by either.

### D2 — One outer flex column owns board + footer + action bar

`GamePage`'s game view is a single `flex flex-col` with:

```
outer column (height = canvas/viewport)
  ├─ board container   flex: 1 1 0; min-h-0; overflow hidden  (hosts .ib-board)
  ├─ optional panels   flex: 0 0 auto  (BotSpeedControl, Seed)   [shrink: 0]
  └─ ActionBar         flex: 0 0 auto                            [shrink: 0]
```

The action bar and footer panels are `flex-shrink: 0`, and the board container
is the only `flex: 1` element, so the board absorbs all slack and the action bar
is always laid out within the column (fixes the maximized clipping). The
existing `.ib-board { min-height: 620px }` is removed or lowered so the column
can compress below 620px at small presets instead of overflowing.

Rationale: the clipping bug and the smushing bug are the same root cause (fixed
heights in a shorter-than-canvas box); making the board the sole flex-grow
element and the chrome shrink-0 resolves both at the container level.

### D3 — Preserve overlays, arrows, FLIP, DnD, scale model unchanged

`.ib-board` keeps `position: relative` so the existing `absolute inset-0`
overlays (arrows, security/battle/digivolve banners) still anchor to it. All
geometry consumers measure live rects, so no code change is needed for them.
CanvasScaler and `canvasScale` are untouched. The non-band absolute children
that are genuinely overlays (attack arrow SVG, banners, the hand-count chip)
stay absolute; only the **layout bands** move to flow.

### D4 — Hand band keeps its "cards hang into the field" aesthetic

The hand currently sits `absolute bottom:8px` and lets cards overlap upward into
the field via negative card margins. Decision: make the hand a flow band at the
bottom of the player region (or the outer column), but allow upward visual
overlap via negative margin / `overflow: visible` so it does not consume extra
layout height. This keeps the aesthetic while removing the absolute anchor that
fought the bands for space.

Rationale: keeping the hand absolute would reintroduce a fixed bottom anchor;
putting it in flow with controlled negative-margin overlap is the smallest
change that satisfies the proportional-layout goal without a visual redesign.

**Implemented decision (deviation):** in implementation the hand was kept
`absolute bottom:8px` and the new flex stage instead *reserves* the hand's
space via `bottom:146px`. This is strictly lower-risk than moving the hand into
flow (the overlap aesthetic is preserved byte-for-byte and there is no
negative-margin-in-flow tuning), while still removing the absolute-offset
conflict that smushed the player field. The hand is an overlay anchored to the
board's bottom edge; only the three bands (opp field / gauge / player field)
became flow. Net effect on the spec is identical (proportional bands, hand
unaffected).

### D5 — Dev chrome: gate + dedicated region

- `TensorDebugBadge` is rendered only under `import.meta.env.DEV` (gating
  requirement). In production/desktop release it is absent.
- When shown, dev overlays live in a **dedicated dev region** (e.g. a stacked
  rail anchored top-left, or a single corner distinct from the bottom-right
  gameplay chrome) so the tensor badge never shares an anchor with the
  hand-count chip. The hand-count chip remains the bottom-right gameplay
  element.
- The Seed readout is gameplay-useful (repro), so it stays available, but is
  condensed/relocated as part of de-crowding the footer; it is not required to
  be dev-gated. (Open question below.)

### D6 — Validation across presets and builds

Because there are no automated visual tests here, validation is manual + the
`run-desktop` recipe: render at each `RESOLUTION_PRESET` (1024×576 … 1920×1080),
maximized, and in the web build, confirming (a) balanced halves, (b) gauge clear
of rows, (c) action bar visible, (d) arrows/FLIP/overlays/DnD intact, (e) tensor
badge absent in a production build and non-overlapping in dev.

## Risks / Trade-offs

- **Hand-overlap aesthetic regresses when moved into flow** → Mitigation: use
  negative margin / `overflow: visible` so the hand band renders its upward
  overlap without consuming layout height; verify the "cards hang over the field
  edge" look at each preset before/after.
- **First-render FLIP jump** — moving bands to flow could trigger a one-time
  `usePositionTransitions` animation on mount → Mitigation: FLIP keys are
  per-card identity; a band-level reflow shouldn't register card moves, but
  verify no spurious slide on game start.
- **Other absolute children assume old offsets** (chips/banners positioned
  relative to the old band coordinates) → Mitigation: audit every `.ib-board__*`
  absolute rule; re-anchor any that referenced the removed offsets.
- **Web build divergence** (no CanvasScaler, different available height) →
  Mitigation: the flex model is resolution-agnostic by design; include the web
  build in the validation matrix.
- **Small presets clip if `min-height: 620px` is simply removed** → Mitigation:
  set a sensible `min-height: 0` on the flex children and a modest board
  min-height only if needed; test 1024×576.
- **Scope creep into a full bottom-bar redesign** → Mitigation: keep D4/D5 to
  the minimum needed to satisfy the specs; defer cosmetic chrome consolidation.

## Migration Plan

Pure frontend (CSS + TSX); no data/engine migration, no flags required.
1. Land the outer-column structure (D2) and band flex (D1, D6 gauge band).
2. Re-anchor/adjust remaining absolute overlays (D3 audit) and the hand band
   (D4).
3. Gate + relocate dev chrome (D5).
4. Validate across the preset/maximized/web matrix.
Rollback = revert the CSS/TSX diff (no state or schema changes).

## Open Questions

- Final flex ratios — strictly symmetric `1:auto:1`, or a slight bias (e.g. give
  the player field marginally more for the larger hand)? Default: symmetric.
- Is the Seed readout gameplay chrome (keep, condense) or dev-only (gate)?
  Leaning keep-but-condense; confirm with the user.
- Should the optional footer panels (BotSpeed/Seed) move into the top chrome to
  reclaim more height, or is shrink-0 in the column sufficient? Default:
  shrink-0 in the column; revisit only if the action bar is still tight at the
  smallest preset.
- Follow-up (out of scope): stop computing the per-action tensor summary in
  `engine_commands.rs` once the badge is dev-gated, to drop wasted work on the
  desktop interactive path.
