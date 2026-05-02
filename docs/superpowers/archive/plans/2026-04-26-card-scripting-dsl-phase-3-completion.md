# Card Scripting DSL Phase 3 Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the remaining Phase 3 DSL work from `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` after the landed 3a-3c changes: formula payloads and scope, raw Rust formula dispatch, event-context predicates and bindings, next-turn scheduled effects, scheduled-effect re-entry after selections, and `OnDnaDigivolve` trigger wiring.

**Architecture:** Keep the DSL crate as the declarative schema and compiler boundary, then add the smallest engine-side runtime surfaces needed to execute those declarations. Formula extensions live behind a registry on `Game`; event metadata travels from `TriggerSource` into `QueuedEffect` and then into `EffectContext`; delayed effects remain queued in `Game::scheduled_effects` with extra generation metadata for next-turn semantics.

**Tech Stack:** Rust 2021, `digimon-dsl`, `digimon-engine`, serde/serde_yml, `cargo test`, existing `DebugRunner` DSL helpers, existing DSL integration tests under `code/digimon-engine/tests/dsl`.

---

## Scope Check

Phase 3a, 3b, and 3c have already landed. This plan only covers the remaining Phase 3 work:

- Phase 3d formula/runtime gaps:
  - `CardCountInZone` with zone/player payload.
  - Aggregate selector scope.
  - Raw Rust formula dispatch through an engine registry.
  - Event predicates and event bindings backed by runtime trigger context.
  - Scheduled next-turn generation checks.
- Phase 3e runtime wiring:
  - Continue scheduled drains safely when a scheduled effect parks on a selection.
  - Fire `OnDnaDigivolve` from both user-action DNA and effect-initiated DNA paths.

This is one cohesive runtime slice because each task touches the same DSL lowering, `EffectContext`, trigger queue, and scheduled-effect plumbing. If parallelized, split by task ownership exactly as listed below to avoid overlapping edits.

---

## File Map

Primary DSL schema and compiler files:

- `code/digimon-dsl/src/formula.rs`
- `code/digimon-dsl/src/compiled.rs`
- `code/digimon-dsl/src/compile.rs`
- `code/digimon-dsl/src/predicate.rs`
- `code/digimon-dsl/src/common.rs`

Primary engine runtime files:

- `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- `code/digimon-engine/src/dsl_cards/mod.rs`
- `code/digimon-engine/src/dsl_cards/predicate.rs`
- `code/digimon-engine/src/dsl_cards/binding_ref.rs`
- `code/digimon-engine/src/dsl_cards/step/modifiers.rs`
- `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs`
- `code/digimon-engine/src/dsl_cards/timing_map.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/game_actions.rs`
- `code/digimon-engine/src/scheduled_effects.rs`
- `code/digimon-engine/src/selection.rs`
- `code/digimon-engine/src/enums.rs`

Test harness and DSL tests:

- `code/digimon-engine/tests/dsl/main.rs`
- `code/digimon-engine/tests/dsl/phase2f2_formula_eval.rs`
- `code/digimon-engine/tests/dsl/phase2f4_schedule_delayed.rs`
- New files named in each task below.

Documentation to update at the end:

- `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`
- `docs/RUST_DSL_TEST_API.md`
- `qa/dsl-test-pool.md`

---

## Phase 0: Baseline Confirmation

- [ ] Run the current Phase 3 tests before editing:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3
```

Expected output includes:

```text
test result: ok. 8 passed; 0 failed
```

- [ ] Run a focused formula baseline:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_formula_eval
```

Expected output includes all existing formula tests passing. If the existing unused-variable warning in `phase2b_zone_moves.rs` appears, leave it alone unless this plan later touches that file.

- [ ] Commit the current documentation-only work before runtime edits if it is still unstaged:

```powershell
git status --short
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-3-completion.md
git commit -m "docs: clarify dsl phase 3 completion plan"
```

---

## Phase 1: CardCountInZone Payload

### Goal

Make `CardCountInZone` count cards in a selected zone for a selected player instead of returning `0`.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3d_formula_zone_count.rs`.

