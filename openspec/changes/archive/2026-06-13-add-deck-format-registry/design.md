## Context

Deck legality for non-standard formats already exists in the Rust engine but is unreachable and structurally duplicated. Three notions of "format" disagree:

| Source | Formats it knows | What it encodes |
|---|---|---|
| `enums.rs` `GameMode` | Standard, Pauper, NoRestriction, Eden, EdhCommander, Titan | just the typed key |
| `rules.rs` `Rules` presets | standard, pauper, no_restriction, eden, edh, titan_* | deck_size, egg_max, `singleton`, `allowed_card_rarity_mask`, `restriction` ← the real data |
| `deck_tools.rs` `DeckRuleset` | Standard, NoRestriction, Pauper, Eden (no EDH/Titan/singleton) | **separate enum; re-implements rarity logic inline; ignores `Rules`** |
| frontend `formatCatalog.ts` | standard, titan, edh, nobanlist, draft, tutorial | hardcoded; names don't match backend |

Consequences: the rarity gate is duplicated (`Rules::pauper()` has the mask, but the validator hardcodes `is_common_or_uncommon()`); singleton is enforced **only in Python** (`decks.py:_validate_for_mode`) for EDH and nowhere in Rust; the EDEN banlist is a Rust `LazyLock` *and* a parallel Python list; the EDEN anomaly protocol is a hardcoded name-substring heuristic (`is_eden_anomaly`) that can't be extended without editing Rust.

Constraints carried in from exploration: config must be **editable as data, not code** (compile-time baked is acceptable — matches `cards.json`/`tested_cards.json`); EDEN Singleton = EDEN anomaly policy + EDEN banlist + singleton; all five formats playable; the codebase has a documented history of Rust↔Python and desktop↔browser duplication drifting, so a single source of truth is the explicit goal.

## Goals / Non-Goals

**Goals:**
- One editable data file (`data/deck_formats.json`) as the source of truth for format descriptors, named restrictions, and the anomaly protocol.
- A `FormatDescriptor` registry in the engine that drives validation, `Rules` derivation, and metadata — no per-format `if` branches in the validator.
- A `card_legality(card_id, format)` primitive so the frontend filters/badges cards using engine logic, never re-implemented in TS.
- New `eden_singleton` format; EDEN banlist and anomaly list trivially editable/expandable via data.
- Five formats selectable in the deck builder and queueable in matchmaking.
- Remove the Rust↔Python banlist duplication and move singleton validation into Rust.

**Non-Goals:**
- Runtime-fetched / hot-reloadable format config for installed desktop apps (decided: compile-time baked; a future change can add a fetch path like `models.rs`).
- Changing in-game `Rules` used at runtime — game creation stays `Rules::standard()` for these 50-card formats (legality is a build-time gate; gameplay params are identical). EDH/Titan runtime wiring is untouched.
- Draft / Tutorial play formats (remain disabled concepts in the catalog).
- Reworking the alpha "tested-cards" gate — it stays an orthogonal layer composed on top of format validation.

## Decisions

### D1: Format config lives in `data/deck_formats.json`, baked via `include_str!`

A single JSON file with three top-level sections: `restrictions` (named banlists), `anomaly_protocol` (rarity-exception rules), and `formats` (descriptors referencing a restriction + anomaly policy by name).

