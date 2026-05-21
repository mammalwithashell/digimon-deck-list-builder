# digimon-engine

Rust library crate — the **target source of truth** for Digimon TCG rules.

Replaces the sunset Python engine in [`engine_py_legacy/`](../engine_py_legacy/). Cross-engine divergences are tracked in [`docs/RUST_PYTHON_PARITY.md`](../../docs/RUST_PYTHON_PARITY.md) until the Python engine is retired.

## Surface

- `game.rs` — `Game` struct, turn state machine, phases
- `player.rs`, `permanent.rs`, `card_source.rs` — game-object types
- `card_data.rs`, `card_registry.rs`, `cards.rs` — card metadata + effect registry
- `effect.rs` — `Effect` + `EffectBuilder` + `CardEffect` trait
- `effect_context.rs` — `EffectContext`, the curated card-scripting API
- `effect_queue.rs` — triggered-effect queue + drainer
- `modifiers.rs` — typed `ModifierRegistry` with expiry
- `combat.rs` — attack state machine + interrupts (Alliance / Counter / Block)
- `selection.rs` — pending selection / interrupt state machine
- `tensor.rs` — observation tensor (1375 floats; parity with Python obs)
- `action/` — action space (2192) + mask + decoder
- `cards/test_cards.rs` — `TEST-001..022` worked examples used in behavioral tests
- `runners/` — `HeadlessRunner` (RL-shaped API)
- `debug_runner.rs` — deterministic test harness

## Consumers

- [`digimon-engine-py`](../digimon-engine-py/) — PyO3 bindings (`RustHeadlessGame`)
- [`src-tauri`](../src-tauri/) — desktop app embeds the crate directly (no Python)

## Tests

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
cargo test --manifest-path code/digimon-engine/Cargo.toml --test security_effects
cargo test --manifest-path code/digimon-engine/Cargo.toml --test test_cards_behavioral
```

## Authoring new card effects

TDD only — write a failing `DebugRunner` behavioral test under `tests/` (see `tests/test_cards_behavioral.rs`) **before** implementing the `CardEffect` struct. The no-approximations policy applies identically here: no stubs, no auto-selections, every choice must surface through `pending_selection` so the RL action space sees it.

See [`docs/RUST_ENGINE_API.md`](../../docs/RUST_ENGINE_API.md) for the `EffectContext` API and the full TDD walkthrough.