Use this structure:

```rust
use digimon_dsl::compile::compile_card;
use digimon_dsl::card::CardSpec;
use digimon_dsl::formula::{FormulaSpec, PerSelector};
use digimon_dsl::common::PlayerRef;
use digimon_dsl::predicate::Zone;
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::debug_runner::{dsl_card, DebugRunner};

#[test]
fn card_count_in_zone_counts_controller_trash() {
    let mut runner = DebugRunner::default();
    let target = runner.create_battle_permanent(0, "SRC", 3, 1000);
    runner.move_card_to_trash(0, "TRASH-A");
    runner.move_card_to_trash(0, "TRASH-B");
    runner.move_card_to_trash(1, "OPP-TRASH");

    let compiled = compile_card(&dsl_card(
        "COUNT",
        FormulaSpec::BasePerDelta {
            base: 1000,
            per: PerSelector::CardCountInZone {
                zone: Zone::Trash,
                of: PlayerRef::You,
            },
            delta: 2000,
        },
    ))
    .unwrap();

    let ctx = runner.effect_context_for(0);
    let value = formula_eval::evaluate(
        compiled.effects[0].steps[0].formula().unwrap(),
        &ctx,
        target,
    );

    assert_eq!(value, 5000);
}
```

Adjust helper names to the existing `DebugRunner` API. If a helper is missing, add it to `code/digimon-engine/src/debug_runner.rs` with a direct card-zone mutation used only by tests.

- [ ] Register the new test file in `code/digimon-engine/tests/dsl/main.rs`.

- [ ] Run the test and confirm it fails because `CardCountInZone` still evaluates to `0`:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_formula_zone_count
```

### DSL Schema and Compiler Changes

- [ ] Replace the payload-less `CardCountInZone` variant in `code/digimon-dsl/src/formula.rs` with a payload variant:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CardCountInZoneSpec {
    pub zone: Zone,
    pub of: PlayerRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    DigivolutionColorCount,
    CardCountInZone(CardCountInZoneSpec),
}
```

Import `Zone` from `crate::predicate` and `PlayerRef` from `crate::common`.

- [ ] Update `code/digimon-dsl/src/compiled.rs`:

```rust
pub enum CompiledPerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    DigivolutionColorCount,
    CardCountInZone {
        zone: CompiledZone,
        of: CompiledPlayerRef,
    },
}
```

- [ ] Update `compile_per_selector` in `code/digimon-dsl/src/compile.rs` to compile the zone and player payload:

```rust
PerSelector::CardCountInZone(spec) => CompiledPerSelector::CardCountInZone {
    zone: compile_zone(spec.zone),
    of: compile_player_ref(spec.of),
},
```

### Runtime Changes

- [ ] Add player-resolution and zone-count helpers in `code/digimon-engine/src/dsl_cards/formula_eval.rs`.

Use these names:

```rust
fn players_for_ref(of: CompiledPlayerRef, ctx: &EffectContext<'_>) -> Vec<usize>;
fn count_zone(zone: CompiledZone, player: usize, ctx: &EffectContext<'_>) -> i32;
```

`PlayerRef::You` maps to `ctx.player`; `Opponent` maps to the other player; `TurnPlayer` maps to `ctx.game.turn_player`; `Any` sums both players.

- [ ] Implement zone counts for every zone represented by `CompiledZone`:

```rust
match zone {
    CompiledZone::Hand => ctx.game.players[player].hand.len(),
    CompiledZone::Deck => ctx.game.players[player].deck.len(),
    CompiledZone::DigitamaDeck => ctx.game.players[player].digitama_deck.len(),
    CompiledZone::Security => ctx.game.players[player].security.len(),
    CompiledZone::Trash => ctx.game.players[player].trash.len(),
    CompiledZone::BattleArea => ctx.game.players[player].battle_area.len(),
    CompiledZone::BreedingArea => ctx.game.players[player].breeding_area.len(),
}
```

