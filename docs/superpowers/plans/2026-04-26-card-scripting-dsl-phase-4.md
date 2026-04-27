# Card Scripting DSL Phase 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Phase 4 raw_rust escape hatch and long-tail migration path so DSL-authored cards can delegate the last stubborn mechanics to typed Rust functions, while shrinking `code/digimon-engine/src/cards/` to test cards, token cards, keyword auto-effects, and a raw-rust function module.

**Architecture:** Add an engine-owned `EngineRawRustRegistry` that resolves raw function names at runtime for all three DSL granularities: step-level process functions, whole-clause functions, and formula functions. Thread an immutable `Arc<EngineRawRustRegistry>` through `DslCardEffect`, recursive DSL step execution, scheduled/delayed bodies, and formula evaluation; then add a migration report plus final retirement guard so the last hand-written production cards are either converted to YAML or moved behind named raw-rust functions.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl`, `cargo test --test dsl`, PowerShell for migration/report commands.

**Spec reference:** `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` §§ 6, 7.5, 7.6. This plan assumes most of Phase 3 (§7.4) is landed, but explicitly folds the remaining Phase 3 carry-forward items into Task 0.5 before raw_rust migration begins: replacement process body lowering, two remaining selection helpers, delay/partition gating, cost-delta expressiveness, and DNA cost data ingestion.

---

## File Structure

- Create: `code/digimon-engine/src/dsl_cards/raw_rust.rs` — engine-side registry with typed function maps for step, declarative, and formula raw_rust calls.
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs` — store the registry on `DslCardEffect`, expose `register_dsl_cards_with_raw`, and dispatch whole-clause raw_rust.
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs` — pass the raw registry into triggered process closures.
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs` — thread a small `StepRuntime` through recursive step execution and dispatch `CompiledStep::RawRust`.
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs` — fold in Phase 3 replacement process body lowering before Phase 4 raw-rust migration.
- Modify: `code/digimon-engine/src/dsl_cards/step/control_flow.rs` — pass `StepRuntime` into nested `run_steps` calls.
- Modify: `code/digimon-engine/src/dsl_cards/step/iteration.rs` — pass `StepRuntime` into `for_each` / `per_selected` body execution.
- Modify: `code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs` — pass `StepRuntime` into nested selecting-player body execution.
- Modify: `code/digimon-engine/src/dsl_cards/step/replacement.rs` — add replacement process verbs such as `cancel_leave`, `redirect_leave_to`, and `substitute_leave_with`.
- Modify: `code/digimon-engine/src/effect_context/selections.rs` — add `select_any_permanent` and `select_dna_pair` helpers.
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs` — lower DSL selection specs for cross-side permanent and DNA-pair selection.
- Modify: `code/digimon-engine/src/dsl_cards/lower_delay.rs` — honor `active_when` for delay declaratives.
- Modify: `code/digimon-engine/src/dsl_cards/lower_partition.rs` — honor `active_when`, `sources`, and `exclude_cause` for partition declaratives where the engine replacement context exposes enough information.
- Modify: `code/digimon-dsl/src/step.rs` / `code/digimon-dsl/src/compiled.rs` / `code/digimon-dsl/src/compile.rs` — widen cost-delta IR so DSL can express printed-cost-minus-N separately from fixed-cost-N.
- Modify: `code/digimon-engine/src/card_data.rs` and relevant metadata import/export path — populate `dna_costs` from card metadata instead of leaving runtime DNA helpers inert.
- Modify: `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs` — capture runtime into scheduled bodies.
- Modify: `code/digimon-engine/src/scheduled_effects.rs` — store and replay `StepRuntime` when delayed effects fire.
- Modify: `code/digimon-engine/src/effect_context/mod.rs` — add `schedule_delayed_with_runtime` helper used by DSL lowering.
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs` — evaluate `CompiledFormula::RawRust` through the registry.
- Create: `code/digimon-engine/src/cards/raw_rust/mod.rs` — production registrations for long-tail raw-rust functions.
- Modify: `code/digimon-engine/src/cards.rs` — register raw-rust functions and pass them to DSL card registration.
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_registry.rs` — registry tests.
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_step.rs` — step-level dispatch tests.
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_clause.rs` — whole-clause dispatch tests.
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_formula.rs` — formula dispatch tests.
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_end_to_end.rs` — YAML-to-effect end-to-end test across all three granularities.
- Create: `code/digimon-engine/tests/dsl/phase4_phase3_carryforward.rs` — regression tests for the folded Phase 3 carry-forward items.
- Modify: `code/digimon-engine/tests/dsl/main.rs` — include the new Phase 4 test modules.
- Create: `code/tools/dsl_long_tail_report.py` — report hand-written production cards that must be migrated or moved into raw_rust.
- Create: `code/digimon-engine/tests/dsl/phase4_retirement_guard.rs` — final guard that fails when production hand-written card modules remain outside the allowed shell.
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — mark Phase 4 landed and document any raw function signature deviations from §6.

---

## Task 0: Pre-flight Phase 3 Gate

