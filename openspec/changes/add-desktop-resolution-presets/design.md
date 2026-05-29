## Context

The desktop client today sets a single Tauri window default
(1280×800, min 1024×768, resizable freely) and uses responsive CSS inside
the game board — flex/grid layouts plus a single `@media (max-width: ...)`
breakpoint that shrinks `.ib-battle-slot` and `.ib-battle-area` at narrow
widths. That model has produced a concrete bug: at common window sizes the
battle area's 14 slots wrap into 3 rows (the `.ib-battle-area` grid uses
`repeat(6, ...)`), and the third row collides with the memory gauge or the
player's hand.

DCGO, our reference TCG client, takes the opposite approach: a fixed list
of 8 supported window sizes (1024×576 → 5160×2160), a single authored UI
that always renders at one design resolution internally, and CSS-level
scaling to fit whatever window the user picks. Layout never reflows; the
whole canvas just gets bigger or smaller. This change adopts the same
shape for our desktop build.

The Rust engine models the battle area as `Vec<Permanent>` — packed,
`field_index` = position in the Vec. Cards always play to `len()` (the
end), and removal of a middle card shifts subsequent indices left.
"Permanents stay where dropped" is *not* DCGO's behavior either; DCGO
auto-packs left-to-right just like the engine does. Our visual bug isn't
the packing — it's the row wrap. So this change fixes the layout to
2 rows × 7 columns, keeps engine packing, and adds a slide animation
for the rare case where a middle card is removed and survivors shift.

## Goals / Non-Goals

**Goals:**
- Provide a Graphics Settings page that lets the user pick from 8 DCGO
  resolution presets plus a fullscreen toggle, with the selection
  persisting across launches.
- Adopt a fixed 1920×1080 internal canvas with uniform CSS-transform
  scaling so the board renders identically (modulo size) at every preset.
- Make the battle area always render as 2 rows × 7 columns of slots,
  regardless of window size.
- Animate engine-driven slot shifts (cards sliding left after a middle
  card dies) so the layout never visually teleports.
- Letterbox ultrawide (3440×1440) by centering the 16:9 canvas with side
  bars, matching DCGO.

**Non-Goals:**
- Changing the engine's `Vec`-based battle-area model or adding a stable
  `permanent_id`. The frontend stays decoupled from engine semantics by
  keeping the layout purely positional.
- Letting the user freely drag-place cards into specific slots. DCGO
  doesn't do this; the engine doesn't model it; the action space doesn't
  encode it. Drop-target affordances stay slot-based for UX feedback,
  but the action ID still maps to "play this card" without a destination
  slot.
- Custom layouts per preset. The whole point of fixed-canvas scaling is
  to author one layout and re-use it.
- Affecting the web/browser build. Resolution presets only make sense
  in a windowed shell; the browser build remains responsive (and the
  scaler is gated by `VITE_BUILD_TARGET === 'desktop'`).
- Changing the action space, tensor encoding, or any RL contract.

## Decisions

### Decision: Fixed 1920×1080 internal canvas with CSS transform scaling

The game UI's outer wrapper is a new `<CanvasScaler>` component that
always renders a 1920×1080 inner box and applies
`transform: scale(min(window.innerWidth / 1920, window.innerHeight / 1080))`
with `transform-origin: top left`. The inner box is then translated to
center it horizontally and vertically within the window so ultrawide
windows produce symmetric side bars.

**Alternatives considered:**

- *Continue with responsive CSS at every breakpoint.* Rejected — every
  new resolution is a new layout to test and maintain, and we already
  have a visible bug from this approach.
- *CSS container queries + intrinsic sizing.* Rejected — same maintenance
  burden, and we'd still need media queries inside the canvas. The
  user explicitly asked for DCGO behavior; DCGO is fixed-canvas.
- *Author at 1280×720 to match DCGO's vintage.* Rejected after user
  confirmation. Authoring at 1920×1080 keeps card art crisp on 1080p+
  displays (which is most users) and the scaling math is clean:
  0.5× at 1024×576, 1× at 1920×1080, 2× at 3840×2160.