Cast the final result to `i32`.

- [ ] Replace the current `CompiledPerSelector::CardCountInZone => 0` branch with:

```rust
CompiledPerSelector::CardCountInZone { zone, of } => players_for_ref(*of, ctx)
    .into_iter()
    .map(|player| count_zone(*zone, player, ctx))
    .sum(),
```

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_formula_zone_count
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_formula_eval
```

- [ ] Commit:

```powershell
git add code/digimon-dsl/src/formula.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3d_formula_zone_count.rs code/digimon-engine/src/debug_runner.rs
git commit -m "feat: evaluate card count formulas by zone"
```

---

## Phase 2: Aggregate Scope

### Goal

Allow aggregate formulas such as lowest/highest DP and level to scan `you`, `opponent`, or both players instead of always scanning the effect controller battle area.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3d_aggregate_scope.rs`.

Include these tests:

- `lowest_dp_defaults_to_controller_scope_for_compatibility`
- `lowest_dp_can_scan_opponent_scope`
- `highest_level_can_scan_any_scope`

The opponent test should place controller permanents at DP 3000 and 5000, opponent permanents at DP 1000 and 7000, then assert opponent-lowest returns `1000`.

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_aggregate_scope
```

Expected failure: aggregate formulas cannot express or honor scope yet.

### DSL Schema and Compiler Changes

- [ ] Add `AggregateFormulaSpec` in `code/digimon-dsl/src/formula.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AggregateFormulaSpec {
    pub selector: AggregateSelector,
    #[serde(default = "default_aggregate_scope")]
    pub scope: PlayerRef,
}

fn default_aggregate_scope() -> PlayerRef {
    PlayerRef::You
}
```

- [ ] Change `CompoundFormula::Aggregate` from `AggregateSelector` to `AggregateFormulaSpec`.

The compatible YAML shape is:

```yaml
formula:
  aggregate:
    selector: lowest_dp
    scope: opponent
```

- [ ] Update example cards using old aggregate syntax if any exist:

```powershell
rg "aggregate:" code/digimon-engine/cards docs qa
```

For old scalar syntax, convert:

```yaml
aggregate: lowest_dp
```

to:

```yaml
aggregate:
  selector: lowest_dp
  scope: you
```

- [ ] Update `code/digimon-dsl/src/compiled.rs`:

```rust
pub enum CompiledFormula {
    Literal(i32),
    BasePerDelta { base: i32, per: CompiledPerSelector, delta: i32 },
    FloorDiv(Vec<CompiledFormula>),
    Max(Vec<CompiledFormula>),
    Min(Vec<CompiledFormula>),
    Aggregate {
        selector: CompiledAggregateSelector,
        scope: CompiledPlayerRef,
    },
    RawRust(String),
}
```

- [ ] Update `compile_formula` in `code/digimon-dsl/src/compile.rs`:

```rust
CompoundFormula::Aggregate(spec) => Ok(CompiledFormula::Aggregate {
    selector: compile_aggregate_selector(spec.selector),
    scope: compile_player_ref(spec.scope),
}),
```

### Runtime Changes

- [ ] In `formula_eval.rs`, replace the aggregate branch with one that resolves players from scope and scans those players' battle areas.

Use the same `players_for_ref` helper added in Phase 1.

- [ ] Keep empty aggregate behavior at `0`:

```rust
let values = players_for_ref(scope, ctx)
    .into_iter()
    .flat_map(|player| ctx.game.players[player].battle_area.iter())
    .filter_map(|permanent| aggregate_value(selector, permanent, ctx));

match selector {
    CompiledAggregateSelector::LowestDp | CompiledAggregateSelector::LowestLevel => values.min().unwrap_or(0),
    CompiledAggregateSelector::HighestDp | CompiledAggregateSelector::HighestLevel => values.max().unwrap_or(0),
}
```

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_aggregate_scope
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_formula_eval
```