**Files:**
- Read: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`
- Read: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Read: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Read: `code/digimon-engine/src/dsl_cards/formula_eval.rs`

- [ ] **Step 1: Verify Phase 3 prerequisites**

Run:

```powershell
Select-String -Path code\digimon-engine\src\dsl_cards\lower_replacement.rs -Pattern 'run_steps','cancel','redirect','substitute','handled'
Select-String -Path code\digimon-engine\src\dsl_cards\step\iteration.rs -Pattern 'Parked','resume','remaining'
Select-String -Path code\digimon-engine\src\dsl_cards\formula_eval.rs -Pattern 'CardCountInZone','Aggregate','RawRust'
```

Expected: replacement lowering runs a real process body rather than the Phase 1 no-op; iteration comments no longer say parked iteration aborts remaining iterations; `CardCountInZone` has a zone-aware implementation; `RawRust` is the only formula primitive still waiting on this plan.

- [ ] **Step 2: Route any known carry-forward misses into Task 0.5**

Expected action if the check still shows one of the known reducer gaps below: complete Task 0.5 before starting Task 1. If the check surfaces a different Phase 3 stub outside Task 0.5's scope, stop and open or run `docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-3.md` first, because Phase 4 raw functions must plug into a complete DSL engine surface.

---

## Task 0.5: Fold Remaining Phase 3 Reducer Work Into Phase 4

**Why this is part of Phase 4:** these items are Phase 3 leftovers, but each directly reduces the amount of long-tail card text that would otherwise need `raw_rust`. Land them before Task 1 so Phase 4 uses raw_rust as an escape hatch, not as a substitute for common DSL verbs.

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_delay.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_partition.rs`
- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-dsl/src/validator.rs`
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: metadata import/export path that owns `dna_costs` population
- Create: `code/digimon-engine/tests/dsl/phase4_phase3_carryforward.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [x] **Step 1: Add replacement-process DSL lowering**

Implement DSL lowering for replacement process bodies instead of leaving `lower_replacement.rs` as an engine-only placeholder. Add step verbs that call the existing replacement outcome setters on `EffectContext`:

```yaml
- cancel_leave: {}
- redirect_leave_to: { zone: hand }
- substitute_leave_with: { target: $other }
- handle_replacement: {}
```

Add only the target resolution needed by current Phase 3 replacement substrates. If `move_self_under` or `trash_source_from_stack` is needed for a carried card, lower those verbs in the same step module; otherwise document them as still pending inside the carry-forward test file.

Expected tests:

```powershell
cargo test -p digimon-engine --test dsl replacement_process -- --nocapture
cargo test -p digimon-engine --test replacements -- --nocapture
```

- [x] **Step 2: Add remaining selection helpers**

Add `EffectContext::select_any_permanent` for cross-side battle-area selection and `EffectContext::select_dna_pair` for selecting a validated pair of own Digimon for a hand card's DNA requirements. Thread both through the DSL selection step lowering so YAML can express:

```yaml
- select_any_permanent:
    filter: { has_trait: Rock }
    bind_as: target
- select_dna_pair:
    hand_card: $target_card
    bind_as: materials
```

Acceptance criteria:
- `select_any_permanent` can return either player's permanent and preserves owner/handle information in bindings.
- `select_dna_pair` rejects invalid pairs using the same DNA requirement helper as action/effect-initiated DNA.
- Pending selection masks stay legal for the actual selecting player.

Expected tests:

```powershell
cargo test -p digimon-engine --test dsl phase4_phase3_carryforward -- --nocapture
cargo test -p digimon-engine --test selection -- --nocapture
```

- [x] **Step 3: Honor delay and partition declarative gates**

In `lower_delay.rs`, enforce `active_when` before installing or firing the delayed body. In `lower_partition.rs`, honor `active_when`, `sources`, and `exclude_cause` where the current replacement context exposes the relevant cause/source fields.

Acceptance criteria:
- A delay with false `active_when` does not schedule.
- A partition clause with false `active_when` does not install or fire.
- `exclude_cause` prevents partition replacement for the named cause while leaving other causes intact.
- Any unsupported source/cause dimension fails validation rather than silently ignoring author intent.

Expected tests:

```powershell
cargo test -p digimon-engine --test dsl phase4_phase3_carryforward -- --nocapture
cargo test -p digimon-engine --test dsl phase2f4_schedule_delayed -- --nocapture
cargo test -p digimon-engine --test replacements -- --nocapture
```

- [x] **Step 4: Widen cost-delta IR**

Widen the DSL cost delta shape so it can represent:

```yaml
cost: printed
cost: free
cost: { fixed: 3 }
cost: { reduce: 2 }
cost: { formula: { ... } }
```

Map these distinctly in `step/play_digivolve.rs`:
- `printed` -> `CostDelta::Reduce(0)`
- `free` -> `CostDelta::Free`
- `fixed` -> `CostDelta::Fixed(n)`
- `reduce` -> `CostDelta::Reduce(n)`
- formula -> the existing formula-cost path, or validation error if the runtime cannot safely apply it yet

Expected tests:

```powershell
cargo test -p digimon-dsl cost -- --nocapture
cargo test -p digimon-engine --test dsl phase2f1_digivolve_steps -- --nocapture
```

- [x] **Step 5: Populate DNA cost data**

The DNA execution and trigger path is now useful only when metadata carries real `dna_costs`. Wire the card metadata import/export path so `[DNA Digivolve]` costs are serialized into `CardData::dna_costs` and survive loading.

Acceptance criteria:
- Existing card JSON without `dna_costs` remains backward-compatible.
- A fixture card with DNA requirements loads with non-empty `dna_costs`.
- Main-phase DNA action masks and effect-driven `select_dna_pair` use the populated data.

Expected tests:

```powershell
cargo test -p digimon-engine --test mask_and_tensor dna -- --nocapture
cargo test -p digimon-engine --test dsl phase3_dna_digivolve_triggers -- --nocapture
cargo test -p digimon-engine --test dsl phase4_phase3_carryforward -- --nocapture
```

- [x] **Step 6: Update the Phase 3 gate and carry-forward notes**

