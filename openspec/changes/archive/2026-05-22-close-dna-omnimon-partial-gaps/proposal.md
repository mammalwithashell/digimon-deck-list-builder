## Why

DNA Omnimon is nearly complete in the Rust DSL engine, but two cards still carry partial verdicts because their printed text depends on reusable engine/DSL primitives that are not yet present. Closing those gaps finishes the archetype without reintroducing raw Rust card escapes or approximation-only YAML.

## What Changes

- Add engine and DSL support for dynamic effective-name overlays from a Digimon's digivolution sources, covering BT17-102 Greymon's "[All Turns] has all names of level 3 and lower cards in its digivolution cards" clause.
- Add engine and DSL support for Delay effects triggered by an ally attack event, covering BT23-096 Comet Hammer's "[Your Turn] when one of your [CS] trait Digimon attacks, <Delay>" clause.
- Replace the current BT17-102 Koromon-name proxy with a true effective-name check once dynamic stack-name aliases exist.
- Author the omitted BT17-102 and BT23-096 DSL clauses and re-enable their behavioral tests.
- Keep DNA Omnimon at zero live `raw_rust` escapes; this change does not add new raw Rust card functions.
- Update DNA Omnimon verdict and gap trackers so the two remaining partial cards become implemented when tests pass.

## Capabilities

### New Capabilities

### Modified Capabilities
- `dna-omnimon-archetype-coverage`: Tighten the end-state requirement from "62 implemented / 2 partial" to fully implemented DNA Omnimon coverage, including BT17-102 dynamic source-name aliasing and BT23-096 Delay-on-attack behavior.

## Impact

- Rust engine identity/name query substrate: `code/digimon-engine/src/permanent.rs`, `code/digimon-engine/src/card_source.rs`, and name predicate evaluation.
- DSL identity and predicate lowering: `code/digimon-dsl/src/identity.rs`, `code/digimon-dsl/src/compile.rs`, `code/digimon-dsl/src/compiled.rs`, and `code/digimon-engine/src/dsl_cards/predicate.rs`.
- Delay/event dispatch: `code/digimon-engine/src/dsl_cards/lower_delay.rs`, `code/digimon-engine/src/combat.rs`, and `code/digimon-engine/src/effect_queue.rs`.
- Card YAML and tests for `BT17-102` and `BT23-096`.
- Gap trackers and DNA Omnimon verdict artifacts under `qa/`.