- [ ] Commit:

```powershell
git add code/digimon-dsl/src/formula.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3d_aggregate_scope.rs
git commit -m "feat: add scoped aggregate formulas"
```

---

## Phase 3: Raw Rust Formula Registry

### Goal

Make `CompoundFormula::RawRust(name)` dispatch to an engine-side formula registry instead of evaluating to `0`.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3d_raw_rust_formula.rs`.

Test cases:

- `raw_rust_formula_dispatches_registered_callback`
- `unknown_raw_rust_formula_evaluates_to_zero`

The registered callback should return a distinctive value such as `4242`.

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_raw_rust_formula
```

Expected failure: `RawRust` currently evaluates to `0`.

### Runtime Registry

- [ ] Add `code/digimon-engine/src/dsl_cards/formula_registry.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use crate::effect_context::EffectContext;
use crate::handles::PermanentHandle;

pub type FormulaExtensionFn = fn(&EffectContext<'_>, PermanentHandle) -> i32;

#[derive(Clone, Default)]
pub struct FormulaExtensionRegistry {
    entries: Arc<HashMap<String, FormulaExtensionFn>>,
}

impl FormulaExtensionRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_entries(entries: impl IntoIterator<Item = (&'static str, FormulaExtensionFn)>) -> Self {
        let entries = entries
            .into_iter()
            .map(|(name, f)| (name.to_string(), f))
            .collect();
        Self {
            entries: Arc::new(entries),
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn evaluate(
        &self,
        name: &str,
        ctx: &EffectContext<'_>,
        target: PermanentHandle,
    ) -> Option<i32> {
        self.entries.get(name).map(|f| f(ctx, target))
    }
}
```

- [ ] Export the module from `code/digimon-engine/src/dsl_cards/mod.rs`:

```rust
pub mod formula_registry;
```

- [ ] Add a field to `Game` in `code/digimon-engine/src/game.rs`:

```rust
pub formula_extensions: FormulaExtensionRegistry,
```

Initialize it with `FormulaExtensionRegistry::empty()` wherever `Game` is constructed.

- [ ] Add a test helper on `DebugRunner`:

```rust
pub fn set_formula_extensions(&mut self, registry: FormulaExtensionRegistry) {
    self.game.formula_extensions = registry;
}
```

### Formula Evaluation

- [ ] Update `code/digimon-engine/src/dsl_cards/formula_eval.rs`:

```rust
CompiledFormula::RawRust(name) => ctx
    .game
    .formula_extensions
    .evaluate(name, ctx, target)
    .unwrap_or(0),
```

### DSL Validation Boundary

- [ ] Leave `code/digimon-dsl/src/raw_rust_registry.rs` as the compile-time validation trait:

```rust
pub trait RawRustRegistry: Send + Sync {
    fn contains_fn(&self, name: &str) -> bool;
}
```

- [ ] Add a comment above the engine registry explaining the split:

```rust
// digimon-dsl validates that a raw_rust name is allowed; the engine registry
// resolves allowed names into executable formula callbacks at runtime.
```

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_raw_rust_formula
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_formula_eval
```

- [ ] Commit:

```powershell
git add code/digimon-engine/src/dsl_cards/formula_registry.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/src/game.rs code/digimon-engine/src/debug_runner.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3d_raw_rust_formula.rs
git commit -m "feat: dispatch raw rust formula extensions"
```

---

## Phase 4: Event Context Predicates and Bindings

### Goal

Make DSL event predicates and `event_target` / `event_card` bindings use real trigger metadata instead of returning no value.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3d_event_context.rs`.

Test cases:

- `event_target_kind_predicate_matches_digivolving_permanent`
- `event_card_trait_predicate_matches_revealed_security_card`
- `event_target_binding_resolves_trigger_permanent`
- `event_card_binding_resolves_trigger_card`

