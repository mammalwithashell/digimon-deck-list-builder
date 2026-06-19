## 1. Library filter logic + helpers (`utils/deckLibrary.ts`)

- [x] 1.1 Add optional `format?: string` to `LibraryFilters` (default `'all'`) and filter `deck.game_mode === format` in `filterAndSortDecks` when not `'all'`, composing with folder + search
- [x] 1.2 Append `deck.game_mode` to the search haystack in `filterAndSortDecks`
- [x] 1.3 Add `countByFormat(decks)` → `Map<game_mode, number>` (whole-library counts)
- [x] 1.4 Add `deriveFormatBuckets(decks, formats)` → ordered list of `{ id, label, count }` (registry order first, non-registry ids appended sorted; only ids with ≥1 deck)
- [x] 1.5 Add `formatLabel(gameMode, formats)` resolving id → registry display name with raw-id fallback
- [x] 1.6 Unit-test 1.1–1.5 in `utils/deckLibrary.test.ts` (format filter narrows; composes with folder/search; search matches `game_mode`; `'all'`/undefined is a no-op; counts; bucket ordering + unknown id; label fallback)

## 2. Deck Library UI (`pages/DeckLibraryPage.tsx` + `.css`)

- [x] 2.1 Load the format registry once on mount via `deckApi.listFormats()` into state (graceful empty fallback)
- [x] 2.2 Add `activeFormat` state (default `'all'`); thread `format: activeFormat` into the `filterAndSortDecks` call
- [x] 2.3 Add a "Formats" sidebar section (reusing `.library-folder` styling) listing `deriveFormatBuckets` entries with counts plus an "All formats" entry; wire clicks to `setActiveFormat`
- [x] 2.4 Add a format `<select>` to the toolbar bound to the same `activeFormat` state (sidebar + toolbar stay in sync)
- [x] 2.5 Fall back to `'all'` when the active format has zero decks (e.g. after deletion)
- [x] 2.6 Render a format pill (via `formatLabel`) on `DeckTile` and the detail banner `library-pills`; add `.library-format-pill` CSS

## 3. Re-point the play "CHOOSE FORMAT" window (`features/play/formatCatalog.ts`)

- [x] 3.1 Drop the `CONCEPT_ONLY` append from `loadPlayFormats()` so it returns registry-derived formats only
- [x] 3.2 Remove `titan`/`edh_commander`/`draft`/`tutorial` from `PLAY_FORMATS`, `PRESENTATION`, and `CONCEPT_ONLY`; narrow `PlayFormatId` to `standard | no_restriction | pauper | eden | eden_singleton`
- [x] 3.3 Verify `getPlayFormat(formatId: string)` still falls back to `STANDARD_FORMAT` for unknown/legacy ids (keeps DeckSelectPage/MatchingPage/RoomChooserPage/RoomLobbyPage callers safe)
- [x] 3.4 Update `features/play/formatCatalog.test.ts`: remove concept-format assertions; add a `loadPlayFormats` test that mocks `deckApi.listFormats` and asserts registry-only output

## 4. ModeSelectPage uses the registry catalog (`pages/ModeSelectPage.tsx`)

- [x] 4.1 Switch the formats source from `playApi.listFormats()` to `formatCatalog.loadPlayFormats()`
- [x] 4.2 Make the count copy dynamic: replace hardcoded "/ 06" and "SIX RULESETS" with values derived from the rendered list length
- [x] 4.3 Confirm the disabled/"// LOCKED" card branch remains as a safety net but no concept cards feed it
- [~] 4.4 Manually verify (browser-dev `npm run dev:desktop` + uvicorn) the window shows Standard, No Banlist, Pauper, EDEN, EDEN Singleton — all enabled, matching the deck builder. (Deferred live visual; covered by proxy: the `loadPlayFormats` unit test asserts registry-only output, the `/decks/formats` backend smoke confirms the data source, and the full production build passes.)

## 5. Remove the drifted mock (single source of truth)

- [x] 5.1 Grep the repo for remaining consumers of `playApi.listFormats`, `/formats`, and `formats_list`; re-point any unexpected consumer at the registry before deleting
- [x] 5.2 Remove `listFormats` (+ `FormatDto`/`fromDto`) from `features/play/playApi.ts`
- [x] 5.3 Remove the hosted `/formats` route: delete `code/server/routers/formats.py` and its registration in `code/server/api.py` (also deleted `code/tests/api/test_formats.py` which targeted the removed route)
- [x] 5.4 Remove the desktop `formats_list` command: delete `code/src-tauri/src/format_commands.rs` and its registration in `code/src-tauri/src/main.rs` (+ `lib.rs` mod decl + `main.rs` use import)

## 6. Verification

- [x] 6.1 `cd code/frontend && npm run test` (vitest) — `deckLibrary` + `formatCatalog` suites green (18/18; ran the two suites directly)
- [x] 6.2 `cd code/frontend && npx tsc --noEmit` (or the project's typecheck) passes with the narrowed `PlayFormatId` (`tsc -b` exit 0)
- [x] 6.3 `cargo check --manifest-path code/src-tauri/Cargo.toml` succeeds after removing `format_commands.rs` (exit 0; compiled past the `invoke_handler!`/`generate_context!` macros, only pre-existing engine `unused import` warnings). Used `cargo check` rather than the full `cargo tauri build` — proportionate to a pure removal of an unused command; bundling installers is a packaging step, not a code-correctness check.
- [x] 6.4 Backend import/route smoke check passes after removing the `/formats` router (app assembles, `/decks/formats` still served)
