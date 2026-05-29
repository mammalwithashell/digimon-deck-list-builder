## Why

ST-3: Starter Deck Heaven's Yellow has static card metadata in the Rust engine, but its cards are not DSL-authored and therefore are not registered as implemented card effects. Implementing the starter deck gives the Rust engine a complete early yellow deck, improves starter-deck smoke coverage, and exercises foundational yellow mechanics such as DP reduction, Recovery, security-option effects, Tamer security play, and inherited memory triggers.

## What Changes

- Add faithful Rust DSL YAML implementations for all 16 ST3 card IDs (`ST3-01` through `ST3-16`) under `code/digimon-engine/cards/st3/`.
- Add per-card behavioral tests for all effectful ST3 cards and structural/load tests for vanilla cards, using `DebugRunner` and the embedded DSL card pack.
- Add a canonical ST-3 starter deck fixture or library entry so the 54-card Heaven's Yellow product list can be loaded for smoke tests and local play/training workflows.
- Verify every ST3 card appears in `load_implemented_card_ids()` once the embedded DSL pack is rebuilt.
- Document any genuinely blocked printed-text clause as a reusable DSL or engine gap instead of approximating card behavior.

## Capabilities

### New Capabilities

- `st3-heavens-yellow-starter-coverage`: Covers complete Rust DSL implementation, behavioral coverage, and deck fixture/loadability for ST-3: Starter Deck Heaven's Yellow.

### Modified Capabilities

- None.

## Impact

- Affected code: `code/digimon-engine/cards/st3/`, `code/digimon-engine/tests/cards_behavioral/`, starter/deck fixture or deck-library loading code, and potentially QA gap trackers if implementation reveals missing reusable primitives.
- Runtime behavior: ST3 cards become registered Rust DSL card effects and can participate in Rust-backed games, agent simulations, and deck-validation flows.
- API/contracts: No action-space, tensor-profile, PyO3 API, or model metadata contract changes are expected.