After Steps 1-5 pass, update Task 0's expected gate text so these items no longer appear as blockers. Leave a short note in this plan explaining which Phase 3 leftovers were intentionally not folded in.

Do not fold these into Phase 4:
- broad combat timing gaps (`WhenAttacking`, `EndOfAttack`, `EndOfBattle`, counter-timed Options)
- Ace Overflow
- full keyword parity
- archetype-by-archetype migration batches
- general release/app/admin work

Reducer closeout was executed via
`docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-3-reducers.md`.
The landed test modules are `phase3_reducer_replacement`,
`phase3_reducer_partition`, `phase3_reducer_selection`, and
`phase3_reducer_costs`; the older placeholder
`phase4_phase3_carryforward` file was not created.

- [ ] **Step 7: Commit the carry-forward reducer batch**

```powershell
git add code/digimon-dsl/src code/digimon-engine/src/dsl_cards code/digimon-engine/src/effect_context code/digimon-engine/src/card_data.rs code/digimon-engine/tests/dsl/phase4_phase3_carryforward.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: fold phase 3 reducer work into phase 4"
```

---

## Task 1: Engine RawRust Registry

**Files:**
- Create: `code/digimon-engine/src/dsl_cards/raw_rust.rs`
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_registry.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing registry test**

Create `code/digimon-engine/tests/dsl/phase4_raw_rust_registry.rs`:

```rust
use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::effect::Effect;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn empty_registry_reports_missing_functions() {
    let registry = EngineRawRustRegistry::new();
    assert!(registry.step_fn("missing").is_none());
    assert!(registry.declarative_fn("missing").is_none());
    assert!(registry.formula_fn("missing").is_none());
    assert_eq!(registry.registered_fn_count(), 0);
}

#[test]
fn registry_registers_all_three_raw_rust_shapes() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("mark_step", |_ctx, bindings| {
        bindings.insert_literal("called", 1);
    });
    registry.register_declarative("emit_clause", |card: CardHandle| {
        vec![Effect::on_play(card).name("raw clause").process(|_| {}).build()]
    });
    registry.register_formula("formula_value", |_ctx, _target: PermanentHandle| 7);

    assert!(registry.step_fn("mark_step").is_some());
    assert!(registry.declarative_fn("emit_clause").is_some());
    assert!(registry.formula_fn("formula_value").is_some());
    assert_eq!(registry.registered_fn_count(), 3);
}

#[test]
fn registry_debug_prints_counts_without_closure_values() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("noop", |_ctx, _bindings| {});
    let text = format!("{registry:?}");
    assert!(text.contains("steps"));
    assert!(text.contains("1"));
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase4_raw_rust_registry;
```

- [ ] **Step 2: Run the failing test**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_registry -- --nocapture
```

Expected: FAIL because `digimon_engine::dsl_cards::raw_rust` does not exist.

- [ ] **Step 3: Implement the registry**

Create `code/digimon-engine/src/dsl_cards/raw_rust.rs`:

```rust
//! Engine-side raw_rust dispatch registry for DSL long-tail functions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::effect::Effect;
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub type RawStepFn =
    Arc<dyn for<'a> Fn(&mut EffectContext<'a>, &mut Bindings) + Send + Sync + 'static>;
pub type RawDeclarativeFn =
    Arc<dyn Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static>;
pub type RawFormulaFn =
    Arc<dyn for<'a> Fn(&EffectContext<'a>, PermanentHandle) -> i32 + Send + Sync + 'static>;

#[derive(Default)]
pub struct EngineRawRustRegistry {
    steps: HashMap<String, RawStepFn>,
    declaratives: HashMap<String, RawDeclarativeFn>,
    formulas: HashMap<String, RawFormulaFn>,
}

impl EngineRawRustRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_step<F>(&mut self, name: &str, f: F)
    where
        F: for<'a> Fn(&mut EffectContext<'a>, &mut Bindings) + Send + Sync + 'static,
    {
        self.steps.insert(name.to_string(), Arc::new(f));
    }

    pub fn register_declarative<F>(&mut self, name: &str, f: F)
    where
        F: Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static,
    {
        self.declaratives.insert(name.to_string(), Arc::new(f));
    }

    pub fn register_formula<F>(&mut self, name: &str, f: F)
    where
        F: for<'a> Fn(&EffectContext<'a>, PermanentHandle) -> i32 + Send + Sync + 'static,
    {
        self.formulas.insert(name.to_string(), Arc::new(f));
    }

    pub fn step_fn(&self, name: &str) -> Option<RawStepFn> {
        self.steps.get(name).cloned()
    }

    pub fn declarative_fn(&self, name: &str) -> Option<RawDeclarativeFn> {
        self.declaratives.get(name).cloned()
    }

    pub fn formula_fn(&self, name: &str) -> Option<RawFormulaFn> {
        self.formulas.get(name).cloned()
    }

    pub fn contains_fn(&self, name: &str) -> bool {
        self.steps.contains_key(name)
            || self.declaratives.contains_key(name)
            || self.formulas.contains_key(name)
    }

    pub fn registered_fn_count(&self) -> usize {
        self.steps.len() + self.declaratives.len() + self.formulas.len()
    }
}

impl std::fmt::Debug for EngineRawRustRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineRawRustRegistry")
            .field("steps", &self.steps.len())
            .field("declaratives", &self.declaratives.len())
            .field("formulas", &self.formulas.len())
            .finish()
    }
}