### Decision: Use Tauri's WebviewWindow API for window sizing, not a Rust command

Resolution presets are applied from the frontend via
`@tauri-apps/api/window`'s `appWindow.setSize(new LogicalSize(w, h))`
and `appWindow.setFullscreen(bool)`. No new Tauri command is required.

**Rationale:** the JS API already exists; the values are static (no
server roundtrip needed); the settings page is React-only. Adding a
`set_window_preset` command would just be a thin wrapper that adds
indirection.

**Alternatives considered:**

- *Custom Tauri command per preset.* Rejected — adds Rust code without
  improving anything. The JS API is the canonical path.
- *Edit `tauri.conf.json` per build to lock to one resolution.* Rejected
  — the user wants runtime selection, not build-time.

### Decision: Persist preset via localStorage + Tauri's window-state plugin

Settings live in two places, deliberately:

1. **`localStorage.desktop.graphicsPreset`** — the user's chosen preset
   (the explicit selection), restored by `<CanvasScaler>` on mount and
   applied via `appWindow.setSize()`.
2. **Tauri's window state plugin** (`tauri-plugin-window-state`) —
   captures actual window position + size at close, restores on launch.
   Used to remember where the window was on a multi-monitor setup.

**Rationale:** localStorage is the source of truth for the *preset
selection* (semantic). Window-state plugin restores *position* (chrome
behavior the user expects from any desktop app). They don't conflict
because the preset selection wins at startup for size; position is
independent.

**Alternatives considered:**

- *Tauri Store plugin for everything.* Reasonable but heavier — we'd
  introduce a new dependency for a single string value. localStorage is
  sufficient and already available.
- *Embed preset in `tauri.conf.json` and reload.* Rejected — requires
  app restart on preset change, terrible UX.

### Decision: Battle area becomes `repeat(7, ...)` columns

Single-line CSS change in `code/frontend/src/index.css`:

```css
.ib-battle-area {
  grid-template-columns: repeat(7, minmax(96px, 1fr));
  grid-template-rows: repeat(2, 1fr);
  /* width is fixed because canvas is fixed */
}
```

7 × 2 = exactly 14 slots, matching `MAX_BATTLE_AREA_SLOTS = 14`. No
JavaScript change in `BattleArea.tsx` — it still iterates 0..14 and the
grid auto-flow handles placement.

**Alternatives considered:**

- *`grid-auto-flow: column dense` with `grid-template-rows: repeat(2, ...)`.*
  Equivalent visually; chose row-major (`repeat(7, ...)` cols) because
  it matches the natural "left-to-right play order" reading direction.
- *Two separate flex rows.* Rejected — grid is cleaner for slot alignment.

### Decision: FLIP animation for slot shifts after deletion

When the engine emits a state update that shifts permanent indices left
(because a middle card was deleted), animate the surviving cards sliding
to their new positions over ~250ms.

Implementation uses the "FLIP" technique (First Last Invert Play) — a
small custom hook `usePositionTransitions` that:
1. Captures `getBoundingClientRect` for each slotted permanent before
   render (First).
2. Lets React commit the new positions (Last).
3. Computes the delta and applies `transform: translateX(-delta)` to
   start (Invert).
4. Animates `transform: translateX(0)` (Play) on next frame.

Keyed by `(perm.topCardId, perm.turnPlayed)` — stable enough for the
common case; if it collides (two same-named cards same turn), we just
lose the animation for that pair, which is acceptable.

**Alternatives considered:**

- *Engine `permanent_id`.* Rejected — out of scope per proposal. Would
  give perfect identity tracking but requires touching engine,
  PermanentDto, all DTO populators, and the binding crate.
- *CSS transitions on layout-shifting elements.* Doesn't work cleanly
  because grid placement is discrete; you can't transition between
  grid cells, only continuous properties.
- *Framer Motion `<Reorder>`.* Heavy dep for one animation; FLIP is
  ~30 lines.

