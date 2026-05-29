## Why

ST-1 Gaia Red is the first starter deck and a compact, high-value Rust DSL coverage target: it exercises early-game basics, inherited effects, Blocker, security mirrors, targeted deletion, and Security Digimon DP modification without a large card pool. Implementing it gives the engine a canonical starter-deck smoke target while exposing two reusable DSL/engine gaps that should be solved as shared primitives rather than card-local shortcuts.

## What Changes

- Add faithful Rust DSL YAML coverage for every unique card in the worldwide `ST1-01` through `ST1-16` Gaia Red starter deck card pool.
- Add behavioral regression tests for ST-1 card effects, including no-effect/vanilla cards entering the implemented registry, inherited DP and Security Attack effects, Blocker with attack memory loss, targeted DP buffs, Tamer auras, option security mirrors, and deletion effects.
- Add or update a starter deck fixture so Gaia Red can be selected or smoke-tested as the printed 54-card deck: 4 Digi-Eggs plus the 50-card main deck.
- Close or explicitly track reusable gaps discovered during the assessment:
  - `ST1-09` needs a faithful "when this Digimon is blocked" surface.
  - `ST1-14` needs defender-side Security Digimon DP buffs, distinct from existing attacker-side opponent-security DP debuffs.
- Keep action/tensor contracts unchanged; if implementation reveals a need for new action IDs or observation fields, stop and split that into a separate action/tensor contract change.

## Capabilities

### New Capabilities
- `st1-gaia-red-starter-deck-coverage`: Defines the end-state coverage guarantees for the ST-1 Gaia Red starter deck in the Rust DSL engine and deck fixtures.

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: Add reusable DSL/engine support for faithfully expressing a source Digimon's "when this Digimon is blocked" effects.
- `security-card-effects`: Add reusable support for defender-side effects that modify the DP of that player's Security Digimon during security battles.

## Impact

- Rust DSL card specs under `code/digimon-engine/cards/st1/`.
- Rust behavioral tests under `code/digimon-engine/tests/cards_behavioral/st1/` and related DSL/combat/security tests for shared primitives.
- Deck fixture or library data used by starter deck selection, debug games, training smoke checks, and implemented-card filtering.
- Gap trackers including `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and any ST-1-specific QA ledger introduced by the implementation.
- No expected changes to `ACTION_SPACE_SIZE`, tensor layout metadata, PyO3 action exports, or frontend action constants.
