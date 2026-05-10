# Wire Deferred Track C Modifier Variants

Date: 2026-05-09

Original task: complete the modifier-registry foundation in `code/digimon-engine/` by wiring the deferred `ModifierType` variants that already exist in `enums.rs` but lack payload, consult sites, tests, and DSL surface.

## Required setup

- Initialize the `DCGO` submodule and use it only as reference for consult-site shape and synth-identity overlay patterns.
- Printed card text, `docs/RULES_CONTEXT.md`, canonical rules, and Fandom rulings outrank DCGO.
- Do not transliterate C# coroutine implementation into Rust.
- Do not author new Python card scripts.
- Do not expand `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, frontend constants, or RL wrappers.

## Scope

### Wave A: payload-extension variants

Implement payload, install path, consult site, tests, docs, and DSL where appropriate:

- `ChangeTraits`: add and/or replace `Vec<Trait>`.
- `ChangeBaseCardName`: `CardName` overlay.
- `ChangeBaseCardColor`: `Vec<Color>` overlay.
- `ChangeCardNamesForDigiXros`: DigiXros alias `Vec<CardName>`.
- `ChangeCardDP`: base/printed DP override.
- `ChangeOriginDP`: original/printed DP override.
- `ChangeSAttack`: security attack delta/invert.
- `ChangeEndTurnMinMemory`: end-turn memory clamp.
- `ChangeLinkCost`: link cost adjustment.
- `ChangeLinkMax`: max link adjustment.
- `ChangeCardLevelForAssembly`: assembly-time level overlay; if cast-time assembly is missing, file the gap and skip behavior.

### Wave B: identity-refactor variants

- `TreatAsDigimon`: synth profile `(Kind, Level, Vec<Color>, Vec<Trait>, Dp)`.
- `ChangePermanentLevel`: absolute or delta level overlay.
- Add `Permanent::synth_identity()` or equivalent helper.
- Route production identity reads through the helper except documented printed/origin bypasses.
- Fixture: Tamer treated as level 4 yellow holy Digimon with 5000 DP; verify attacker legality, digivolution material legality, DP, read-site consistency, and expiry reversion.

### Wave C: player-scoped flags

- `OpponentCannotReduceDigivolveCost`: blocks opponent digivolve-cost reduction.
- `CannotPlayFromTrash`: blocks play-from-trash entry points for the affected player.
- `CannotReducePlayCost`: bilateral play-cost reduction lock with optional filter if available.

## Modifier payload request

Prefer a `ModifierPayload` enum on `ModifierEntry`:

```rust
enum ModifierPayload {
    None,
    Traits { add: Vec<Trait>, replace: bool },
    Name { value: CardName, base: bool },
    Colors { value: Vec<Color>, base: bool },
    DigiXrosNames { aliases: Vec<CardName> },
    Dp { value: Dp, base: bool, origin: bool },
    SecurityAttack { delta: i32, invert: bool },
    EndTurnMinMemory { value: i32 },
    LinkCost { delta: i32 },
    LinkMax { delta: i32 },
    LevelForAssembly { value: Level },
    SynthIdentity { kind: Kind, level: Level, colors: Vec<Color>, traits: Vec<Trait>, dp: Dp },
    LevelOverride { value: Level, delta: bool },
}
```

Add a debug assertion in `ModifierRegistry::install` that rejects mismatched `ModifierType`/payload pairs.

## Expiry hardening

- Add debug-build install guard for `Expiry::UntilCondition`: panic in debug builds until the continuous controller lands.
- In release builds, allow install but emit a warning per install.
- Add a follow-up gap entry in `docs/RUST_ENGINE_GAPS.md` for the continuous controller.

## Tests

Mirror `tests/combat/track_c_modifiers.rs` and `tests/track_c_gain_gates.rs` patterns.

For each variant, cover:

- Positive install and consult.
- Non-matching `cause_filter` does not apply.
- Expiry teardown.
- Negative scoping.
- Multi-stack behavior.
- Card-shaped fixture under `tests/cards_behavioral/` where a real card exists.

## Docs and trackers

Update:

- `docs/RUST_ENGINE_API.md` with payload semantics, consult-site checklist, `Permanent::synth_identity()`, and bypass rules.
- `docs/RUST_ENGINE_GAPS.md` for closed/narrowed deferred variants and new `Expiry::UntilCondition` controller gap.
- `qa/archetype-qa/engine-gaps.md` for treat-as/is-also/DigiXros alias/Link cost mechanics.
- `qa/dsl-vocab-gaps.md` for new DSL `kind:` slots.
- Relevant `qa/archetype-qa/dsl/*.md` rollups when cards become expressible.

## Verification commands

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat track_c_modifiers
cargo test --manifest-path code/digimon-engine/Cargo.toml --test track_c_gain_gates
cargo test --manifest-path code/digimon-engine/Cargo.toml --test modifier_disable_effect
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```