impl digimon_dsl::raw_rust_registry::RawRustRegistry for EngineRawRustRegistry {
    fn contains_fn(&self, name: &str) -> bool {
        self.contains_fn(name)
    }
}
```

Add this line near the other `pub mod` entries in `code/digimon-engine/src/dsl_cards/mod.rs`:

```rust
pub mod raw_rust;
```

- [ ] **Step 4: Run the registry test**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_registry -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/raw_rust.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/tests/dsl/phase4_raw_rust_registry.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: add engine raw_rust registry"
```

---

## Task 2: Thread Registry Through DSL Card Effects

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Modify: `code/digimon-engine/src/cards.rs`
- Create: `code/digimon-engine/src/cards/raw_rust/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_clause.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing `DslCardEffect` storage test**

Create `code/digimon-engine/tests/dsl/phase4_raw_rust_clause.rs`:

```rust
use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::EffectTiming;

fn raw_card(fn_name: &str) -> CompiledCard {
    CompiledCard {
        card: "RAW-1".into(),
        name: "Raw Card".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(3),
        dp: Some(2000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            fn_name: fn_name.into(),
            triggers: vec![],
            scope: CompiledScope::FaceUp,
            summary: None,
            summary_key: None,
        })],
    }
}

#[test]
fn dsl_card_effect_keeps_raw_registry() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_declarative("emit_on_play", |card: CardHandle| {
        vec![Effect::on_play(card).name("raw emit").process(|_| {}).build()]
    });
    let registry = Arc::new(registry);
    let effect = DslCardEffect::with_raw_registry(Arc::new(raw_card("emit_on_play")), registry);
    assert!(effect.raw_registry().is_some());
}

#[test]
fn raw_rust_declarative_clause_extends_effect_list() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_declarative("emit_on_play", |card: CardHandle| {
        vec![Effect::on_play(card).name("raw emit").process(|_| {}).build()]
    });
    let effect = DslCardEffect::with_raw_registry(
        Arc::new(raw_card("emit_on_play")),
        Arc::new(registry),
    );

    let effects = effect.effects(CardHandle(42));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::OnPlay);
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase4_raw_rust_clause;
```

- [ ] **Step 2: Run the failing test**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_clause -- --nocapture
```

Expected: FAIL because `DslCardEffect::with_raw_registry` and `raw_registry` do not exist.

- [ ] **Step 3: Add registry storage and whole-clause dispatch**

In `code/digimon-engine/src/dsl_cards/mod.rs`, update the struct and impl:

```rust
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;

pub struct DslCardEffect {
    compiled: Arc<CompiledCard>,
    raw: Option<Arc<EngineRawRustRegistry>>,
}

impl DslCardEffect {
    pub fn new(compiled: Arc<CompiledCard>) -> Self {
        Self { compiled, raw: None }
    }

    pub fn with_raw_registry(
        compiled: Arc<CompiledCard>,
        raw: Arc<EngineRawRustRegistry>,
    ) -> Self {
        Self {
            compiled,
            raw: Some(raw),
        }
    }

    pub fn raw_registry(&self) -> Option<Arc<EngineRawRustRegistry>> {
        self.raw.clone()
    }
}
```

In the `CompiledDeclarativeClause` match, replace the catch-all skip for raw_rust with:

```rust
CompiledDeclarativeClause::RawRust { fn_name, .. } => {
    if let Some(raw) = &self.raw {
        if let Some(f) = raw.declarative_fn(fn_name) {
            out.extend(f(card));
        }
    }
}
```

Add a new registration helper below `register_dsl_cards`:

```rust
pub fn register_dsl_cards_with_raw(
    effect_registry: &mut CardEffectRegistry,
    dsl_registry: &DslCardRegistry,
    raw: Arc<EngineRawRustRegistry>,
) {
    for (card_id, compiled) in dsl_registry.iter() {
        let dsl_effect = Arc::new(DslCardEffect::with_raw_registry(
            Arc::new(compiled.clone()),
            raw.clone(),
        ));
        effect_registry.insert(card_id, dsl_effect);
    }
}
```

- [ ] **Step 4: Add the production raw_rust registration module**

Create `code/digimon-engine/src/cards/raw_rust/mod.rs`:

```rust
//! Production raw_rust functions for DSL long-tail cards.
//!
//! Every function registered here must have a focused unit test before any
//! YAML card references it. Keep names stable; YAML packs validate against
//! these exact strings.

use crate::dsl_cards::raw_rust::EngineRawRustRegistry;

pub fn register_all(registry: &mut EngineRawRustRegistry) {
    registry.register_step("phase4_noop_step", |_ctx, _bindings| {});
    registry.register_declarative("phase4_noop_clause", |_card| Vec::new());
    registry.register_formula("phase4_formula_seven", |_ctx, _target| 7);
}
```

In `code/digimon-engine/src/cards.rs`, add:

```rust
pub mod raw_rust;
```

Then replace DSL registration inside `build_registry()` with:

```rust
    #[cfg(feature = "dsl-yaml-loader")]
    {
        let mut raw = crate::dsl_cards::raw_rust::EngineRawRustRegistry::new();
        raw_rust::register_all(&mut raw);
        let raw = Arc::new(raw);

        match crate::dsl_registry::from_embedded() {
            Ok(pack) => crate::dsl_cards::register_dsl_cards_with_raw(
                &mut registry,
                &pack,
                raw,
            ),
            Err(e) => eprintln!("DSL embedded pack failed to load: {e}"),
        }
    }
```

- [ ] **Step 5: Run the clause test**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_clause -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/src/cards.rs code/digimon-engine/src/cards/raw_rust/mod.rs code/digimon-engine/tests/dsl/phase4_raw_rust_clause.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: thread raw_rust registry through DSL card effects"
```

---

## Task 3: Step-Level RawRust Dispatch Through Recursive Step Runtime

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/control_flow.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/iteration.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs`
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_step.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing step dispatch test**

