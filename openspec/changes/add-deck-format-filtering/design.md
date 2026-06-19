## Context

A deck's format is its `game_mode`, sourced from the engine format registry (`deck-format-registry` capability, backed by `data/deck_formats.json`). Two frontend surfaces consume formats today, from two different sources:

- **Deck builder** (`DeckBuilderPage.tsx`) reads `deckApi.listFormats()` → `rust_list_formats` (desktop) / `GET /decks/formats` (hosted). This is the live registry and lists the real playable formats (`standard`, `no_restriction`, `pauper`, `eden`, `eden_singleton`).
- **Play "CHOOSE FORMAT" window** (`ModeSelectPage.tsx`) reads `playApi.listFormats()` → `formats_list` Tauri command (desktop) / `GET /formats` (hosted). Both of those return a **hardcoded mock** (`server/routers/formats.py`, `src-tauri/src/format_commands.rs`) that has drifted: only `standard` is enabled, it lists locked concept placeholders (`titan`/`edh`/`nobanlist`/`draft`/`tutorial`), and it omits the actually-playable registry formats.

The Deck Library (`DeckLibraryPage.tsx`) filters only by folder + search and never displays a deck's format. The `formatCatalog.loadPlayFormats()` helper already derives a play catalog from the registry (`deckApi.listFormats()` + a presentation overlay) — `ModeSelectPage` simply doesn't use it.

This change has two parts: (A) add format filtering/display to the Deck Library, and (B) re-point the play window at the registry and delete the drifted mock.

## Goals / Non-Goals

**Goals:**
- Filter the Deck Library by `game_mode`, surfaced in a synced sidebar section (with counts) and toolbar dropdown.
- Show each deck's format as a pill, and include `game_mode` in library search.
- Make the play "CHOOSE FORMAT" window source from the engine registry, showing exactly the playable formats — matching the deck builder.
- Remove the concept placeholders and the drifted mock endpoints so the registry is the single source of truth.

**Non-Goals:**
- No engine/registry changes (`data/deck_formats.json`, validation, legality) — the registry already returns the right formats.
- No change to deck-builder format selection, validation, or card-pool filtering behavior.
- No new format support (Titan/EDH/Draft/Tutorial stay out of scope; they are simply not shown).
- The desktop launcher home screen is not a format picker and is untouched.

## Decisions

### A1. Library filter state — one shared `activeFormat`

A single `activeFormat` state (default `'all'`) in `DeckLibraryPage`, read/written by both the sidebar entries and the toolbar `<select>`. This keeps the two surfaces in sync for free without a derived store. `LibraryFilters` in `utils/deckLibrary.ts` gains an optional `format?: string` (defaulting to `'all'` keeps existing callers working), and `filterAndSortDecks` filters `deck.game_mode === format` when not `'all'`, composing with folder + search. `game_mode` is appended to the search haystack.

*Alternative considered:* a Zustand slice for filter state — rejected as overkill for page-local UI state that already lives in `useState`.

### A2. Format list + counts derived from decks, labeled from the registry

The sidebar/dropdown list the formats **present among the user's decks** (distinct `game_mode` values), in registry order, with any non-registry id appended (sorted). Counts are computed over the whole library (folder-independent), matching how the existing folder counts behave. Labels resolve `game_mode` → registry display name via `deckApi.listFormats()` loaded once on mount, with a raw-id fallback. Helpers (`countByFormat`, `deriveFormatBuckets`, `formatLabel`) live in `utils/deckLibrary.ts` so they are unit-testable in isolation.

*Alternative considered:* listing every registry format even with zero decks — rejected to avoid cluttering the sidebar with empty buckets; "All formats" always provides the reset.

### A3. Format pill display

A pill on each `DeckTile` (meta row) and the detail banner `library-pills`, using `formatLabel`. New `.library-format-pill` CSS; sidebar format rows reuse the existing `.library-folder` styling.

