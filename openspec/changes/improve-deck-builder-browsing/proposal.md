## Why

The deck builder is harder to browse and read than the DCGO builder we model it on: the selected-card preview lives in a cramped 240px column with 11px effect text, the pool tiles are tiny (~82px, 7–8 per row), the color filter matches *any* selected color (so picking Green + Blue floods the pool with mono-colour cards instead of the dual cards the user wants), and there is no way to clear all filters at once.

## What Changes

- **Color filter becomes an intersection (AND).** With two or more colours selected the pool shows only cards that contain **all** of them (e.g. Green + Blue → Green/Blue dual cards), instead of the current "any selected colour" (OR). A single selected colour behaves as today.
- **Two card-pool view modes behind a toggle.** A **GRID** view (today's dense grid, the default) and a **DETAIL** view (DCGO-style, where the selected card + its effect text dominate the display). The choice persists across sessions.
- **The selected-card preview and effect text get larger in both views.** The preview card image grows and the effect text scales up from 11px for readability — more so in DETAIL view, but bumped in GRID view too.
- **A RESET button clears all filters at once** — search, colours, type, level, rarity, and the inherited/security/format-legal checkboxes — back to their defaults.
- Out of scope: the card-pool data source, deck saving/validation, the deck-contents panel behaviour, and the in-game board.

## Capabilities

### New Capabilities
- `deck-builder-card-browsing`: How the deck builder presents and filters its card pool — the colour-filter match semantics (intersection across selected colours), the GRID/DETAIL view-mode toggle and its persisted preference, the enlarged selected-card preview and effect typography, and the reset-all-filters control.

### Modified Capabilities
<!-- None. `deck-builder-format-selection` governs only the format-legal filter and legality badges, which are unchanged; the reset control merely returns that filter to its default alongside the others. -->

## Impact

- **Frontend (deck builder only):**
  - `features/deck-builder/deckBuilderView.ts` — `filterBuilderCards` colour matching switches from OR to AND across the selected colour set.
  - `stores/uiStore.ts` — new persisted `deckBuilderView: 'browse' | 'inspect'` preference (localStorage, mirroring the `railCollapsed` pattern).
  - `pages/DeckBuilderPage.tsx` — view-mode toggle in the pool header, a RESET button in the filter bar, and a `view-browse` / `view-inspect` class on `.bld-main`.
  - `pages/DeckBuilderPage.css` — per-view grid proportions and tile sizing; larger preview card + effect typography.
- **No backend, engine, RL, or hosted-web changes.** Existing `<= 1120px` responsive behaviour (preview hidden) is preserved, so the toggle is a no-op at narrow widths.
- **Tests:** `deckBuilderView.test.ts` colour-intersection cases; a `uiStore` test for the new preference persisting/hydrating and rejecting stale values; a reset-to-defaults check.
