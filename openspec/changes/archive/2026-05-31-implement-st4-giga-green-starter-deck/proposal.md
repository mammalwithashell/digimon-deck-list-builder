## Why

ST-4: Starter Deck Giga Green is a compact, historically important green starter deck that is fully present in card metadata but not yet authored as Rust DSL card specs. Implementing it gives the Rust engine a complete beginner-facing green deck while exercising reusable green mechanics such as suspend, Piercing, Digi-Burst, reveal search, security options, and battle-deletion inherited triggers.

## What Changes

- Add faithful Rust DSL YAML implementations for every ST-4 card (`ST4-01` through `ST4-16`), including no-op/vanilla registrations where needed for implemented-card discovery.
- Add focused Rust behavioral tests for every non-vanilla ST-4 effect, with positive and negative coverage for optionality, target filters, and once-per-turn gates.
- Add an ST-4 Giga Green starter deck recipe/fixture if no canonical starter-deck entry already exists in the deck library.
- Add a reusable DSL predicate or trigger-context helper for "this Digimon deleted its battle opponent in battle and survived" so `ST4-11 MegaKabuterimon` can be expressed without over-firing.
- Reconcile ST-4 readiness/test trackers after tests pass; do not mark cards implemented based on metadata-only JSON or approximate behavior.

## Capabilities

### New Capabilities
- `st4-giga-green-starter-deck`: Complete Rust-engine coverage for the ST-4 Giga Green starter deck, including card DSL, behavioral tests, and a playable starter-deck recipe.

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: Add a reusable battle-deletion-survivor predicate/helper for inherited effects like ST4-11 that care whether the source carrier deleted its battle opponent and survived.

## Impact

- **Card content:** `code/digimon-engine/cards/st4/*.yaml` for ST4-01 through ST4-16.
- **Tests:** `code/digimon-engine/tests/cards_behavioral/st4/` plus targeted DSL/predicate tests for the ST4-11 reusable helper.
- **DSL crate:** `code/digimon-dsl/src/` if a new predicate field or syntax is required for the ST4-11 trigger.
- **Rust engine:** `code/digimon-engine/src/dsl_cards/` and trigger-context/predicate evaluation code if the existing battle-opponent context is not currently exposed to DSL predicates.
- **Deck data:** `data/deck_library.json` or the existing starter-deck fixture location, depending on current deck-recipe conventions.
- **Trackers/docs:** `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, `qa/resolved-gaps.md`, and any tested-card/validated-card ledger used by current Rust DSL card coverage.
- **No breaking changes.** No action-space or tensor-contract change is expected; any new gameplay choice must use existing pending-selection surfaces.