Create `code/digimon-engine/tests/dsl/phase4_raw_rust_step.rs`:

```rust
use std::sync::Arc;

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use digimon_engine::effect_context::EffectContext;

#[test]
fn raw_rust_step_can_mutate_bindings() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("mark_called", |_ctx, bindings| {
        bindings.insert_literal("called", 99);
    });
    let runtime = StepRuntime::new(Some(Arc::new(registry)));

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-STEP", "Raw Step"))
        .hand(0, &["RAW-STEP"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    let steps = vec![CompiledStep::RawRust {
        fn_name: "mark_called".into(),
        consumes: vec![],
        binds: vec!["called".into()],
    }];

    run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime);
    assert_eq!(bindings.get_literal("called"), Some(99));
}

#[test]
fn raw_rust_step_inside_if_keeps_runtime() {
    use digimon_dsl::compiled::CompiledPredicate;

    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("inner", |_ctx, bindings| bindings.insert_literal("inner", 1));
    let runtime = StepRuntime::new(Some(Arc::new(registry)));

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-STEP", "Raw Step"))
        .hand(0, &["RAW-STEP"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    let steps = vec![CompiledStep::If {
        condition: CompiledPredicate::default(),
        then: vec![CompiledStep::RawRust {
            fn_name: "inner".into(),
            consumes: vec![],
            binds: vec!["inner".into()],
        }],
        else_branch: vec![],
    }];

    run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime);
    assert_eq!(bindings.get_literal("inner"), Some(1));
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase4_raw_rust_step;
```

- [ ] **Step 2: Run the failing step tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_step -- --nocapture
```

Expected: FAIL because `StepRuntime` and `run_steps_with_runtime` do not exist.

- [ ] **Step 3: Add `StepRuntime` and raw step dispatch**

In `code/digimon-engine/src/dsl_cards/step/mod.rs`, add:

```rust
use std::sync::Arc;

use crate::dsl_cards::raw_rust::EngineRawRustRegistry;

#[derive(Clone, Default)]
pub struct StepRuntime {
    raw: Option<Arc<EngineRawRustRegistry>>,
}

impl StepRuntime {
    pub fn new(raw: Option<Arc<EngineRawRustRegistry>>) -> Self {
        Self { raw }
    }

    pub fn raw(&self) -> Option<&EngineRawRustRegistry> {
        self.raw.as_deref()
    }
}
```

Keep the old public function as a wrapper:

```rust
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> RunOutcome {
    run_steps_with_runtime(steps, ctx, bindings, &StepRuntime::default())
}
```

Move the existing body into:

```rust
pub fn run_steps_with_runtime(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> RunOutcome {
    // Existing run_steps body, but pass `runtime` into control_flow,
    // as_selecting_player, iteration, selections callback install, and run_step.
}
```

Change `run_step` to:

```rust
pub fn run_step(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) {
    if let CompiledStep::RawRust { fn_name, .. } = step {
        if let Some(raw) = runtime.raw() {
            if let Some(f) = raw.step_fn(fn_name) {
                f(ctx, bindings);
            }
        }
        return;
    }

    // Existing synchronous families stay below this guard.
}
```

- [ ] **Step 4: Thread runtime through recursive handlers**

Update handler signatures:

```rust
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> Option<RunOutcome>
```

Inside `control_flow.rs`, replace nested `run_steps(...)` calls with:

```rust
run_steps_with_runtime(body, ctx, bindings, runtime)
```

Inside `iteration.rs`, replace nested `run_steps(...)` calls with:

```rust
run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime)
```

Inside `as_selecting_player.rs`, replace nested `run_steps(...)` calls with:

```rust
run_steps_with_runtime(body, ctx, bindings, runtime)
```

In `lower_triggered.rs`, capture the registry:

```rust
let runtime = crate::dsl_cards::step::StepRuntime::new(raw.clone());
builder = builder.process(move |ctx| {
    let mut bindings = Bindings::new();
    run_steps_with_runtime(process_steps.as_slice(), ctx, &mut bindings, &runtime);
});
```

Then change `lower_triggered::lower` to accept:

```rust
pub fn lower(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Option<Arc<EngineRawRustRegistry>>,
) -> Vec<Effect>
```

And update `DslCardEffect::effects()`:

```rust
CompiledClause::Triggered(clause) => {
    out.extend(lower_triggered::lower(card, clause, self.raw.clone()));
}
```

- [ ] **Step 5: Run the step tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_step -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run DSL regression tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/step/mod.rs code/digimon-engine/src/dsl_cards/step/control_flow.rs code/digimon-engine/src/dsl_cards/step/iteration.rs code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs code/digimon-engine/src/dsl_cards/lower_triggered.rs code/digimon-engine/tests/dsl/phase4_raw_rust_step.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: dispatch raw_rust process steps through StepRuntime"
```

---

## Task 4: Preserve RawRust Runtime Through Scheduled Effects

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs`
- Modify: `code/digimon-engine/src/scheduled_effects.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_end_to_end.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing scheduled raw-rust test**

Create `code/digimon-engine/tests/dsl/phase4_raw_rust_end_to_end.rs`:

