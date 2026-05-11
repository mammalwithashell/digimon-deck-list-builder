# Track J: DSL Formula Plumbing for Predicate Thresholds and Result-Bound Bindings

> Reference plan saved from the Track J prompt on 2026-05-10. Use this file as the durable implementation checklist for the Rust engine crate at `code/digimon-engine/` and the YAML DSL crate at `code/digimon-dsl/`.

## Goal

Land the DSL formula plumbing layer that lets predicates and selection counts read runtime values from formulas, earlier-step bindings, aggregates, and per-effect result logs. This closes residual gaps across Alter-S Ladder, BG Imperial, Zephagamon, TS Olympos, BT21, and BT24 migration work without expanding action space or tensor contracts.

## Source Priority

Use printed card text in `data/cards.json`, `docs/RULES_CONTEXT.md`, the canonical rules PDF when needed, Fandom card rulings, then DCGO C# as the implementation reference. DCGO is a reference for processing order, formula vocabulary, and binding shape only; printed text wins on disagreement.

## Required Reading

Read these before changing behavior:

1. `CLAUDE.md`, especially Working Rules 17-22.
2. `docs/RUST_ENGINE_API.md`.
3. `docs/RUST_PYTHON_PARITY.md`.
4. `code/digimon-dsl/src/formula.rs`.
5. `code/digimon-dsl/src/predicate.rs`.
6. `code/digimon-dsl/src/step.rs`, `clause.rs`, `compile.rs`, `compiled.rs`, and `validator.rs`.
7. Any DSL binding/result modules that already exist.
8. `code/digimon-engine/src/dsl_cards/predicate.rs`.
9. `code/digimon-engine/src/dsl_cards/formula_eval.rs`.
10. DSL step lowering and selection files under `code/digimon-engine/src/dsl_cards/`.
11. `code/digimon-engine/src/effect_context/mod.rs`.
12. Existing tests under `code/digimon-engine/tests/dsl/` and `code/digimon-engine/tests/cards_behavioral/`.
13. Gap trackers under `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/dsl/`, `docs/RUST_ENGINE_GAPS.md`, and `qa/archetype-qa/engine-gaps.md`.
14. DCGO reference files:
    - `DCGO/Assets/Scripts/Script/CardEffectCommons/GameContextDeterminarion.cs`
    - `DCGO/Assets/Scripts/Script/CardEffectCommons/MinMax_DP_Cost_Level/`
    - `DCGO/Assets/Scripts/Script/Effects.cs`
    - `DCGO/Assets/Scripts/Script/CardEffectCommons/IsDigivolvedByTheEffect.cs`
    - `DCGO/Assets/Scripts/Script/CardEffects/ChangeCardLevelForAssemblyClass.cs`
    - `DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/PermanentEnterField/PermanentEnterField.cs`
    - `DCGO/Assets/Scripts/Script/Permanent.cs`

## Implementation Order

### 1. Formula Node Extensions

Audit the existing `Formula` enum and add only missing nodes needed by the gap batch. Candidate nodes from the prompt are:

- `SuspendedCount { scope }`
- `SourceCount { permanent }`
- `SourceCountByPredicate { permanent, filter }`
- `SourceColorsDistinct { permanent }`
- `HandSize { player }`
- `MemoryAbs`
- `SecurityCount { player }`
- `BindingPlayCost { binding }`
- `BindingDp { binding }`
- `BindingLevel { binding }`
- `BindingTraitCount { binding, trait_filter }`
- `Min(Vec<Formula>)`
- `Max(Vec<Formula>)`
- `Sum(Vec<Formula>)`
- `FloorDiv { numerator, denominator }`
- `CountReturnedCards`

For each node, add serde, display/parser support, validator checks, and engine-side runtime evaluation through `EffectContext` helpers.

### 2. BindingId and Binding Scope

Add a stable per-card-effect-resolution binding identifier if one does not already exist:

```rust
#[derive(Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct BindingId(pub u8);
```

Bindings declared in step N are visible only to steps N+1 through the end of the same effect. Bindings do not survive across effect boundaries. The runtime map should carry typed bound values such as cards, permanents, players, card lists, permanent lists, and integers. Add type-checked accessors on `EffectContext`, and document the scope rule in `docs/RUST_ENGINE_API.md`.