Use direct trigger enqueue helpers if they exist. If not, add explicit `DebugRunner` helpers that call `enqueue_triggered` with `TriggerSource` variants and then drain the effect queue.

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_event_context
```

Expected failure: event bindings currently return `None` and predicates lack runtime trigger data.

### Trigger Context Model

- [ ] Add this struct in `code/digimon-engine/src/effect_context/mod.rs` or a new `code/digimon-engine/src/trigger_context.rs` module:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TriggerContext {
    pub target_permanent: Option<PermanentHandle>,
    pub target_card: Option<CardHandle>,
    pub event_card: Option<CardHandle>,
    pub source_player: Option<usize>,
    pub was_security_skill: bool,
}
```

Import `CardHandle` and `PermanentHandle` from the existing handle module.

- [ ] Add `pub current_trigger_context: Option<TriggerContext>` to `Game` and initialize it to `None`.

- [ ] Add `pub trigger_context: Option<TriggerContext>` to `QueuedEffect` in `code/digimon-engine/src/selection.rs`.

### Populate Queued Effects

- [ ] In `code/digimon-engine/src/effect_queue.rs`, construct `TriggerContext` from each `TriggerSource`:

```rust
fn trigger_context_for_source(source: &TriggerSource, source_permanent: Option<PermanentHandle>) -> TriggerContext
```

Mapping:

- `TriggerSource::Permanent(handle)`: `target_permanent = Some(handle)`, `target_card` is the current top card of that permanent.
- `TriggerSource::PlayerBattleArea(player)`: each queued permanent receives `target_permanent = Some(permanent.handle)` and top card as `target_card`.
- `TriggerSource::SecurityRevealed(card)`: `event_card = Some(card)`, `target_card = Some(card)`, `was_security_skill = true`.
- `TriggerSource::OnSecurityCheck { attacker, defender, revealed_card, .. }`: `target_permanent = Some(attacker)`, `event_card = Some(revealed_card)`, `target_card = Some(revealed_card)`, `source_player = Some(defender)`.

- [ ] Store that context on every `QueuedEffect` produced by `enqueue_triggered`.

### Run Effects With Current Trigger Context

- [ ] Wrap `run_queued_effect_inner` in `effect_queue.rs` so `Game::current_trigger_context` is set while the queued effect is checking conditions and executing steps:

```rust
let previous_trigger_context = self.current_trigger_context;
self.current_trigger_context = qe.trigger_context;
let result = self.run_queued_effect_inner(qe);
self.current_trigger_context = previous_trigger_context;
result
```

Use the existing method names in the file. If the current code cannot use this exact shape because of borrows, move the wrap to the outer caller that owns `&mut Game`.

### Predicate Evaluation

- [ ] Add compiled predicate fields if they are missing in `code/digimon-dsl/src/compiled.rs`:

```rust
pub event_target_kind: Option<CompiledCardKind>,
pub event_target_trait_has: Option<String>,
pub event_card_trait_has: Option<String>,
```

- [ ] Ensure `compile_predicate` in `code/digimon-dsl/src/compile.rs` lowers all three fields from `PredicateSpec`.

- [ ] In `code/digimon-engine/src/dsl_cards/predicate.rs`, evaluate:

```rust
predicate.event_target_kind
predicate.event_target_trait_has
predicate.event_card_trait_has
```

against `ctx.game.current_trigger_context`.

For target-kind and target-trait, prefer `target_permanent` top card. If no permanent exists, use `target_card`.

For event-card-trait, use `event_card`.

Missing trigger context makes the predicate fail.

### Binding Resolution

- [ ] Update `code/digimon-engine/src/dsl_cards/binding_ref.rs`:

```rust
CompiledBindingRef::EventTarget => ctx
    .game
    .current_trigger_context
    .and_then(|t| t.target_permanent.map(BindingValue::Permanent)),

CompiledBindingRef::EventCard => ctx
    .game
    .current_trigger_context
    .and_then(|t| t.event_card.or(t.target_card))
    .map(BindingValue::Card),
```

