## Why

ST-5: Starter Deck Machine Black is a compact, well-scoped starter deck with only 16 unique cards, but none of its cards are currently executable through the Rust DSL registry. Implementing it adds a complete Black starter-deck baseline for Rust headless testing, deck validation, and future pilot/architect smoke checks without relying on legacy Python behavior.

The deck is also a useful substrate check: most cards exercise already-supported primitives, while ToyAgumon and Greymon require a reusable way to express "the opponent did not attack with a Digimon this turn" at end-of-turn timing.

## What Changes

- Add faithful Rust DSL YAML implementations for all 16 ST5 cards, with no omitted clauses, placeholder effects, hidden choices, or auto-selections.
- Add Rust behavioral tests for each non-vanilla ST5 card before or alongside the implementation.
- Add a reusable DSL predicate or equivalent engine-backed condition for "no Digimon controlled by the referenced player attacked this turn" so ST5-04 and ST5-06 can be authored faithfully.
- Add focused coverage for ST5-14 Tai Kamiya's blocker-response effect so the trigger fires only when a controller's Digimon uses `<Blocker>` to redirect an attack.
- Add the exact Machine Black starter decklist to the repository's deck surfaces used for validation/training fixtures, once all ST5 card IDs are implemented.
- Update implementation/test ledgers and gap trackers so ST5 readiness reflects actual Rust behavior.
- No breaking changes. This change must not alter action-space size, tensor layout, or active observation contracts.

## Capabilities

### New Capabilities
- `st5-machine-black-starter-coverage`: Complete Rust DSL/card-test/decklist coverage for ST-5: Starter Deck Machine Black, including all 16 unique cards and the exact starter deck composition.

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: Add reusable attack-history condition support so DSL-authored effects can test whether a referenced player attacked with any Digimon during the current turn.

## Impact

- **Card content:** `code/digimon-engine/cards/st5/*.yaml` for the 16 ST5 cards.
- **Tests:** `code/digimon-engine/tests/cards_behavioral/st5/*.rs` plus any focused DSL/predicate tests needed for the attack-history condition.
- **DSL crate:** `code/digimon-dsl/src/` for the new predicate schema and parser/lowering support.
- **Rust engine:** `code/digimon-engine/src/` for predicate evaluation, attack-history semantics, and blocker-trigger verification if the existing attack-target-change path is insufficient.
- **Deck surfaces:** `data/deck_library.json` and `data/tested_cards.json`, or the current equivalent fixture/implemented-card gates used by deck validation and starter-deck smoke checks.
- **Trackers:** `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, `qa/resolved-gaps.md`, and any ST5 readiness report or ledger introduced by the implementation pass.
- **No action/tensor contract impact:** all choices must use existing pending-selection/action surfaces; any discovery requiring a new action range stops this change and becomes a separate action/tensor contract proposal.