### 3. Predicate Threshold Extension

Extend every cost, DP, count, level, security, hand-size, and memory threshold predicate to accept literal or formula thresholds while keeping existing YAML unchanged:

```yaml
play_cost_lte: 5
play_cost_lte:
  formula:
    binding_play_cost: 1
```

Use a `FormulaOrLiteral`-style type or the local equivalent. The evaluator must evaluate formulas at runtime through the same formula evaluator as selection counts.

### 4. Validator Extensions

The validator must reject formula bindings that reference an undeclared binding ID or a binding declared too late in the same effect. Add negative tests for each binding shape. Runtime missing bindings should fail loudly when schema-only paths are encountered.

### 5. Formula-Driven Selection Counts

Audit DSL and engine selection count sites and extend count parameters that represent player-visible selection bounds to accept literal or formula counts. Evaluate the formula at selection-install time and clamp to the existing pending-selection limit. Formula values below zero clamp to zero. Document the chosen zero-count behavior and make it test-covered.

### 6. Result-Bound Predicates and EffectResultLog

Add result-bound predicates that read only what the current effect resolution has already done. Candidate predicates:

- `EffectSuspendedAnyOwnDigimon`
- `EffectReturnedAnyCard`
- `EffectDeletedAnyOwnDigimon`
- `EffectDeletedAnyOpponentDigimon`
- `EffectPlayedAnyDigimon`
- `EffectDigivolvedAnyDigimon`
- `EffectAddedAnyCardToHand`
- `AnyReturnedCard`
- `BindingIsNone(BindingId)`
- `BindingIsPresent(BindingId)`

Carry an append-only per-effect `EffectResultLog` on `EffectContext`; drop it at effect end. Every relevant mutation helper appends to the log.

### 7. DSL Surface and Backward-Compatible Serde

Existing cards must keep parsing unchanged. New YAML forms must support formula thresholds, formula selection counts, result-bound predicates, and `bind_as` declarations.

### 8. Card-Shaped Fixtures

Land at least one card-shaped fixture for each gap family:

- BT15-096 Supreme Connection!: activate and pass the 6 ignored `G-PLAY-COST-LTE-BINDING` tests.
- BT21-102: same binding-relative play-cost threshold shape.
- EX11-074 Zephagamon-style result-bound own-suspend branch.
- BT20-101 Zephagamon-style suspended-count divided by 2 selection count.
- One TS Olympos source-stack aggregate driver.
- One BG Imperial G-BG-04 source-count or source-name predicate in a triggered selection filter.

### 9. Tracker and Documentation Updates

Update the relevant gap trackers with proof commands:

- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/dsl/alter-s-ladder-2026-05-03.md`
- `qa/archetype-qa/dsl/zephagamon-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`
- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `docs/RUST_ENGINE_API.md`

## Acceptance Gates

- Every cost, DP, count, level, security, hand-size, and memory predicate accepts literal and formula thresholds.
- Every affected selection-count parameter accepts literal and formula counts.
- A `BindingId` declared by one step is visible to later steps in the same effect.
- The validator rejects undeclared or too-late bindings.
- Result-bound predicates read from the current effect's append-only result log.
- BT15-096's 6 ignored tests activate and pass.
- BT21-102 fixture passes.
- EX11-074 and BT20-101 Zephagamon fixtures pass.
- TS Olympos and BG Imperial fixtures pass.
- Every new player-visible choice surfaces through `pending_selection` and the action mask.

## Constraints

- No approximations, stubs, hidden auto-selections, or silent skips.
- Do not expand `ACTION_SPACE_SIZE`, active tensor profiles, PyO3 exports, frontend constants, or RL wrappers.
- Do not transliterate DCGO; use it as a C# reference for vocabulary and processing shape.
- Write failing tests before production code.
- Do not author Python-side card scripts.
- Do not import from `code/engine_py_legacy/`.
- Preserve backward-compatible YAML parsing.
- Binding and result-log state is per effect resolution, never global.
- File gaps instead of building private substitutes for missing event payload, modifier, replacement, selection, or zone-movement primitives owned by other tracks.

## Verification Commands

```powershell
cargo test --manifest-path code/digimon-dsl/Cargo.toml
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt21_102
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