Use the actual binding enum constructors in the file.

### Selection Parking Safety

- [ ] If a queued effect parks on a selection, capture `trigger_context` in the parked continuation state.

The invariant: any continuation created while handling a queued effect must restore the same trigger context before executing its tail.

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_event_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3
```

- [ ] Commit:

```powershell
git add code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/effect_context code/digimon-engine/src/game.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/binding_ref.rs code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3d_event_context.rs
git commit -m "feat: wire dsl event trigger context"
```

---

## Phase 5: Scheduled Next-Turn Generation Semantics

### Goal

Support scheduled effects that fire at the next matching turn boundary, not the same turn boundary where they were scheduled.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3d_scheduled_generation.rs`.

Test cases:

- `end_of_your_next_turn_skips_current_end_turn`
- `end_of_opponents_next_turn_skips_current_opponent_end_turn`
- `until_next_unsuspend_skips_current_unsuspend_window`

Each test should:

1. Schedule a delayed memory gain or DP modifier.
2. Fire the same timing immediately.
3. Assert the effect has not applied.
4. Advance to the next eligible timing.
5. Assert the effect applies exactly once.

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_scheduled_generation
```

Expected failure: the compiled timing variants and generation checks are not present.

### DSL Timing

- [ ] Add these variants to `CompiledTiming` in `code/digimon-dsl/src/compiled.rs` if they are not already present:

```rust
EndOfYourNextTurn,
EndOfOpponentsNextTurn,
UntilNextUnsuspend,
```

- [ ] Add matching surface timing variants in the DSL timing enum if the spec surface does not already expose them.

- [ ] Update `compile_timing` in `code/digimon-dsl/src/compile.rs`.

### Engine Timing

- [ ] Add matching variants to `EffectTiming` in `code/digimon-engine/src/enums.rs`:

```rust
EndOfYourNextTurn,
EndOfOpponentsNextTurn,
UntilNextUnsuspend,
```

- [ ] Update `code/digimon-engine/src/dsl_cards/timing_map.rs` and `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs`.

### Scheduled Effect Metadata

- [ ] Add generation metadata to `ScheduledEffect` in `code/digimon-engine/src/scheduled_effects.rs`:

```rust
pub scheduled_at_turn: u32,
```

If the engine uses a different turn counter type, use that type consistently.

- [ ] Set `scheduled_at_turn` in `EffectContext::schedule_delayed` from the current `Game` turn counter.

- [ ] Add this helper in `scheduled_effects.rs`:

```rust
fn can_fire_scheduled(effect: &ScheduledEffect, current_turn: u32) -> bool {
    match effect.when {
        EffectTiming::EndOfYourNextTurn
        | EffectTiming::EndOfOpponentsNextTurn
        | EffectTiming::UntilNextUnsuspend => current_turn > effect.scheduled_at_turn,
        _ => true,
    }
}
```

- [ ] Use `can_fire_scheduled` inside `fire_scheduled_for_timing` before running a matching scheduled effect.

### Turn-Boundary Wiring

- [ ] Wire `EndOfYourNextTurn` and `EndOfOpponentsNextTurn` into the same end-turn drain locations that currently fire end-turn scheduled effects.

- [ ] Wire `UntilNextUnsuspend` into the existing unsuspend-phase boundary. Fire it before the permanent would be considered past the next unsuspend timing.

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3d_scheduled_generation
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f4_schedule_delayed
```

- [ ] Commit:

```powershell
git add code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/enums.rs code/digimon-engine/src/dsl_cards/timing_map.rs code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs code/digimon-engine/src/scheduled_effects.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3d_scheduled_generation.rs
git commit -m "feat: add next-turn scheduled effect semantics"
```

---

## Phase 6: Scheduled Drain Re-Entry After Selection Parking

### Goal

