## Why

A deck's format (`game_mode`) is set in the deck builder and persisted, but the Deck Library gives no way to filter by it and never even shows it — so a library mixing Standard, Pauper, and EDEN decks can't be narrowed to one ruleset. Separately, the play "CHOOSE FORMAT" window has drifted off the engine registry onto a stale hardcoded mock: it offers only Standard plus locked concept placeholders (Titan/EDH/Draft/Tutorial) and omits the actually-playable No Banlist, Pauper, EDEN, and EDEN Singleton — directly violating the existing `deck-builder-format-selection` requirement that the play/format catalog come from `list_formats()`.

## What Changes

- **Deck Library format filter (new):** Add a format filter to the Deck Library, surfaced in two synced places — a "Formats" section in the left sidebar (with per-format deck counts) and a dropdown in the toolbar — both driven by one selection state. Add `game_mode` to the library search haystack.
- **Deck format pill (new):** Display each deck's format as a pill on the deck tiles and the detail banner, resolved to its registry display name (raw id fallback for legacy/unknown modes).
- **Fix the CHOOSE FORMAT window:** Source the play format-selection window from the engine registry (the same `list_formats()` the deck builder uses) instead of the stale mock, so it shows exactly the playable formats and stays in lockstep with the builder.
- **Remove the concept-format placeholders** (Titan/EDH/Draft/Tutorial) from the play catalog; the window renders the registry's playable formats only, with its "SIX RULESETS / 06" copy made dynamic.
- **Remove the drifted mock source (BREAKING for those endpoints):** delete the redundant hosted `/formats` route and the desktop `formats_list` Tauri command (and the now-dead frontend `playApi.listFormats`), leaving `/decks/formats` + `rust_list_formats` as the single source of truth — contingent on confirming no other consumers.

## Capabilities

### New Capabilities
- `deck-library-format-filter`: Filtering the Deck Library by a deck's format (sidebar section with counts + synced toolbar dropdown), surfacing each deck's format as a pill, and including format in library search.

### Modified Capabilities
- `deck-builder-format-selection`: The play "CHOOSE FORMAT" window must source its formats from the engine registry (no separate hardcoded list, no non-playable concept placeholders), and the redundant mock format endpoints are removed.

## Impact

- **Frontend:** `pages/DeckLibraryPage.tsx` (+ `.css`), `utils/deckLibrary.ts` (+ test), `pages/ModeSelectPage.tsx`, `features/play/formatCatalog.ts` (+ test), `features/play/playApi.ts`.
- **Hosted API:** remove `code/server/routers/formats.py` and its registration in `code/server/api.py`.
- **Desktop (Tauri):** remove `code/src-tauri/src/format_commands.rs` and its registration in `code/src-tauri/src/main.rs` (requires a desktop rebuild).
- **Single source of truth:** `/decks/formats` (hosted) and `rust_list_formats` (desktop), both derived from `data/deck_formats.json` via the `deck-format-registry` capability. No new dependencies.
