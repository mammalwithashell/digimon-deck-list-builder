## Why

TS Olympos is close to being usable in Rust-backed training runs, but the representative deck still has unauthored cards and a small set of reusable engine/DSL gaps that would force approximations if cards were implemented today. Closing those gaps now turns the recent deck-resolver and DCGO-reference exploration into a concrete path for faithful card coverage.

## What Changes

- Add reusable DSL and engine support for effect-driven Option use from hand, including cost-ceiling filters and the normal Option lifecycle.
- Add source-stack aggregate predicates so effects can target all Digimon tied for fewest digivolution cards.
- Add formula-valued De-Digivolve amounts for effects whose peel count depends on live game state.
- Extend timing-suppression modifiers so aura/predicate-scoped effects can prevent specific permanents from activating `[When Attacking]` and `[When Digivolving]` effects.
- Use those primitives to author and test the remaining representative TS Olympos Rust YAML cards needed for training eligibility.
- Reconcile TS Olympos QA and gap trackers so remaining blockers, closed gaps, and implemented-card counts reflect verified Rust source.

## Capabilities

### New Capabilities

- `ts-olympos-rust-coverage`: Defines the end-state readiness guarantees for including the TS Olympos archetype in Rust-backed training pools.

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: Adds source-stack aggregate predicates, formula-valued De-Digivolve amounts, and predicate-scoped timing-suppression authoring.
- `option-dual-play-mode`: Extends Option lifecycle/play-mode behavior to cover effect-driven Option use from hand outside the ordinary Main-phase user action.

## Impact

- Rust engine: `code/digimon-engine/src/`, including `EffectContext`, Option lifecycle, modifier/timing dispatch, action masks, and DSL lowering.
- DSL crate: `code/digimon-dsl/src/`, including schema, compiler, validator, packed forms, and predicate/formula surfaces.
- Card data: production YAML under `code/digimon-engine/cards/` for remaining representative TS Olympos cards.
- Tests: focused DSL tests plus card behavioral tests under `code/digimon-engine/tests/`.
- QA docs: TS Olympos archetype QA, DSL vocabulary gaps, Rust engine gaps, and resolved-gap notes.
- Training eligibility: pilot/generalist training deck pools can include TS Olympos once the representative pool is fully implemented and validated.