When a scheduled effect asks for a selection, finish that effect after the selection resolves and then continue draining the remaining scheduled effects for the same timing.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3e_scheduled_reentry.rs`.

Test case:

- Queue two scheduled effects for the same timing.
- First scheduled effect requires a selection and then gains memory.
- Second scheduled effect draws one card or gains a different memory amount.
- Fire the timing.
- Assert execution parks on the first selection.
- Resolve the selection.
- Assert both scheduled effects have completed in order.
- Assert `Game::scheduled_effects` no longer contains either completed effect.

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3e_scheduled_reentry
```

Expected failure: current scheduled drain code notes parked resume as a Phase 3 gap.

### Re-Entry State

- [ ] Add a scheduled-drain continuation state to `Game`:

```rust
pub scheduled_drain_tail: Option<ScheduledDrainTail>,
```

- [ ] Define `ScheduledDrainTail` in `scheduled_effects.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ScheduledDrainTail {
    pub timing: EffectTiming,
    pub remaining: Vec<ScheduledEffect>,
}
```

Initialize `scheduled_drain_tail` to `None` in all game constructors.

### Drain Algorithm

- [ ] Rewrite `fire_scheduled_for_timing` to:

1. Partition `game.scheduled_effects` into runnable matching effects and retained effects.
2. Run runnable effects in original order.
3. If `run_steps` returns `RunOutcome::Synchronous`, continue.
4. If `run_steps` returns `RunOutcome::Parked`, store all not-yet-run matching effects plus retained effects in `game.scheduled_drain_tail`, leave already-completed effects removed, and return immediately.

- [ ] Add:

```rust
pub fn resume_scheduled_drain(game: &mut Game)
```

This function takes `scheduled_drain_tail`, restores its `remaining` list into the same drain algorithm, and continues until completion or another park.

### Selection Resolution Hook

- [ ] Find the hook that currently calls `drain_dsl_outer_tail` after resolving a DSL selection.

- [ ] After `drain_dsl_outer_tail` completes and no selection is pending, call `resume_scheduled_drain(game)`.

Ordering invariant:

1. Finish the parked effect tail.
2. Resume scheduled drain for remaining scheduled effects.
3. Let the normal effect queue continue.

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3e_scheduled_reentry
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f4_schedule_delayed
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3
```

- [ ] Commit:

```powershell
git add code/digimon-engine/src/game.rs code/digimon-engine/src/scheduled_effects.rs code/digimon-engine/src/dsl_cards/step/mod.rs code/digimon-engine/src/selection.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3e_scheduled_reentry.rs
git commit -m "feat: resume scheduled dsl effects after selections"
```

---

## Phase 7: OnDnaDigivolve Trigger Wiring

### Goal

Fire `OnDnaDigivolve` consistently when DNA digivolution succeeds, including effect-initiated DNA and user-action DNA paths.

### Test First

- [ ] Add `code/digimon-engine/tests/dsl/phase3e_on_dna_digivolve.rs`.

Test cases:

- `effect_initiated_dna_fires_on_dna_digivolve`
- `user_action_dna_fires_on_dna_digivolve`
- `non_dna_digivolve_does_not_fire_on_dna_digivolve`

Use a simple inherited or global effect that gains `1` memory on `on_dna_digivolve`, then assert the memory change happens once.

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3e_on_dna_digivolve
```

Expected failure: effect-initiated DNA currently does not enqueue `OnDnaDigivolve`, and the user-action DNA callback path is incomplete.

### Shared DNA Execution Helper

- [ ] Add a shared helper in `code/digimon-engine/src/game_actions.rs` or a new `code/digimon-engine/src/dna_digivolve.rs`:

```rust
pub fn resolve_dna_digivolve(
    game: &mut Game,
    player: usize,
    materials: Vec<PermanentHandle>,
    target_card: CardHandle,
    cost: i32,
    source: DnaDigivolveSource,
) -> Option<PermanentHandle>
```