```jsonc
{
  "restrictions": {
    "official_eng": { "banned": ["BT2-090", ...], "limited": ["BT1-090", ...],
                      "limited_to": {}, "choice_groups": [[["EX2-007"],["EX7-064"]], ...] },
    "eden":         { "banned": ["BT3-097", ...], "limited": ["BT1-107", ...],
                      "limited_to": { "BT6-085": 4 }, "choice_groups": [[["EX4-015"],["EX5-065"]]] }
  },
  "anomaly_protocol": {
    "max_total": 4,
    "categories": [
      { "card_kind": "tamer",  "rarities": ["R","P"] },
      { "card_kind": "option", "name_contains": "memory boost", "rarities": ["R","SR","P"] },
      { "card_kind": "option", "name_contains": "training",     "rarities": ["P"] },
      { "card_kind": "option", "name_contains": "scramble",     "rarities": ["P"] }
    ],
    "extra_card_ids": []
  },
  "formats": [
    { "id": "standard",       "name": "Standard",        "rarity_policy": "all",
      "banlist": "official_eng", "singleton": false, "default_max_copies": 4, "playable": true },
    { "id": "no_restriction", "name": "No Banlist",      "rarity_policy": "all",
      "banlist": null,           "singleton": false, "default_max_copies": 4, "playable": true },
    { "id": "pauper",         "name": "Pauper",          "rarity_policy": "common_uncommon",
      "banlist": "official_eng", "singleton": false, "default_max_copies": 4, "playable": true },
    { "id": "eden",           "name": "EDEN",            "rarity_policy": "eden_anomaly",
      "banlist": "eden",         "singleton": false, "default_max_copies": 4, "playable": true },
    { "id": "eden_singleton", "name": "EDEN Singleton",  "rarity_policy": "eden_anomaly",
      "banlist": "eden",         "singleton": true,  "default_max_copies": 1, "playable": true }
  ]
}
```

The anomaly protocol carries `max_total` and the named-policy hook so a format references it by `rarity_policy: "eden_anomaly"`. Only EDEN-family formats reference it today, but it is a named policy so future anomaly variants are possible.

*Rationale:* mirrors the existing `include_str!` pattern, so desktop gets it baked with no resource-bundling step and the hosted API reads the same file at runtime — one set of bytes, no Rust↔Python drift. *Alternative considered:* runtime-fetched config (rejected this round — more infra; banlist edits ride the normal release train, which is acceptable).

### D2: `FormatDescriptor` registry is the single source of truth; `Rules` derives from it

`format.rs` parses the data file once (`OnceLock`) into a `HashMap<String, FormatDescriptor>` plus an ordered `Vec` for `list_formats()`. `FormatDescriptor` holds id, display name/description, `deck_size`, `egg_max`, `RarityPolicy`, a resolved `&CardRestriction`, `singleton`, `default_max_copies`, `playable`.

`Rules::for_mode(mode)` / `Rules::<preset>()` are re-expressed as "look up the descriptor, build `Rules` from it." `CardRestriction::eden()`/`official_eng()` become thin accessors over the parsed registry rather than hand-written `LazyLock`s. `GameMode` stays the typed key, mapped to a descriptor id 1:1 (`GameMode::EdenSingleton` ↔ `"eden_singleton"`).

*Rationale:* collapses the three-way redundancy to one table; adding a format = one JSON row. *Alternative considered:* keep `DeckRuleset` and bolt new branches on (rejected — perpetuates the duplication this change exists to remove).

### D3: Validation is fully descriptor-derived — generic checks, no per-format branches

`validate_deck` takes a descriptor and applies, in order: deck size (`deck_size`/`egg_max`), per-card DB `max_count_in_deck`, **effective limit** `min(restriction_limit_or_default, singleton ? 1 : default_max_copies)`, rarity policy, anomaly cap, and choice groups. `RarityPolicy`:
- `All` → no rarity gate.
- `CommonUncommon` → reject rare-or-higher non-egg cards (reads the mask concept; Digi-Eggs always exempt).
- `EdenAnomaly` → C/U legal; rare+ legal only if it matches an anomaly category or `extra_card_ids`, and the total anomaly count ≤ `max_total`.

The existing `DeckRuleset` enum and its `format_card_limit`/`is_common_or_uncommon`/`is_eden_anomaly`/inline EDEN block are deleted; `validate_deck_for_game_mode(game_mode: &str)` resolves the descriptor by id and calls the generic path.

*Rationale:* the per-format `if ruleset == Eden`/`== Pauper` blocks are exactly what makes new formats expensive. Generic singleton also retroactively gives EDH a correct Rust validation path (a `RUST_PYTHON_PARITY.md` win), letting Python defer.

### D4: `card_legality(card_id, format)` is the new query primitive

