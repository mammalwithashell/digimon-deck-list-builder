## Context

The deck builder (`pages/DeckBuilderPage.tsx` + `.css`, with pure filter logic in `features/deck-builder/deckBuilderView.ts`) is a three-column layout: a 240px preview column, a `1fr` pool grid with ~82px tiles, and a 320px deck-contents column. The selected card's effect text renders at 11px in the narrow preview column. The colour filter (`filterBuilderCards`) currently passes a card when *any* selected colour matches its primary or secondary colour (OR). There is no reset-all-filters affordance. The user compared this unfavourably to the DCGO builder, where the selected card and its effect text are large and readable and the pool shows fewer, larger tiles.

The recently merged `uiStore` (`stores/uiStore.ts`) establishes the idiom for a persisted UI preference: a `localStorage` key, a validating `loadPersisted*` loader, a `persist*` writer, and state + setter/toggle on the Zustand store (`railCollapsed`, `botSpeed`). The builder is responsive: below 1120px the preview column is hidden entirely.

## Goals / Non-Goals

**Goals:**
- Colour filter is an intersection (AND) across selected colours.
- A persisted GRID/DETAIL view toggle that re-proportions the layout without changing data or filters.
- A larger, more readable selected-card preview and effect text in both views.
- A single control that resets every card-pool filter to its default.

**Non-Goals:**
- No change to the card-pool data source, deck saving, validation, legality badges, or deck-contents behaviour.
- No new card render component or hover/zoom overlay.
- No change to the in-game board or any non-builder surface.
- No collapsible deck panel (a possible future follow-up, not in this change).

## Decisions

### Colour filter: superset test in `filterBuilderCards`
Replace the OR test with: build the card's colour identity set from `color` (+ `color2` if present) and require it to be a superset of the selected colour set. This makes single-colour selection behave exactly as today (the card's set contains the one colour) while two-colour selection requires both. Three-colour selection yields nothing because no card has three colours — an honest, self-evident result given the live result count and the new reset button.

*Alternative considered:* an "exact match" (card colours equal selected colours). Rejected — the user asked for "cards that are green **and** blue," which is the superset/contains semantics; exact match would also drop a hypothetical tri-colour card and is no closer to the intent.

### View modes: one DOM tree, a CSS class on `.bld-main`, persisted in `uiStore`
Add `deckBuilderView: 'browse' | 'inspect'` to `uiStore` following the existing persisted-preference idiom (storage key, validating loader that falls back to `'browse'`, writer, setter + toggle). `DeckBuilderPage` reads it and applies `view-browse` / `view-inspect` to `.bld-main`; the CSS defines per-class `grid-template-columns`, pool-tile `minmax`, and preview/effect sizing. Keeping a single DOM tree means the toggle is a pure restyle — no remount, no change to which cards render, no filter coupling — which directly satisfies the spec's "layout only" requirement and keeps it cheap.

*Alternatives considered:* (a) conditionally rendering two different JSX subtrees — rejected as more code, remount cost, and risk of behavioural drift between the two; (b) a local `useState` for the mode — rejected because the user asked for the choice to stick, and `uiStore` already owns persisted UI prefs.

*Naming:* the store values are `'browse'` / `'inspect'`; the user-facing labels are **GRID** / **DETAIL**. The toggle lives in the existing `.bld-pool-head` row (next to the result count / legend), which is always visible above the grid.

### Typography/sizing bumps live in CSS, scoped by view class
The base (GRID) rules raise the preview card size and effect text above 11px; the `view-inspect` rules widen the preview column further and scale the card + effect text up again, while shrinking the pool tile `minmax`. No inline styles; all sizing is in `DeckBuilderPage.css` so the two views stay declaratively described in one place.

### Reset: reuse `DEFAULT_FILTERS`
A RESET button calls `setBuilderFilters(DEFAULT_FILTERS)` — the constant already defined in `DeckBuilderPage.tsx`. It sits in the `.bld-filters` bar (right-aligned). No new state.

## Risks / Trade-offs

- **Three+ colours selected → empty pool could read as a bug.** → The live result count already shows "0 RESULTS" and the new RESET button gives a one-click escape; the spec encodes this as expected behaviour.
- **DETAIL view squeezes the pool grid on small windows.** → Below 1120px the preview is already hidden by the existing media query, so the toggle is a no-op there; DETAIL's re-proportioning only applies at widths where the preview is shown. The view class is harmless when the preview is hidden.
- **`localStorage` unavailable (private mode / quota).** → The loader/writer wrap access in try/catch and fall back to the default, mirroring the existing `uiStore` helpers.
- **CSS regressions in the dense GRID view.** → GRID stays the default and keeps proportions close to today's; changes are additive (a view class) so the untoggled experience is the current one plus the readability bumps.
