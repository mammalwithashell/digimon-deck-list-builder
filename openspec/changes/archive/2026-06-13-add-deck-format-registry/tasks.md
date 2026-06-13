## 1. Format config data file (source of truth)

- [x] 1.1 Author `data/deck_formats.json` with `restrictions` (`official_eng`, `eden` — transcribed from the current `OFFICIAL_ENG_RESTRICTION` and `EDEN_RESTRICTION`, including `limited_to` for Eosmon=4 and the choice groups), `anomaly_protocol` (`max_total: 4`, category rules for Tamer/Memory Boost/Training/Scramble, empty `extra_card_ids`), and `formats` (standard, no_restriction, pauper, eden, eden_singleton).
- [x] 1.2 Add a parse + structural-validation unit test: every `format.banlist` resolves to a defined restriction, every `rarity_policy` is known, every `GameMode` variant has a matching descriptor id.

## 2. Engine: format registry (`format.rs`)

- [x] 2.1 Add `format.rs`: `RarityPolicy { All, CommonUncommon, EdenAnomaly }`, `AnomalyProtocol`, `FormatDescriptor`, and a `OnceLock` registry parsed from the baked `deck_formats.json` (`include_str!`), plus `list_formats()` and `descriptor(id)`.
- [x] 2.2 Add equivalence test asserting the parsed `official_eng` and `eden` restrictions equal today's hardcoded `CardRestriction` values (run BEFORE deleting the `LazyLock`s).
- [x] 2.3 Re-express `CardRestriction::official_eng()/eden()` as accessors over the registry; delete `OFFICIAL_ENG_RESTRICTION` / `EDEN_RESTRICTION` `LazyLock`s.

## 3. Engine: enums + Rules

- [x] 3.1 Add `GameMode::EdenSingleton` in `enums.rs` (and its serde + any match arms).
- [x] 3.2 Re-express `Rules` presets and `Rules::for_mode` to build from the descriptor registry; add the `eden_singleton` mapping; keep EDH/Titan presets as-is.
- [x] 3.3 Port/adjust the `rules.rs` tests to the registry-derived path.

## 4. Engine: descriptor-derived validation

- [x] 4.1 Rewrite `validate_deck_for_ruleset` in `deck_tools.rs` to take a `FormatDescriptor` and apply generic checks: size, DB copy limits, effective limit `min(restriction_or_default, singleton ? 1 : default_max_copies)`, rarity policy, anomaly cap, choice groups — no per-format branches.
- [x] 4.2 Implement `RarityPolicy` evaluation (`All` / `CommonUncommon` / `EdenAnomaly`) and anomaly matching (category rules + `extra_card_ids` + `max_total`) reading the registry.
- [x] 4.3 Repoint `validate_deck_for_game_mode(&str)` / `validate_deck_for_mode(GameMode)` to resolve a descriptor and call the generic path; delete the `DeckRuleset` enum, `format_card_limit`, `is_common_or_uncommon`, `is_eden_anomaly`, and the inline EDEN block.
- [x] 4.4 Port existing deck-validation tests; add tests for Standard parity (error strings), Pauper rarity, EDEN anomaly within/over cap, rare-outside-anomaly, EDEN Singleton (one-copy + anomaly cap + EDEN ban).

## 5. Engine: `card_legality` primitive

- [x] 5.1 Implement `card_legality(card_id, format)` → `{ legal, max_copies, reason }` deriving from the same descriptor logic (rarity policy + restriction + singleton; report anomaly-counted constraint rather than hard-rejecting on the deck-level cap).
- [x] 5.2 (Decision from design Open Questions) implement a batch `card_legality` over the tested pool if the filter UX needs it.
- [x] 5.3 Unit tests: legal common, banned card (0 copies), EDEN Singleton (1 copy), anomaly-counted card.

## 6. Bindings + hosted API

- [x] 6.1 PyO3 `digimon-engine-py/src/lib.rs`: export `list_formats` and `card_legality`; ensure `eden_singleton` validates via `validate_deck_for_game_mode`.
- [x] 6.2 Tauri `src-tauri/src/deck_commands.rs`: add `rust_list_formats` and `rust_card_legality` commands; `rust_validate_deck_raw` already threads `game_mode` (verify new modes pass through).
- [x] 6.3 Hosted `server/routers/deck_tools.py`: route every rarity/banlist/anomaly/singleton mode through `validate_deck_for_game_mode`; add `/decks/formats` and `/decks/card-legality` endpoints.
- [x] 6.4 Hosted `server/db/routers/decks.py`: simplify `_validate_for_mode` to defer to the engine for the registry-backed modes (keep EDH/Titan Python paths); accept `pauper`/`eden_singleton` in the `game_mode` query patterns.
- [x] 6.5 `server/routers/matchmaking.py`: accept the new `game_mode` values for queueing.

## 7. Database migration

- [x] 7.1 New Alembic migration adding `pauper` + `eden_singleton` to the `decks` and `game_sessions` `game_mode` check constraints; downgrade normalizes those rows to `standard` (mirror `20260427_0017`).

## 8. Frontend

- [x] 8.1 `api/deckApi.ts`: add `listFormats()` and `cardLegality(...)` (Tauri `invoke` + hosted `/decks/...`), and corresponding types in `types/deck.ts`.
- [x] 8.2 `features/play/formatCatalog.ts`: source formats from `listFormats()`; keep purely-presentational fields (e.g. `populationPct`) client-side; reconcile id names with backend (`no_restriction`, `edh_commander`).
- [x] 8.3 `stores/deckBuilderStore.ts`: add `gameMode` state; thread it through load/clear.
- [x] 8.4 `pages/DeckBuilderPage.tsx`: add a format selector; pass `gameMode` to `validateDeckRaw` and `saveBuilderDeck`; load the deck's `game_mode` on open.
- [x] 8.5 `features/deck-builder/deckBuilderView.ts` + builder UI: add a "format-legal only" pool filter and per-card legality badges (banned / limit / anomaly) using `cardLegality`.
- [x] 8.6 Fix `features/deck-builder/deckBuilderAdapter.ts` so the browser update path includes `game_mode` in the payload.
- [x] 8.7 Update/extend frontend tests (`formatCatalog.test.ts`, deck builder view/filter) for engine-sourced formats and the legality filter.

## 9. Docs + data QA

- [x] 9.1 Update `docs/EDEN_FORMAT_RULES.md` (data-file location, EDEN Singleton) and `docs/RUST_PYTHON_PARITY.md` (banlist single-sourced; singleton validation now in Rust).
- [x] 9.2 Rarity-accuracy sweep of `cards.json` for anomaly-protocol and C/U-boundary cards; patch via `card_overrides.json` or `extra_card_ids` where wrong.

## 10. Verification

- [x] 10.1 `cargo test --manifest-path code/digimon-engine/Cargo.toml` (format, rules, deck_tools, card_legality) green.
- [x] 10.2 `cargo test --manifest-path code/src-tauri/Cargo.toml` and `maturin develop` + Python deck-validation/binding tests green.
- [x] 10.3 Frontend build + tests green; manual desktop check: select each of the five formats, filter the pool, validate, and save a deck per format.