### Decision: Letterbox ultrawide by computing the smaller scale axis

The 3440×1440 preset is 21.5:9. With a 16:9 internal canvas:

```
scale = min(window.innerWidth / 1920, window.innerHeight / 1080)
      = min(3440/1920, 1440/1080)
      = min(1.79, 1.33)
      = 1.33
```

So the canvas renders at 1920 × 1.33 = 2560 wide and 1080 × 1.33 = 1440
tall, centered in the 3440-wide window → 440px black bars on each side.

The `<CanvasScaler>` wrapper sets:
```css
background: black;            /* the letterbox color */
display: flex;
align-items: center;
justify-content: center;
```

**Alternatives considered:**

- *Stretch to fill width on ultrawide.* Rejected — distorts the board.
- *Author a separate 21:9 layout.* Rejected — defeats the purpose.

### Decision: Disable user resize; presets are the only size control

Set `resizable: false` in `tauri.conf.json` plus
`appWindow.setResizable(false)` at runtime. The Graphics Settings page
is the only path to change window size.

**Rationale:** dragging the window edge isn't a useful affordance once
the canvas is fixed-scale — you'd just produce non-preset sizes that
look identical to the nearest preset, plus a window-state confusion. By
locking resize, every desktop session is at a known preset.

**Trade-off:** users who want fine-grained custom sizes can't get them
without code changes. Acceptable — DCGO has the same constraint.

## Risks / Trade-offs

- **[Risk] Tauri window-state plugin not yet a project dependency** →
  Mitigation: if adding the plugin proves invasive, fall back to
  localStorage-only persistence and skip position memory.

- **[Risk] FLIP animation conflicts with existing card-play animation**
  (`.animate-card-play-in` in `BattleArea.tsx`) → Mitigation: gate FLIP
  on "card already existed last render" (i.e., the existing
  `prevCardIds` mechanism); play-in animation continues to fire for new
  arrivals only.

- **[Risk] Fixed canvas at 1024×576 makes UI text painfully small** →
  Mitigation: 1024×576 is the smallest preset and matches DCGO exactly;
  if testing shows it's unusable we'll either drop it or treat it as
  "minimum viable" parity with DCGO.

- **[Trade-off] No mid-flight resize.** Once a preset is chosen, the
  window is locked to that size until the user opens settings again.
  Some users may want to drag-resize for one-off needs. Accepted —
  matches DCGO and simplifies the model.

- **[Trade-off] Slot-shift animation has heuristic identity tracking.**
  Two cards with the same `(topCardId, turnPlayed)` will animate
  imperfectly. Edge case; acceptable until we revisit engine
  `permanent_id`.

- **[Risk] Web/browser build regressions.** The `<CanvasScaler>` gate
  must be tight — if it fires in the browser build we'd break the
  responsive web layout. Mitigation: gate on `VITE_BUILD_TARGET` at
  the component level and verify the browser build still renders a
  responsive board.

## Migration Plan

This change ships as one PR. There's no backward compatibility concern
— the desktop client has no shipped users yet beyond developers, and
no persisted state from the old responsive model needs migration. On
first launch after install, `<CanvasScaler>` reads `localStorage`,
finds no preset, defaults to 1280×720 (the second preset and the
existing Tauri default size), applies it, and writes it back. No
explicit migration step is required.

Rollback: revert the PR. Frontend reverts to responsive CSS; Tauri
window config reverts to free-resize. No data loss.

## Open Questions

- **Should the Graphics Settings page be accessible mid-game?** Resizing
  the window mid-match would visually jump but engine state is
  untouched. Probably fine — defer the call to first implementation
  review.

- **Default fullscreen behavior on first launch.** Off (windowed at
  1280×720) is the safer default; fullscreen-on by default could
  surprise users on multi-monitor setups. Going with off-by-default
  unless testing surfaces a reason to flip.

- **Apply preset immediately on click vs require an "Apply" button?**
  DCGO applies immediately. Going immediate unless prototype testing
  surfaces accidental clicks as a problem.
