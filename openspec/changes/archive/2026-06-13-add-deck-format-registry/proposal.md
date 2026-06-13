## Why

The Digimon engine already fully implements EDEN and Pauper deck legality (rarity gates, anomaly protocol, custom banlists), but none of it is reachable from the deck builder — the UI hardcodes `game_mode: 'standard'`. Worse, "format" is defined in **three disagreeing places** (`GameMode` enum, `Rules` presets, and a separate `DeckRuleset` enum that re-implements rarity logic and ignores `Rules` entirely), plus a fourth hardcoded list in the frontend whose names don't even match the backend. Adding a new format today means editing Rust logic in multiple places and keeping a parallel Python banlist in sync. We want one editable source of truth so formats — including a new EDEN Singleton format and an expandable EDEN banlist/anomaly list — can be added and tuned without touching engine code.

## What Changes

- **Format config becomes editable data.** A new `data/deck_formats.json` holds every format descriptor, all named restrictions (official ENG + EDEN), and the EDEN anomaly protocol definition (category rules + an `extra_card_ids` expansion hook). Edit JSON, not Rust/Python. Baked into the engine via `include_str!` (compile-time), read at runtime by the hosted API — both consume the same bytes.
- **A single format registry in the engine.** A `FormatDescriptor` registry (parsed from the data file) becomes the source of truth. Deck validation derives all checks from the descriptor — generic rarity gate (reads `allowed_card_rarity_mask`), generic singleton (`effective_limit = min(format_limit, singleton ? 1 : default)`), restriction (bans/limits/choice groups), and anomaly protocol — with **no per-format `if` branches**.
- **New `card_legality(card_id, format)` engine primitive** returning `{ legal, max_copies, reason }`, exposed via PyO3 + Tauri + hosted API. Powers the deck-builder pool filter and per-card badges with zero rules duplicated in TypeScript.
- **New EDEN Singleton format** (`eden_singleton`): EDEN anomaly rarity policy + EDEN banlist + singleton (every card max 1).
- **Five formats selectable & playable**: Standard, No Banlist, Pauper, EDEN, EDEN Singleton — in the deck builder and in matchmaking/Play.
- **Frontend deck builder gains a format selector**, a format-legality pool filter, per-card ban/limit/anomaly badges, and threads `game_mode` through save/validate/load. `formatCatalog.ts` is fed by the engine's `list_formats()` instead of its hardcoded list.
- **Rust↔Python banlist duplication removed**: the hosted API's `_validate_for_mode` collapses into deferring to the engine for rarity/banlist/anomaly/singleton modes.
- **DB migration**: add `pauper` and `eden_singleton` to the `decks` / `game_sessions` `game_mode` check constraints.
- Fix the bug where `saveBuilderDeck`'s browser **update** path drops `game_mode`.

## Capabilities

### New Capabilities
- `deck-format-registry`: The engine's data-driven format system — format descriptors loaded from `deck_formats.json`, the rarity/banlist/anomaly/singleton policy model, deck validation derived from descriptors, the `list_formats` and `card_legality` query primitives, and their PyO3/Tauri/hosted-API surfaces.
- `deck-builder-format-selection`: The player-facing behavior — selecting a format in the deck builder, filtering/searching the card pool by format legality with per-card badges, format-aware validation and persistence, and exposing the formats in matchmaking/Play.

### Modified Capabilities
<!-- No existing spec defines deck validation or game formats; both capabilities above are new. -->

## Impact

- **Engine** (`code/digimon-engine/`): new `format.rs` (descriptor registry, `RarityPolicy`, anomaly model, `card_legality`); `rules.rs` (`Rules::eden_singleton`, `for_mode` arm, descriptor→`Rules`); `deck_tools.rs` (rewrite `validate_deck_for_ruleset` to derive from descriptors; remove the parallel `DeckRuleset` inline logic); `enums.rs` (`GameMode::EdenSingleton`).
- **Data**: new `data/deck_formats.json` (source of truth); existing hardcoded `EDEN_RESTRICTION`/`OFFICIAL_ENG_RESTRICTION` `LazyLock`s and `is_eden_anomaly` heuristic removed/replaced.
- **Bindings/API**: `code/digimon-engine-py/src/lib.rs` and `code/src-tauri/src/deck_commands.rs` expose `list_formats` + `card_legality`; hosted `code/server/routers/deck_tools.py` + `code/server/db/routers/decks.py` route the new modes and defer to the engine; `code/server/routers/matchmaking.py` accepts the new modes.
- **Frontend** (`code/frontend/src/`): `features/play/formatCatalog.ts` (engine-sourced), `pages/DeckBuilderPage.tsx`, `stores/deckBuilderStore.ts`, `features/deck-builder/deckBuilderView.ts` (legality filter), `features/deck-builder/deckBuilderAdapter.ts` (game_mode fix), `api/deckApi.ts` (new endpoints), `types/deck.ts`.
- **DB**: new Alembic migration adding `pauper` + `eden_singleton` to game-mode check constraints.
- **Docs**: `docs/EDEN_FORMAT_RULES.md` (note data-file location + EDEN Singleton), `docs/RUST_PYTHON_PARITY.md` (banlist now single-sourced; singleton validation moved to Rust).
- **Data QA**: depends on `cards.json` `rarity` accuracy across four rarity-sensitive formats — a sweep is in scope.