### B1. Re-point the play window via the existing catalog helper

`ModeSelectPage` switches from `playApi.listFormats()` to `formatCatalog.loadPlayFormats()`, which already derives from `deckApi.listFormats()` (registry) on both runtimes and overlays presentational flavour. `loadPlayFormats()` is changed to return registry formats only (drop the `CONCEPT_ONLY` append). This fixes both desktop and hosted in one frontend edit because `deckApi.listFormats()` already routes correctly per runtime.

*Alternative considered:* rewriting the backend `/formats` route and the `formats_list` Tauri command to proxy the registry (keeping the existing data-flow). Rejected as the primary path because it duplicates the presentation overlay in Python + Rust and leaves two parallel format sources; re-pointing the frontend collapses to one source. The mock is removed instead (B3).

### B2. Trim concept formats from `formatCatalog`

Remove `titan`/`edh_commander`/`draft`/`tutorial` from `PLAY_FORMATS`, `PRESENTATION`, and `CONCEPT_ONLY`; narrow `PlayFormatId` to the registry ids (`standard | no_restriction | pauper | eden | eden_singleton`). `getPlayFormat(formatId: string)` stays tolerant and falls back to `STANDARD_FORMAT` for any legacy/unknown id, so the widely-used `getPlayFormat`/`canUseDeckForFormat` callers (DeckSelectPage, MatchingPage, RoomChooserPage, RoomLobbyPage) keep working for decks whose `game_mode` is outside the narrowed set. `ModeSelectPage` copy ("SIX RULESETS", "/ 06") is made dynamic from the rendered list length; the disabled/"// LOCKED" card branch is retained as a safety net (in case the registry ever reports a non-playable format) but no concept cards feed it.

### B3. Delete the drifted mock (single source of truth)

`playApi.listFormats` is consumed only by `ModeSelectPage`; after B1 it is dead and is removed (with its `FormatDto`/`fromDto`). The hosted `/formats` route (`server/routers/formats.py` + its registration in `server/api.py`) and the desktop `formats_list` command (`src-tauri/src/format_commands.rs` + its registration in `src-tauri/src/main.rs`) are removed. This is gated on a repo-wide grep confirming no other consumers (`/formats`, `formats_list`); if an unexpected consumer exists, that consumer is re-pointed at the registry first.

## Risks / Trade-offs

- **A stale-flagged or non-playable registry format** → `ModeSelectPage` keeps the disabled-card rendering branch, so a `playable: false` registry format degrades to a locked card rather than vanishing or crashing.
- **Legacy decks with `game_mode` of `titan`/`edh_commander`** (allowed by DB constraints) → narrowing `PlayFormatId` only affects compile-time literals in our own code/tests; `getPlayFormat`/`canUseDeckForFormat` take `string`/fall back to standard, so such decks still render in the library (raw-id pill) and in play surfaces.
- **Removing a Tauri command requires a desktop rebuild** → handled by the normal desktop build; no runtime migration. Hosted route removal is a standard deploy. Rollback = revert the change.
- **Tests assert concept formats** (`formatCatalog.test.ts` checks `titan`/`edh_commander` disabled) → those assertions are updated as part of B2; a new `loadPlayFormats` test mocks `deckApi.listFormats` and asserts registry-only output.
- **Format-count semantics** (whole-library vs current-folder) → chosen whole-library for consistency with existing folder counts; documented so it isn't mistaken for a bug.

## Migration Plan

Frontend-first, no data migration. Land A and B together: ship the frontend re-point + library filter, remove the dead `playApi.listFormats`, then delete the hosted route and Tauri command. Deploy the hosted API and cut a desktop build. Rollback is a straight revert (the registry endpoints `/decks/formats` and `rust_list_formats` are unchanged throughout).

## Open Questions

- None blocking. (If a future build wants to advertise upcoming formats, that would be a separate "roadmap teaser" feature rather than the play-format catalog.)