```rust
use std::sync::Arc;

use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;

#[test]
fn scheduled_body_preserves_raw_runtime() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("gain_three", |ctx, _bindings| {
        ctx.gain_memory(3);
    });
    let runtime = StepRuntime::new(Some(Arc::new(registry)));

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-SCHEDULE", "Raw Schedule"))
        .hand(0, &["RAW-SCHEDULE"])
        .memory(0)
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    let steps = vec![CompiledStep::ScheduleDelayed {
        when: CompiledTiming::EndOfYourTurn,
        body: vec![CompiledStep::RawRust {
            fn_name: "gain_three".into(),
            consumes: vec![],
            binds: vec![],
        }],
    }];

    run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourTurn,
    );
    assert_eq!(runner.game.memory, 3);
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase4_raw_rust_end_to_end;
```

- [ ] **Step 2: Run the failing scheduled test**

Run:

```powershell
cargo test -p digimon-engine --test dsl scheduled_body_preserves_raw_runtime -- --nocapture
```

Expected: FAIL because `ScheduledEffect` does not capture `StepRuntime`.

- [ ] **Step 3: Add runtime capture to scheduled effects**

In `code/digimon-engine/src/scheduled_effects.rs`, add a field:

```rust
pub runtime: crate::dsl_cards::step::StepRuntime,
```

When firing, replace:

```rust
let _outcome = run_steps(&body, &mut ctx, &mut bindings);
```

with:

```rust
let _outcome = crate::dsl_cards::step::run_steps_with_runtime(
    &body,
    &mut ctx,
    &mut bindings,
    &runtime,
);
```

In `code/digimon-engine/src/effect_context/mod.rs`, keep the existing helper and add a runtime-aware variant:

```rust
pub fn schedule_delayed_with_runtime(
    &mut self,
    when: EffectTiming,
    body: Vec<digimon_dsl::compiled::CompiledStep>,
    bindings: crate::dsl_cards::bindings::Bindings,
    runtime: crate::dsl_cards::step::StepRuntime,
) {
    self.game.scheduled_effects.push(ScheduledEffect {
        when,
        body,
        source_card: self.source_card,
        source_permanent: self.source_permanent,
        controller: self.player,
        captured_bindings: bindings,
        runtime,
    });
}
```

Make the old `schedule_delayed` call `schedule_delayed_with_runtime(..., StepRuntime::default())`.

In `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs`, accept `runtime: &StepRuntime` and call:

```rust
ctx.schedule_delayed_with_runtime(t, body.clone(), bindings.clone(), runtime.clone());
```

- [ ] **Step 4: Run the scheduled test**

Run:

```powershell
cargo test -p digimon-engine --test dsl scheduled_body_preserves_raw_runtime -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs code/digimon-engine/src/scheduled_effects.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/tests/dsl/phase4_raw_rust_end_to_end.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: preserve raw_rust runtime through scheduled effects"
```

---

## Task 5: Formula-Level RawRust Dispatch

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/modifiers.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs`
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_formula.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing formula test**

Create `code/digimon-engine/tests/dsl/phase4_raw_rust_formula.rs`:

```rust
use std::sync::Arc;

use digimon_dsl::compiled::CompiledFormula;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::effect_context::EffectContext;

#[test]
fn raw_rust_formula_uses_registered_value() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_formula("stack_plus_five", |_ctx, _target| 12);
    let registry = Arc::new(registry);

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-FORMULA", "Raw Formula"))
        .field(0, &["RAW-FORMULA"])
        .build();
    let target = runner.game.player(0).battle_area[0].handle(0);
    let source = runner.game.player(0).battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, source, Some(target), 0);

    let got = formula_eval::evaluate_with_raw(
        &CompiledFormula::RawRust("stack_plus_five".into()),
        &ctx,
        target,
        Some(registry.as_ref()),
    );
    assert_eq!(got, 12);
}

#[test]
fn missing_raw_rust_formula_returns_zero() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-FORMULA", "Raw Formula"))
        .field(0, &["RAW-FORMULA"])
        .build();
    let target = runner.game.player(0).battle_area[0].handle(0);
    let source = runner.game.player(0).battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, source, Some(target), 0);

    let got = formula_eval::evaluate_with_raw(
        &CompiledFormula::RawRust("missing".into()),
        &ctx,
        target,
        None,
    );
    assert_eq!(got, 0);
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase4_raw_rust_formula;
```

- [ ] **Step 2: Run the failing formula test**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_formula -- --nocapture
```

Expected: FAIL because `evaluate_with_raw` does not exist.

- [ ] **Step 3: Add raw-aware formula evaluation**

In `code/digimon-engine/src/dsl_cards/formula_eval.rs`, keep `evaluate` as a wrapper:

```rust
pub fn evaluate(
    f: &CompiledFormula,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
) -> i32 {
    evaluate_with_raw(f, ctx, target, None)
}
```

Add:

```rust
pub fn evaluate_with_raw(
    f: &CompiledFormula,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    raw: Option<&crate::dsl_cards::raw_rust::EngineRawRustRegistry>,
) -> i32 {
    match f {
        CompiledFormula::RawRust(name) => raw
            .and_then(|registry| registry.formula_fn(name))
            .map(|f| f(ctx, target))
            .unwrap_or(0),
        CompiledFormula::FloorDiv(args) => {
            if args.len() != 2 {
                return 0;
            }
            let l = evaluate_with_raw(&args[0], ctx, target, raw);
            let r = evaluate_with_raw(&args[1], ctx, target, raw);
            if r == 0 { 0 } else { l.div_euclid(r) }
        }
        CompiledFormula::Max(args) => args
            .iter()
            .map(|a| evaluate_with_raw(a, ctx, target, raw))
            .max()
            .unwrap_or(0),
        CompiledFormula::Min(args) => args
            .iter()
            .map(|a| evaluate_with_raw(a, ctx, target, raw))
            .min()
            .unwrap_or(0),
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            base + evaluate_per(*per, ctx, target) * delta
        }
        CompiledFormula::Aggregate(sel) => evaluate_aggregate(*sel, ctx),
    }
}
```