Returns `{ legal: bool, max_copies: u32, reason: Option<String> }` for a single card under a format — the per-card projection of the same logic `validate_deck` uses (rarity policy + restriction + singleton; the ≤`max_total` anomaly *cap* is deck-level so `card_legality` reports "counts toward anomaly limit" rather than a hard reject). Exposed as `rust_card_legality` (Tauri), a PyO3 function, and `GET/POST /decks/card-legality` (hosted). The deck builder calls it (or a batch variant over the pool) to drive the legality filter and per-card badges.

*Rationale:* the only way to filter/search a pool by legality without re-implementing the rules in TypeScript. *Alternative considered:* port the predicate to TS (rejected — duplication drift, the precise failure mode this change targets).

### D5: Hosted API defers to the engine; frontend catalog is engine-sourced

`deck_tools.py:_validate_for_mode` and `decks.py` route every rarity/banlist/anomaly/singleton mode through `validate_deck_for_game_mode`; only genuinely engine-unsupported modes (EDH/Titan, which need player-count/size rules the binding doesn't yet expose) keep their Python path. `formatCatalog.ts` is populated from `list_formats()` (with purely-presentational fields like `populationPct` staying client-side), so backend and frontend can no longer disagree on names.

## Risks / Trade-offs

- **`cards.json` rarity accuracy** (CLAUDE.md flags it as imperfect API ingest) → four formats now gate on `rarity`, and the anomaly heuristic also keys on name substrings. A wrong value mis-gates a card everywhere. *Mitigation:* in-scope QA sweep of rarities for anomaly-protocol and C/U-boundary cards; `card_overrides.json` patches where wrong; `extra_card_ids` can force-include a miscategorized anomaly card.
- **Behavior drift during the `DeckRuleset` → descriptor rewrite** → existing Standard/EDEN/Pauper validation must stay byte-identical (error strings included; the frontend shows the first error). *Mitigation:* port the existing `rules.rs`/`deck_tools.rs` tests to the new path and assert the parsed-from-JSON `official_eng`/`eden` restrictions equal today's hardcoded values before deleting the `LazyLock`s.
- **Compile-time config** → a banlist edit doesn't reach installed desktop apps until the next release. *Accepted* per D1; revisitable via a future runtime-fetch change.
- **JSON parse failure is fatal** (baked resource, like `cards.json`) → a malformed `deck_formats.json` panics the engine at first use. *Mitigation:* a `dsl-lint`-style unit test parses and structurally validates the file in CI (every `format.banlist` resolves to a defined restriction; every `rarity_policy` is known).
- **EDEN Singleton anomaly cap interaction with singleton** → `≤4 total anomaly cards` still applies independently of the max-1 copy rule (4 *distinct* anomaly singles is legal; a 5th is not). *Mitigation:* explicit behavioral test for this combination.

## Migration Plan

1. Land `data/deck_formats.json` + `format.rs` parsing with an equivalence test against the current hardcoded restrictions (no behavior change yet).
2. Rewrite `validate_deck`/`Rules` to consume the registry; delete `DeckRuleset` and the `LazyLock`s; run the full ported test suite.
3. Add `GameMode::EdenSingleton` + `card_legality`; expose via PyO3 + Tauri + hosted API.
4. Alembic migration: add `pauper` + `eden_singleton` to `decks` / `game_sessions` check constraints (downgrade rewrites those rows to `standard`, mirroring `20260427_0017`).
5. Frontend: format selector + store threading + legality filter/badges + `formatCatalog` from `list_formats` + matchmaking exposure + the `saveBuilderDeck` `game_mode` fix.
6. Docs + rarity QA sweep.

*Rollback:* the Alembic `downgrade` reverts the constraints and normalizes rows; reverting the code restores the hardcoded path. No persisted format data is lost except decks saved under the two newly-added modes (which downgrade rewrites to `standard`).

## Open Questions

- Should `card_legality` ship a **batch** form (`card_legality_bulk(format)` returning a map for the whole tested pool) to avoid N calls when the builder filters the pool? Leaning yes — the pool is ~hundreds of cards and the data is already resident; resolve during implementation based on the filter UX.
- Display copy (tagline/description) for the four newly-surfaced formats — pull from `deck_formats.json` `description`, or keep marketing copy in the frontend? Default: engine provides a plain `description`; frontend may override presentation.