- [ ] Define:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaDigivolveSource {
    UserAction,
    EffectInitiated,
}
```

The helper must:

1. Validate that all materials are controlled by the player.
2. Move the target card from hand or the effect-selected source zone into the final stack.
3. Merge materials into the new stack in existing engine order.
4. Apply memory cost exactly once.
5. Return the resulting permanent handle.

Use existing DNA code if it already implements any of these steps; move shared logic rather than creating a second implementation.

### Trigger Sequence

- [ ] After successful DNA resolution, enqueue triggers in this order:

```rust
game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(result));
game.enqueue_triggered(EffectTiming::OnDnaDigivolve, TriggerSource::Permanent(result));
game.enqueue_triggered(EffectTiming::OnDigivolve, TriggerSource::Permanent(result));
```

Keep the current `WhenDigivolving` and `OnDigivolve` behavior intact.

### Effect-Initiated DNA

- [ ] Update `EffectContext::effect_initiated_dna_digivolve` in `code/digimon-engine/src/effect_context/mod.rs` to call the shared helper.

- [ ] Ensure `EffectInitiatedDigivolve.cost_delta` still applies before final cost payment.

### User-Action DNA

- [ ] Update the action callback path in `code/digimon-engine/src/game_actions.rs` so user selection resolves into the same helper.

- [ ] If action masking uses a separate DNA legality check, keep that legality check as the source of legal action generation and use the shared helper only after a legal action is selected.

### Verification

- [ ] Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3e_on_dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3
```

- [ ] Commit:

```powershell
git add code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/enums.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase3e_on_dna_digivolve.rs
git commit -m "feat: fire on dna digivolve triggers"
```

---

## Phase 8: Documentation and Full Verification

### Documentation

- [ ] Update `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` Phase 3 status:

```md
- Phase 3d: complete for formula payloads, aggregate scope, raw Rust formula dispatch, event predicates, event bindings, and next-turn scheduled effects.
- Phase 3e: complete for scheduled drain re-entry and OnDnaDigivolve wiring.
```

- [ ] Update `docs/RUST_DSL_TEST_API.md` with:

  - Formula payload examples for `card_count_in_zone`.
  - Aggregate scope example.
  - Event binding examples for `event_target` and `event_card`.
  - Scheduled next-turn test helper guidance.

- [ ] Update `qa/dsl-test-pool.md` with the next cards now unblocked by Phase 3d/3e. Include at least one card that uses:

  - zone count formula,
  - aggregate scope,
  - event-card predicate,
  - delayed next-turn effect,
  - DNA-trigger timing.

### Full Verification

- [ ] Run all DSL tests:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
```

Expected output: all DSL tests pass.

- [ ] Run the engine crate test suite:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected output: all engine crate tests pass. Existing warnings may remain if unrelated to the changed files.

- [ ] Run a workspace-level check if this repository uses a top-level Cargo workspace for the Rust crates:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --all-targets
```

### Final Commit

- [ ] Commit docs and any verification-only test fixture updates:

```powershell
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md docs/RUST_DSL_TEST_API.md qa/dsl-test-pool.md
git commit -m "docs: record completed dsl phase 3 runtime coverage"
```

- [ ] Confirm the final branch status:

```powershell
git status --short --branch
```

Expected output has no unstaged edits except files intentionally left for review.

---

## Acceptance Criteria

- `CardCountInZone` evaluates real zone counts with `zone` and `of` payloads.
- Aggregate formulas can scan controller, opponent, or both battle areas.
- `RawRust` formulas dispatch through an engine runtime registry while the DSL crate keeps validation-only registry semantics.
- Event predicates can inspect target kind, target traits, and event-card traits from runtime trigger context.
- `event_target` and `event_card` bindings resolve during triggered effects and stay available after selection parking.
- Next-turn scheduled effects do not fire on the same turn they were scheduled.
- Scheduled effects that park on a selection resume and continue draining remaining scheduled effects.
- `OnDnaDigivolve` fires for both effect-initiated DNA and user-action DNA.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl` passes.
- Phase 3 status in the spec and test-plan docs matches the implementation.