Update modifier and cost-reduction callers that evaluate formulas from DSL card effects to pass the raw registry through the same `StepRuntime` / `DslCardEffect` path. The concrete end state:

```rust
formula_eval::evaluate_with_raw(formula, ctx, target, runtime.raw())
```

For cost-reduction clauses, change `lower_cost_reduction::lower(...)` to accept `raw: Option<Arc<EngineRawRustRegistry>>` and call `evaluate_with_raw(..., raw.as_deref())` wherever `amount_fn` is evaluated.

- [ ] **Step 4: Run formula tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_formula -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run modifier and cost-reduction regressions**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase2f2_modifier_formula phase1c_lowering -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/src/dsl_cards/step/modifiers.rs code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs code/digimon-engine/tests/dsl/phase4_raw_rust_formula.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: dispatch raw_rust formulas through engine registry"
```

---

## Task 6: Long-Tail Raw Function Pattern and Budget Signal

**Files:**
- Modify: `code/digimon-engine/src/cards/raw_rust/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase4_raw_rust_budget.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`
- Modify: `code/digimon-engine/src/cards.rs`

- [ ] **Step 1: Write the failing budget test**

Create `code/digimon-engine/tests/dsl/phase4_raw_rust_budget.rs`:

```rust
use digimon_engine::cards::raw_rust::raw_rust_budget_status;

#[test]
fn raw_rust_budget_allows_three_percent_or_less() {
    assert!(raw_rust_budget_status(3, 100).is_ok());
    assert!(raw_rust_budget_status(0, 0).is_ok());
}

