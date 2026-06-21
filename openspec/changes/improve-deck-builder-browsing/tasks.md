## 1. Colour filter intersection

- [x] 1.1 In `features/deck-builder/deckBuilderView.ts`, change `filterBuilderCards` colour matching from OR to a superset test: build the card's colour identity set from `color` (+ `color2` when present) and exclude the card unless it contains every selected colour.
- [x] 1.2 In `features/deck-builder/deckBuilderView.test.ts`, add cases: two colours include only the dual-colour card (mono cards excluded); one colour includes mono + dual cards containing it; no colour selected excludes nothing; three colours selected yields zero matches.

## 2. Persisted view-mode preference

- [x] 2.1 In `stores/uiStore.ts`, add a `deckBuilderView: 'browse' | 'inspect'` field with a storage key, a validating `loadPersistedDeckBuilderView` (falls back to `'browse'` on missing/invalid), a `persistDeckBuilderView` writer, and `setDeckBuilderView` + `toggleDeckBuilderView`; export the key/loader via `__uiStoreInternals`.
- [x] 2.2 Add a `uiStore` test covering: default is `'browse'`, a valid stored value hydrates, and a stale/invalid value falls back to `'browse'`.

## 3. Deck builder view toggle + reset wiring

- [x] 3.1 In `pages/DeckBuilderPage.tsx`, read `deckBuilderView`/`toggleDeckBuilderView` from `uiStore` and apply a `view-browse` / `view-inspect` class to `.bld-main`.
- [x] 3.2 Add a GRID/DETAIL segmented toggle to the `.bld-pool-head` row, bound to the store value.
- [x] 3.3 Add a RESET button to the `.bld-filters` bar that calls `setBuilderFilters(DEFAULT_FILTERS)`.

## 4. Layout + typography styling

- [x] 4.1 In `pages/DeckBuilderPage.css`, raise the base (GRID) selected-card preview image size and effect text above 11px while preserving the card-text token styles (`.tm` / `.rf` / `.kw`).
- [x] 4.2 Add `view-browse` / `view-inspect` rules on `.bld-main` that set per-view `grid-template-columns`, pool-tile `minmax`, and preview/effect sizing — DETAIL widens the preview column and enlarges the card + effect text while shrinking the pool tiles.
- [x] 4.3 Style the GRID/DETAIL toggle and the RESET button consistent with the existing builder chrome (including the `[data-theme="light"]` Win95 variants), and verify the `<= 1120px` media query still hides the preview so the toggle is a no-op there.

## 5. Verification

- [x] 5.1 Run the deck-builder frontend tests (`deckBuilderView.test.ts`, `uiStore` test) and the type check; confirm green.
- [x] 5.2 Manually verify in the running builder: colour AND filtering, GRID↔DETAIL toggle with persistence across reload, larger/readable preview + effect text in both views, and one-click RESET clearing every filter.

## 6. Third view mode: DECKLIST

- [x] 6.1 In `stores/uiStore.ts`, extend `DeckBuilderView` to `'browse' | 'inspect' | 'decklist'`, add `'decklist'` to `DECK_BUILDER_VIEWS`, and make `toggleDeckBuilderView` cycle through all three views (next-in-list, wrapping). Default + invalid fallback stay `'browse'`.
- [x] 6.2 Update the `uiStore` test for the new enum value and three-way toggle cycling.
- [x] 6.3 In `pages/DeckBuilderPage.tsx`, add a third **DECK** button to the GRID/DETAIL segmented toggle, bound to `setDeckBuilderView('decklist')`.
- [x] 6.4 In `pages/DeckBuilderPage.css`, add `.bld-main.view-decklist` rules (min-width-guarded): hide the preview, collapse the pool to a ~340px add strip, give the deck panel the remaining width, and render `.bld-deck-list` in two columns (CSS multi-column, sections kept intact). Include the `[data-theme="light"]` parity if needed.
- [x] 6.5 Verify: uiStore test green, `tsc -b` clean, and live in the running builder confirm DECKLIST emphasises a two-column deck list with the pool as an add strip, and the choice persists.
