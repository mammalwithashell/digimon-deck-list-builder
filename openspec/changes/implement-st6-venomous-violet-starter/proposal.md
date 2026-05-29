## Why

ST-6: Starter Deck Venomous Violet is a compact, self-contained purple starter deck whose card pool is already present as metadata but not executable Rust DSL behavior. Implementing it provides a low-risk starter-deck target for Rust-engine smoke games while expanding purple trash, recursion, Blocker, Retaliation, Digi-Burst, and Option lifecycle coverage.

## What Changes

- Add faithful Rust DSL implementations for all sixteen unique ST6 cards (`ST6-01` through `ST6-16`) under `code/digimon-engine/cards/st6/`.
- Add focused Rust behavioral tests for every effect-bearing ST6 card, including inherited effects, Security effects, optional choices, and multi-step Option resolution.
- Add a playable Venomous Violet starter-deck fixture/library entry with the official 54-card product composition: four Digi-Eggs plus fifty main-deck cards.
- Verify the full starter deck is accepted by the implemented-card registry and can run through Rust headless smoke play without relying on hidden choices or no-op placeholders.

## Capabilities

### New Capabilities

- `st6-venomous-violet-coverage`: Coverage guarantees for the ST-6 Venomous Violet starter deck in the Rust DSL engine, including faithful card behavior, behavioral tests, and a playable starter-deck fixture.

### Modified Capabilities

- None.

## Impact

- Affected Rust DSL card specs: `code/digimon-engine/cards/st6/`.
- Affected Rust tests: `code/digimon-engine/tests/cards_behavioral/st6/` and module registration.
- Affected deck data or fixtures: starter-deck/deck-library data used by engine smoke tests and training/deck tooling.
- No tensor layout, action-space, PyO3 API, frontend constant, or model metadata contract changes are intended.