#[test]
fn raw_rust_budget_flags_above_three_percent() {
    let err = raw_rust_budget_status(4, 100).unwrap_err();
    assert!(err.contains("4 raw_rust"));
    assert!(err.contains("4.0%"));
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase4_raw_rust_budget;
```

- [ ] **Step 2: Implement the budget helper**

In `code/digimon-engine/src/cards/raw_rust/mod.rs`, add:

```rust
pub fn raw_rust_budget_status(raw_fn_count: usize, dsl_card_count: usize) -> Result<(), String> {
    if dsl_card_count == 0 {
        return Ok(());
    }
    let pct = (raw_fn_count as f64 / dsl_card_count as f64) * 100.0;
    if pct > 3.0 {
        Err(format!(
            "raw_rust budget exceeded: {raw_fn_count} raw_rust fns for {dsl_card_count} DSL cards ({pct:.1}%)"
        ))
    } else {
        Ok(())
    }
}
```

In `code/digimon-engine/src/cards.rs`, after loading `pack` and before registration, log a warning:

```rust
let raw_count = raw.registered_fn_count();
let dsl_count = pack.len();
if let Err(msg) = raw_rust::raw_rust_budget_status(raw_count, dsl_count) {
    eprintln!("WARNING: {msg}");
}
```

- [ ] **Step 3: Run the budget test**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase4_raw_rust_budget -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add code/digimon-engine/src/cards/raw_rust/mod.rs code/digimon-engine/src/cards.rs code/digimon-engine/tests/dsl/phase4_raw_rust_budget.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: report raw_rust budget during registry load"
```

---

## Task 7: Long-Tail Migration Report

**Files:**
- Create: `code/tools/dsl_long_tail_report.py`
- Create: `code/digimon-engine/tests/dsl/phase4_retirement_guard.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [x] **Step 1: Write the migration report script**

Create `code/tools/dsl_long_tail_report.py`:

```python
from __future__ import annotations

import argparse
from pathlib import Path


ALLOWED_SRC_CARDS_CHILDREN = {"test", "tokens", "raw_rust"}
ALLOWED_SRC_CARDS_FILES = {"keyword_effects.rs", "mod.rs"}


def production_rust_cards(src_cards: Path) -> list[Path]:
    found: list[Path] = []
    for path in src_cards.rglob("*.rs"):
        rel = path.relative_to(src_cards)
        if len(rel.parts) == 1 and rel.name in ALLOWED_SRC_CARDS_FILES:
            continue
        if rel.parts[0] in ALLOWED_SRC_CARDS_CHILDREN:
            continue
        found.append(path)
    return sorted(found)


def yaml_cards(cards_dir: Path) -> set[str]:
    return {p.stem.upper() for p in cards_dir.rglob("*.yaml") if "_examples" not in p.parts}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", default="code/digimon-engine")
    args = parser.parse_args()

    engine = Path(args.engine)
    src_cards = engine / "src" / "cards"
    cards_dir = engine / "cards"
    rust = production_rust_cards(src_cards)
    yaml_ids = yaml_cards(cards_dir)

    print("# DSL long-tail report")
    print(f"production_rust_card_modules={len(rust)}")
    print(f"yaml_card_files={len(yaml_ids)}")
    for path in rust:
        print(path.as_posix())

    return 1 if rust else 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [x] **Step 2: Run the report**

Run:

```powershell
python code\tools\dsl_long_tail_report.py --engine code\digimon-engine
```

Expected during migration: nonzero exit while production modules remain. Expected at Phase 4 close: `production_rust_card_modules=0`.

- [x] **Step 3: Write the final retirement guard**

Create `code/digimon-engine/tests/dsl/phase4_retirement_guard.rs`:

```rust
use std::path::PathBuf;

const ALLOWED_DIRS: &[&str] = &["test", "tokens", "raw_rust"];
const ALLOWED_FILES: &[&str] = &["keyword_effects.rs", "mod.rs"];

#[test]
fn src_cards_contains_only_test_tokens_keyword_and_raw_rust_shells() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("cards");
    let mut offenders = Vec::new();
    for entry in walkdir_like(&root) {
        let rel = entry.strip_prefix(&root).unwrap();
        if rel.components().count() == 1
            && ALLOWED_FILES.contains(&rel.file_name().unwrap().to_str().unwrap())
        {
            continue;
        }
        let first = rel.components().next().unwrap().as_os_str().to_str().unwrap();
        if ALLOWED_DIRS.contains(&first) {
            continue;
        }
        offenders.push(rel.display().to_string());
    }
    assert!(
        offenders.is_empty(),
        "production hand-written card modules must migrate to DSL YAML or cards/raw_rust:\n{}",
        offenders.join("\n")
    );
}

fn walkdir_like(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                stack.push(entry);
            } else if entry.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(entry);
            }
        }
    }
    out
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs` only after the report shows zero production modules:

```rust
mod phase4_retirement_guard;
```

- [x] **Step 4: Migrate each remaining production Rust module**

For each path printed by the report:

1. Create or update the matching YAML under `code/digimon-engine/cards/<set>/<CARD-ID>.yaml`.
2. If the card is pure DSL after Phase 3, express it entirely as YAML.
3. If the card requires bespoke logic, move only that bespoke logic into a named function in `code/digimon-engine/src/cards/raw_rust/mod.rs` and reference it from YAML with `kind: raw_rust`, `raw_rust` process step, or `raw_rust` formula.
4. Add at least one card-level behavioural test before deleting the old module.
5. Remove the old module and its `mod` export.

Run after each card batch:

```powershell
python code\tools\dsl_long_tail_report.py --engine code\digimon-engine
cargo test -p digimon-engine --test dsl -- --nocapture
cargo test -p digimon-engine --test cards_behavioral -- --nocapture
```

Expected: report count decreases; DSL and card behavioural tests pass.

- [ ] **Step 5: Commit**

```powershell
git add code/tools/dsl_long_tail_report.py code/digimon-engine/tests/dsl/phase4_retirement_guard.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/cards code/digimon-engine/src/cards
git commit -m "dsl: retire hand-written production card modules"
```

---

## Task 8: Phase 4 Closeout Docs and Exit Suite

**Files:**
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`
- Modify: `docs/RUST_PYTHON_PARITY.md`

- [x] **Step 1: Update the DSL spec status**

In `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`, under §7.5, append:

```markdown
**Phase 4 status (2026-04-26):** LANDED. `EngineRawRustRegistry`
supports whole-clause, process-step, and formula-level raw_rust dispatch.
The residual hand-written production card surface now lives under
`code/digimon-engine/src/cards/raw_rust/`; `src/cards/` no longer contains
production set modules outside the raw_rust shell.
```

If the implemented formula signature uses `(&EffectContext, PermanentHandle)` instead of the older §6 sketch `(&EffectReadContext)`, add a short note in §6.2:

```markdown
Implementation note: formula raw_rust receives `(&EffectContext,
PermanentHandle)` so it can share the same target-resolution point as
`formula_eval::evaluate_with_raw`. Functions that only need read-only state
must treat the context as read-only.
```

- [x] **Step 2: Update parity docs**

In `docs/RUST_PYTHON_PARITY.md`, add a short DSL migration note near the current DSL status section:

```markdown
As of Phase 4, DSL-authored cards can delegate residual mechanics through
engine-registered raw_rust functions at clause, process-step, and formula
granularity. Production hand-written card modules are retired from
`code/digimon-engine/src/cards/` except `test/`, `tokens/`,
`keyword_effects.rs`, and `raw_rust/`.
```

- [x] **Step 3: Run the exit suite**

Run:

```powershell
cargo test -p digimon-engine --test dsl -- --nocapture
cargo test -p digimon-engine --test cards_behavioral -- --nocapture
cargo test -p digimon-engine --test replacements -- --nocapture
python code\tools\dsl_long_tail_report.py --engine code\digimon-engine
```

Expected: all Rust tests pass; report exits `0` and prints `production_rust_card_modules=0`.

- [ ] **Step 4: Commit**

```powershell
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md docs/RUST_PYTHON_PARITY.md
git commit -m "docs: mark card scripting DSL phase 4 landed"
```

---

## Self-Review

**Spec coverage:** §7.4 Phase 3 carry-forward reducers are covered by Task 0.5. §6 raw_rust registry is covered by Tasks 1-6. Step, whole-clause, and formula granularities are covered by Tasks 2, 3, and 5. §7.5 long-tail card-by-card triage is covered by Task 7. §7.6 retirement of `src/cards/` is enforced by the report and retirement guard in Task 7.

**Placeholder scan:** This plan intentionally uses `phase4_noop_*` only as test/demo registry entries, not as production placeholders. Every production raw-rust function added during migration must have a named card-specific function and unit/card-level test before YAML references it.

**Type consistency:** `EngineRawRustRegistry::{register_step, register_declarative, register_formula, step_fn, declarative_fn, formula_fn, contains_fn, registered_fn_count}` is used consistently. `StepRuntime::new`, `StepRuntime::raw`, `run_steps_with_runtime`, and `evaluate_with_raw` are introduced before later tasks use them.

**Residual risk:** The largest integration risk is recursive runtime threading through selection callbacks. If a Phase 4 card parks a selection inside a raw-rust-bearing delayed or nested body, add a focused regression test before continuing the migration batch.
