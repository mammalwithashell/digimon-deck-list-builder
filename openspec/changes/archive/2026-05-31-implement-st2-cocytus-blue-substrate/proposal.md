## Why

ST-2 Starter Deck Cocytus Blue is a compact, historically important blue starter deck whose card texts exercise reusable source-trash, no-digivolution-card, bounce, and source-play behavior. The Rust engine currently has ST2 metadata and a faithful ST2-13 Hammer Spark DSL implementation, but the rest of the starter deck is not production-authored in DSL, leaving the full starter list unusable as a verified deck and leaving several blue substrate questions unresolved.

## What Changes

- Add a verified ST-2 Cocytus Blue starter deck artifact with the official 4 Digi-Egg + 50 main-deck composition.
- Author production Rust DSL YAML for `ST2-01` through `ST2-12` and `ST2-14` through `ST2-16`; keep the existing `ST2-13` implementation as the baseline Option pattern.
- Add behavioral tests for every ST2 card, including vanillas, inherited effects, Option security/main effects, Tamer security play, source-trash effects, bounce effects, source-play effects, attack restrictions, and once-per-turn unsuspend.
- Close or introduce reusable DSL substrate as needed for faithful ST2 authoring:
  - a no-choice bottom-source trash step for printed text that says to trash bottom digivolution cards, distinct from player-selected `select_opponent_sources`;
  - battle-context predicate support for inherited DP modifiers that depend on the opposing battled Digimon's source count;
  - source-play authoring coverage for `Kaiser Nail` using existing `select_material` / `play_from_materials` substrate or a narrowly scoped follow-up if the current substrate cannot express ST2-15 faithfully;
  - source-trash soft-fail coverage for any new bottom-source trash primitive.
- Reconcile stale gap tracker entries that still claim `select_opponent_sources` is missing, and record any genuinely new blocker in the reusable gap trackers.

## Capabilities

### New Capabilities

- `st2-cocytus-blue-coverage`: End-state coverage guarantee for ST-2 Cocytus Blue: the official starter deck list is represented, every unique ST2 card has faithful DSL YAML and behavioral tests, and the verified card/deck status is reflected in the relevant trackers.

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: Add any DSL vocabulary required for faithful ST2 blue starter cards, especially no-choice bottom-source trash and battle-context predicates.
- `source-trash-soft-fail`: Extend the source-trash no-panic contract to any new bottom-source trash primitive introduced for ST2.

## Impact

- **Card content:** `code/digimon-engine/cards/st2/*.yaml` for ST2 production DSL cards.
- **Deck artifacts:** starter-deck/deck-library data used by deck tools, training, or tests for ST-2 Cocytus Blue.
- **Tests:** `code/digimon-engine/tests/cards_behavioral/st2/*.rs` plus targeted `dsl` tests for any new vocabulary.
- **DSL crate:** `code/digimon-dsl/src/` for new step/predicate fields and compiled forms if current vocabulary is insufficient.
- **Engine (Rust):** `code/digimon-engine/src/dsl_cards/`, `effect_context/`, combat predicate evaluation, and source-trash movement helpers as needed for faithful substrate.
- **Trackers:** `qa/qa-reports/validated_cards_dsl.json`, `data/tested_cards.json`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/resolved-gaps.md`.
- **No tensor/action contract expansion expected.** ST2 should use existing pending-selection/action surfaces. If implementation proves a new player-visible choice cannot fit the current action space, that work must stop and be planned as a separate action/tensor contract change.
