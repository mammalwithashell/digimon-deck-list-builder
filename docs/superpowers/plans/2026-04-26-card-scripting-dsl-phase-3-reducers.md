# Card Scripting DSL Phase 3 Reducers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the Phase 3 reducer leftovers that reduce common card text to DSL verbs instead of forcing `raw_rust`.

**Architecture:** Keep the existing compiled-AOT shape: parse YAML into `digimon-dsl` specs, compile to `Compiled*` IR, and lower/run through focused `digimon-engine/src/dsl_cards/*` modules. Prefer small new step modules over growing `step/mod.rs`, and keep replacement-specific behavior behind the existing `ReplacementContext` / parked-replacement APIs.

**Tech Stack:** Rust, `digimon-dsl`, `digimon-engine`, serde YAML, existing DSL test binary `cargo test -p digimon-engine --test dsl`.

---

## Implementation Status

**Status (2026-04-26):** LANDED. The reducer batch was implemented via
parallel workers and integrated in this worktree. Coverage lives in
`phase3_reducer_replacement`, `phase3_reducer_partition`,
`phase3_reducer_selection`, `phase3_reducer_costs`, and
`phase3_dna_digivolve_triggers`.

Verified:

```powershell
cargo test -p digimon-engine --test dsl -- --nocapture
cargo test -p digimon-engine --test replacements -- --nocapture
cargo test -p digimon-engine --test selection -- --nocapture
cargo test -p digimon-engine --test cost_hooks -- --nocapture
cargo test -p digimon-engine --test cards_behavioral -- --nocapture
python code\tools\dsl_long_tail_report.py --engine code\digimon-engine
```

---

## Scope Check

The 2026-04-21 DSL spec phases 0-4 are largely landed, and Phase 4 closeout now enforces zero hand-written production `src/cards/<set>/` modules. This plan covers only the reducer leftovers called out by §7.4 and the Phase 4 carry-forward note:

- Replacement `process:` body lowering and `active_when` gating.
- Partition `active_when`, `sources`, and `exclude_cause` gates.
- Selection helpers `select_any_permanent` and `select_dna_pair`.
- Cost delta expressiveness for printed-cost reductions.
- Declarative cost-reduction `amount_fn`, `pay_cost`, and broader trigger shapes.
- DNA cost metadata ingestion hooks for DSL alt paths.

The following stay out of scope: runtime pack update channel (§7a.2), localization/event popups (§7b), DSL formatter/LSP (§9), and full card-by-card authoring.

---

## File Structure

- `code/digimon-dsl/src/step.rs` — add author-facing step verbs and serde round-trip keys.
- `code/digimon-dsl/src/compiled.rs` — add compiled IR variants for new verbs and widened cost deltas.
- `code/digimon-dsl/src/compile.rs` — compile new spec variants into IR.
- `code/digimon-dsl/src/validator.rs` — reject malformed new reducers early.
- `code/digimon-engine/src/dsl_cards/lower_replacement.rs` — lower replacement `active_when` and `process` into engine replacement effects.
- `code/digimon-engine/src/dsl_cards/lower_partition.rs` — lower partition gates into replacement conditions.
- `code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs` — support formula amounts, pay-cost bodies, and non-self cost-reduction hooks.
- `code/digimon-engine/src/dsl_cards/step/replacement_outcomes.rs` — new module for replacement-only outcome steps.
- `code/digimon-engine/src/dsl_cards/step/selections.rs` — add `select_any_permanent` and `select_dna_pair` installs.
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — lower widened `CompiledCostDelta`.
- `code/digimon-engine/src/dsl_cards/mod.rs` — pass raw registry/runtime into replacement and cost-reduction lowerers as needed.
- `code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs` — replacement body and gate regressions.
- `code/digimon-engine/tests/dsl/phase3_reducer_partition.rs` — partition gate regressions.
- `code/digimon-engine/tests/dsl/phase3_reducer_selection.rs` — new selection helper regressions.
- `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs` — cost delta, formula amount, and pay-cost regressions.
- `code/digimon-engine/tests/dsl/main.rs` — include the new test modules.
- `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — mark reducer leftovers landed after tests pass.
- `docs/RUST_PYTHON_PARITY.md` — update DSL migration note with reducer status.

---

## Task 1: Replacement Outcome Step Vocabulary

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Create: `code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add failing parse/compile tests for replacement outcome verbs**

Create `code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs`:

```rust
use digimon_engine::dsl::loader::load_card_str;
use digimon_engine::dsl::{compile_card, CardKind};
use digimon_engine::dsl::compiled::{CompiledDeclarativeClause, CompiledStep, CompiledZone};

fn compile_replacement_process(yaml_process: &str) -> Vec<CompiledStep> {
    let yaml = format!(
        r#"
card: TEST-REDUCER
name: Reducer Test
kind: digimon
level: 3
color: [red]
play_cost: 3
dp: 1000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    process:
{yaml_process}
"#
    );
    let spec = load_card_str(&yaml).expect("parse replacement YAML");
    assert_eq!(spec.kind, CardKind::Digimon);
    let compiled = compile_card(&spec).expect("compile replacement YAML");
    match &compiled.effects[0].body {
        CompiledDeclarativeClause::Replacement { process, .. } => process.clone(),
        other => panic!("expected replacement clause, got {other:?}"),
    }
}

#[test]
fn replacement_outcome_verbs_parse_and_compile() {
    let process = compile_replacement_process(
        r#"      - cancel_leave: {}
      - handle_replacement: {}
      - redirect_replacement: { destination: deck }
      - substitute_permanent: { target: source }
"#,
    );

    assert!(matches!(process[0], CompiledStep::CancelLeave));
    assert!(matches!(process[1], CompiledStep::HandleReplacement));
    assert!(matches!(
        process[2],
        CompiledStep::RedirectReplacement {
            destination: CompiledZone::Deck
        }
    ));
    assert!(matches!(process[3], CompiledStep::SubstitutePermanent { .. }));
}
```

Add to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase3_reducer_replacement;
```

- [ ] **Step 2: Run the failing test**

Run:

```powershell
cargo test -p digimon-engine --test dsl replacement_outcome_verbs_parse_and_compile -- --nocapture
```

Expected: FAIL because `cancel_leave`, `handle_replacement`, `redirect_replacement`, and `substitute_permanent` are unknown step keys.

- [ ] **Step 3: Add author-facing step structs and serde keys**

In `code/digimon-dsl/src/step.rs`, add variants to `StepSpec` after `TrashTopSource(TargetArg)`:

```rust
    CancelLeave(EmptyArgs),
    HandleReplacement(EmptyArgs),
    RedirectReplacement(RedirectReplacementArgs),
    SubstitutePermanent(TargetArg),
```

Add this struct near other small args structs:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RedirectReplacementArgs {
    pub destination: Zone,
}
```

If `EmptyArgs` does not already exist in `step.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyArgs {}
```

Add serialize arms in `impl Serialize for StepSpec`:

```rust
            StepSpec::CancelLeave(v) => kv!(s, "cancel_leave", v),
            StepSpec::HandleReplacement(v) => kv!(s, "handle_replacement", v),
            StepSpec::RedirectReplacement(v) => kv!(s, "redirect_replacement", v),
            StepSpec::SubstitutePermanent(v) => kv!(s, "substitute_permanent", v),
```

Add matching helper enum variants inside the private serde helper for `StepSpec` in the same file. Use the same Rust variant names and `snake_case` YAML keys as the serialize arms.

- [ ] **Step 4: Add compiled IR variants**

In `code/digimon-dsl/src/compiled.rs`, add variants to `CompiledStep` after `TrashTopSource`:

```rust
    CancelLeave,
    HandleReplacement,
    RedirectReplacement {
        destination: CompiledZone,
    },
    SubstitutePermanent {
        target: CompiledBindingRef,
    },
```

- [ ] **Step 5: Compile the new step variants**

In `code/digimon-dsl/src/compile.rs`, inside `compile_step`, add match arms:

```rust
        StepSpec::CancelLeave(_) => CompiledStep::CancelLeave,
        StepSpec::HandleReplacement(_) => CompiledStep::HandleReplacement,
        StepSpec::RedirectReplacement(args) => CompiledStep::RedirectReplacement {
            destination: compile_zone(args.destination),
        },
        StepSpec::SubstitutePermanent(args) => CompiledStep::SubstitutePermanent {
            target: compile_binding_ref(&args.target),
        },
```

- [ ] **Step 6: Run the parse/compile test**

Run:

```powershell
cargo test -p digimon-engine --test dsl replacement_outcome_verbs_parse_and_compile -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: add replacement outcome step verbs"
```

---

## Task 2: Replacement Process Runtime Lowering

**Files:**
- Create: `code/digimon-engine/src/dsl_cards/step/replacement_outcomes.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Test: `code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs`

- [ ] **Step 1: Add failing runtime tests**

Append to `code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs`:

```rust
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::dsl_cards::lower_replacement;
use digimon_engine::enums::{Color, EffectTiming, PlayerId, Zone};
use digimon_engine::game::Game;
use digimon_engine::permanent::Permanent;
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};

fn game_with_one_permanent() -> (Game, digimon_engine::permanent::PermanentHandle) {
    let mut game = Game::new();
    let card = CardSource::new_for_test(CardHandle(10), "TEST-REDUCER", 3, 1000, vec![Color::Red]);
    let handle = game.add_card_source(card);
    let permanent = Permanent::new(handle, PlayerId::Player1);
    game.players[0].battle_area.push(permanent);
    let permanent_handle = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    (game, permanent_handle)
}

#[test]
fn replacement_process_cancel_leave_sets_cancelled_outcome() {
    let process = compile_replacement_process("      - cancel_leave: {}\n");
    let effect = lower_replacement::lower(
        CardHandle(10),
        digimon_engine::dsl::compiled::CompiledScope::FaceUp,
        None,
        "when_would_be_deleted",
        &process,
    )
    .expect("lower replacement");

    let (mut game, target) = game_with_one_permanent();
    game.effect_registry.insert("TEST-REDUCER", std::sync::Arc::new(
        digimon_engine::dsl_cards::DslCardEffect::from_effects_for_test(vec![effect])
    ));

    let outcome = game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(target),
        ReplacementCause::OpponentEffect,
        Some(Zone::Trash),
    );

    assert_eq!(outcome, ReplacementOutcome::Cancelled);
}

#[test]
fn replacement_active_when_false_suppresses_candidate() {
    use digimon_engine::dsl::compiled::CompiledPredicate;

    let process = compile_replacement_process("      - cancel_leave: {}\n");
    let never = CompiledPredicate {
        your_turn: Some(false),
        opponents_turn: Some(false),
        ..Default::default()
    };
    let effect = lower_replacement::lower(
        CardHandle(10),
        digimon_engine::dsl::compiled::CompiledScope::FaceUp,
        Some(&never),
        "when_would_be_deleted",
        &process,
    )
    .expect("lower replacement");

    let (mut game, target) = game_with_one_permanent();
    game.effect_registry.insert("TEST-REDUCER", std::sync::Arc::new(
        digimon_engine::dsl_cards::DslCardEffect::from_effects_for_test(vec![effect])
    ));

    let outcome = game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(target),
        ReplacementCause::OpponentEffect,
        Some(Zone::Trash),
    );

    assert_eq!(outcome, ReplacementOutcome::None);
}
```

If `DslCardEffect::from_effects_for_test` does not exist, add it in this task as a `#[cfg(test)]` constructor on `DslCardEffect` that stores a fixed `Vec<Effect>` for test-only use. If the existing tests already have a test-card registry helper, use that helper instead and keep this test body equivalent.

- [ ] **Step 2: Run the failing runtime tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl replacement_process -- --nocapture
```

Expected: FAIL because `lower_replacement` still installs a no-op replacement process and ignores `active_when`.

- [ ] **Step 3: Add replacement outcome step runner**

Create `code/digimon-engine/src/dsl_cards/step/replacement_outcomes.rs`:

```rust
use digimon_dsl::compiled::{CompiledStep, CompiledZone};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;
use crate::enums::Zone;
use crate::replacement::ReplacementSubject;

fn map_zone(zone: CompiledZone) -> Zone {
    match zone {
        CompiledZone::Hand => Zone::Hand,
        CompiledZone::Trash => Zone::Trash,
        CompiledZone::Deck => Zone::Deck,
        CompiledZone::Security => Zone::Security,
        CompiledZone::BattleArea => Zone::BattleArea,
        CompiledZone::BreedingArea => Zone::BreedingArea,
        CompiledZone::Reveal => Zone::Reveal,
        CompiledZone::Source => Zone::Source,
        CompiledZone::Materials => Zone::Materials,
    }
}

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::CancelLeave => {
            ctx.cancel_leave();
            true
        }
        CompiledStep::HandleReplacement => {
            ctx.handle_replacement();
            true
        }
        CompiledStep::RedirectReplacement { destination } => {
            ctx.redirect_replacement(map_zone(*destination));
            true
        }
        CompiledStep::SubstitutePermanent { target } => {
            if let Some(ResolvedBinding::Permanent(handle)) = resolve_binding_ref(target, ctx, bindings) {
                ctx.substitute_replacement(ReplacementSubject::Permanent(handle));
            }
            true
        }
        _ => false,
    }
}
```

- [ ] **Step 4: Wire the new runner before generic mutation families**

In `code/digimon-engine/src/dsl_cards/step/mod.rs`, add:

```rust
pub mod replacement_outcomes;
```

In `run_step_with_runtime`, after raw-rust dispatch and before memory/draw:

```rust
    if replacement_outcomes::try_run(step, ctx, bindings) {
        return;
    }
```

- [ ] **Step 5: Lower replacement process bodies with runtime and active_when**

Replace `code/digimon-engine/src/dsl_cards/lower_replacement.rs` with this shape:

```rust
use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::trigger_map::lookup_replacement_trigger;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;

fn new_when_would_builder(card: CardHandle, timing: EffectTiming) -> Option<EffectBuilder> {
    match timing {
        EffectTiming::WhenWouldBeDeleted
        | EffectTiming::WhenWouldLeaveBattleArea
        | EffectTiming::WhenWouldBeReturnedToHand
        | EffectTiming::WhenWouldBeReturnedToDeck
        | EffectTiming::WhenWouldBeTrashed
        | EffectTiming::WhenWouldBeDeDigivolved
        | EffectTiming::WhenWouldLoseSecurity
        | EffectTiming::WhenWouldDraw
        | EffectTiming::WhenWouldPlaceInSecurity => Some(EffectBuilder::new(card, timing)),
        _ => None,
    }
}

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    process: &[CompiledStep],
) -> Option<Effect> {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        process,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    process: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let label = format!("Replacement: {trigger}");
    let active_when = active_when.cloned().map(Arc::new);
    let process = Arc::new(process.to_vec());
    let runtime = StepRuntime::new(raw);

    let mut builder = new_when_would_builder(card, timing)?.name(&label);
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    if let Some(predicate) = active_when.clone() {
        builder = builder.replacement_condition(move |ctx, _cause| {
            eval_predicate(&predicate, ctx, PredicateSubject::None)
        });
    }

    builder = builder.replacement_process(move |rctx| {
        let mut bindings = crate::dsl_cards::bindings::Bindings::new();
        run_steps_with_runtime(&process, rctx.effect, &mut bindings, &runtime);
    });

    Some(builder.build())
}
```

Update `code/digimon-engine/src/dsl_cards/mod.rs` so replacement clauses call `lower_replacement::lower_with_raw(..., self.raw.clone())`.

- [ ] **Step 6: Run the replacement reducer tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase3_reducer_replacement -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Run replacement regression suite**

Run:

```powershell
cargo test -p digimon-engine --test replacements -- --nocapture
```

Expected: PASS. The two `should panic` tests may print panic text and still pass.

- [ ] **Step 8: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/step/replacement_outcomes.rs code/digimon-engine/src/dsl_cards/step/mod.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs
git commit -m "dsl: lower replacement process bodies"
```

---

## Task 3: Partition Declarative Gates

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_partition.rs`
- Test: `code/digimon-engine/tests/dsl/phase3_reducer_partition.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add failing partition tests**

Create `code/digimon-engine/tests/dsl/phase3_reducer_partition.rs`:

```rust
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl::compiled::{CompiledPredicate, CompiledScope};
use digimon_engine::dsl_cards::lower_partition;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::Color;
use digimon_engine::game::Game;

#[test]
fn partition_active_when_false_does_not_grant_keyword() {
    let never = CompiledPredicate {
        your_turn: Some(false),
        opponents_turn: Some(false),
        ..Default::default()
    };
    let effect = lower_partition::lower(
        CardHandle(0),
        CompiledScope::FaceUp,
        Some(&never),
        &[],
        &[],
    );

    let mut game = Game::new();
    let source = game.add_card_source_for_test("TEST-PARTITION", 3, 1000, vec![Color::Red]);
    let permanent = game.play_card_from_hand_for_test(0, source);
    let mut ctx = EffectContext::new(&mut game, CardHandle(0), Some(permanent), 0);

    (effect.process.as_ref().expect("partition process"))(&mut ctx);

    assert!(!ctx.game.permanent_has_keyword(permanent, digimon_engine::enums::Keyword::Partition));
}
```

Add to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase3_reducer_partition;
```

- [ ] **Step 2: Run the failing test**

Run:

```powershell
cargo test -p digimon-engine --test dsl partition_active_when_false_does_not_grant_keyword -- --nocapture
```

Expected: FAIL because `lower_partition` ignores `active_when`.

- [ ] **Step 3: Gate partition process on `active_when` and source predicates**

In `code/digimon-engine/src/dsl_cards/lower_partition.rs`, import predicate helpers:

```rust
use std::sync::Arc;

use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
```

Change argument names from `_active_when`, `_sources`, `_exclude_cause` to `active_when`, `sources`, `exclude_cause`, and capture them:

```rust
    let active_when = active_when.cloned().map(Arc::new);
    let sources = Arc::new(sources.to_vec());
    let exclude_cause = Arc::new(exclude_cause.to_vec());
```

Replace the process closure body with:

```rust
        .process(move |ctx| {
            if let Some(predicate) = &active_when {
                if !eval_predicate(predicate, &ctx.as_read(), PredicateSubject::None) {
                    return;
                }
            }
            if !exclude_cause.is_empty() {
                let cause = ctx.deletion_cause().map(|c| format!("{c:?}").to_ascii_lowercase());
                if cause
                    .as_ref()
                    .is_some_and(|c| exclude_cause.iter().any(|excluded| excluded == c))
                {
                    return;
                }
            }
            let Some(handle) = ctx.source_permanent else {
                return;
            };
            if !sources.is_empty()
                && !sources
                    .iter()
                    .any(|predicate| eval_predicate(predicate, &ctx.as_read(), PredicateSubject::Permanent(handle)))
            {
                return;
            }
            ctx.grant_keyword(handle, Keyword::Partition, Expiry::Permanent);
        });
```

- [ ] **Step 4: Run partition reducer tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase3_reducer_partition -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/lower_partition.rs code/digimon-engine/tests/dsl/phase3_reducer_partition.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: honor partition declarative gates"
```

---

## Task 4: Selection Helper Reducers

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs`
- Test: `code/digimon-engine/tests/dsl/phase3_reducer_selection.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add failing parse/runtime tests**

Create `code/digimon-engine/tests/dsl/phase3_reducer_selection.rs`:

```rust
use digimon_engine::dsl::loader::load_card_str;
use digimon_engine::dsl::compile_card;
use digimon_engine::dsl::compiled::{CompiledDeclarativeClause, CompiledStep};

#[test]
fn select_any_permanent_and_select_dna_pair_compile() {
    let yaml = r#"
card: TEST-SELECT
name: Selection Reducer
kind: digimon
level: 3
color: [red]
play_cost: 3
dp: 1000
effects:
  - when: [on_play]
    process:
      - select_any_permanent:
          filter: { kind: digimon }
          bind_as: target
          prompt: Pick anything
      - select_dna_pair:
          left_filter: { kind: digimon }
          right_filter: { kind: digimon }
          bind_left_as: left
          bind_right_as: right
          prompt: Pick DNA pair
"#;
    let spec = load_card_str(yaml).expect("parse");
    let compiled = compile_card(&spec).expect("compile");
    match &compiled.effects[0].body {
        CompiledDeclarativeClause::Triggered { process, .. } => {
            assert!(matches!(process[0], CompiledStep::SelectAnyPermanent { .. }));
            assert!(matches!(process[1], CompiledStep::SelectDnaPair { .. }));
        }
        other => panic!("expected triggered clause, got {other:?}"),
    }
}
```

Add to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase3_reducer_selection;
```

- [ ] **Step 2: Run the failing selection test**

Run:

```powershell
cargo test -p digimon-engine --test dsl select_any_permanent_and_select_dna_pair_compile -- --nocapture
```

Expected: FAIL because both step keys are unknown.

- [ ] **Step 3: Add DSL and compiled variants**

In `code/digimon-dsl/src/step.rs`, add:

```rust
    SelectAnyPermanent(SelectFieldArgs),
    SelectDnaPair(SelectDnaPairArgs),
```

Add:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectDnaPairArgs {
    pub left_filter: PredicateSpec,
    pub right_filter: PredicateSpec,
    pub bind_left_as: String,
    pub bind_right_as: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
    #[serde(default)]
    pub optional: bool,
}
```

Add serialize arms:

```rust
            StepSpec::SelectAnyPermanent(v) => kv!(s, "select_any_permanent", v),
            StepSpec::SelectDnaPair(v) => kv!(s, "select_dna_pair", v),
```

In `code/digimon-dsl/src/compiled.rs`, add:

```rust
    SelectAnyPermanent {
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectDnaPair {
        left_filter: CompiledPredicate,
        right_filter: CompiledPredicate,
        bind_left_as: String,
        bind_right_as: String,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
```

In `code/digimon-dsl/src/compile.rs`, add match arms:

```rust
        StepSpec::SelectAnyPermanent(args) => CompiledStep::SelectAnyPermanent {
            filter: compile_predicate(&args.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: args.bind_as.clone(),
            prompt: args.prompt.clone(),
            prompt_key: args.prompt_key.clone(),
            optional: args.optional,
        },
        StepSpec::SelectDnaPair(args) => CompiledStep::SelectDnaPair {
            left_filter: compile_predicate(&args.left_filter, &format!("{prefix}.left_filter"), card_id, errors),
            right_filter: compile_predicate(&args.right_filter, &format!("{prefix}.right_filter"), card_id, errors),
            bind_left_as: args.bind_left_as.clone(),
            bind_right_as: args.bind_right_as.clone(),
            prompt: args.prompt.clone(),
            prompt_key: args.prompt_key.clone(),
            optional: args.optional,
        },
```

- [ ] **Step 4: Implement runtime selection installs**

In `code/digimon-engine/src/dsl_cards/step/selections.rs`, extend `try_install`:

```rust
        CompiledStep::SelectAnyPermanent {
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => install_select_any_permanent(filter, bind_as.as_deref(), prompt, *optional, tail, ctx, bindings, runtime),
        CompiledStep::SelectDnaPair {
            left_filter,
            right_filter,
            bind_left_as,
            bind_right_as,
            prompt,
            optional,
            ..
        } => install_select_dna_pair(
            left_filter,
            right_filter,
            bind_left_as,
            bind_right_as,
            prompt,
            *optional,
            tail,
            ctx,
            bindings,
            runtime,
        ),
```

Add helper implementations that reuse existing `select_permanent` plumbing:

```rust
fn install_select_any_permanent(
    filter: &CompiledPredicate,
    bind_as: Option<&str>,
    prompt: &str,
    optional: bool,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
    runtime: &StepRuntime,
) -> bool {
    install_field_selection(
        SelectionKind::Target,
        |candidate| eval_predicate(filter, &ctx.as_read(), PredicateSubject::Permanent(candidate)),
        bind_as,
        prompt,
        optional,
        tail,
        ctx,
        bindings,
        runtime,
    )
}

fn install_select_dna_pair(
    left_filter: &CompiledPredicate,
    right_filter: &CompiledPredicate,
    bind_left_as: &str,
    bind_right_as: &str,
    prompt: &str,
    optional: bool,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
    runtime: &StepRuntime,
) -> bool {
    install_select_any_permanent(
        left_filter,
        Some(bind_left_as),
        prompt,
        optional,
        &[CompiledStep::SelectAnyPermanent {
            filter: right_filter.clone(),
            bind_as: Some(bind_right_as.to_string()),
            prompt: prompt.to_string(),
            prompt_key: None,
            optional,
        }]
        .iter()
        .cloned()
        .chain(tail.iter().cloned())
        .collect::<Vec<_>>()
        .as_slice(),
        ctx,
        bindings,
        runtime,
    )
}
```

If the local `selections.rs` does not expose an `install_field_selection` helper, factor the common body out of `install_select_own_permanent` and `install_select_opponent_permanent` first, preserving their current tests.

- [ ] **Step 5: Run selection reducer tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase3_reducer_selection -- --nocapture
cargo test -p digimon-engine --test selection -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/step/selections.rs code/digimon-engine/tests/dsl/phase3_reducer_selection.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: add any-permanent and dna-pair selections"
```

---

## Task 5: Widen Cost Delta IR

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Test: `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add failing cost-delta tests**

Create `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`:

```rust
use digimon_engine::dsl::compile_card;
use digimon_engine::dsl::compiled::{CompiledCostDelta, CompiledDeclarativeClause, CompiledStep};
use digimon_engine::dsl::loader::load_card_str;

#[test]
fn cost_delta_reduce_compiles_for_play_steps() {
    let yaml = r#"
card: TEST-COST
name: Cost Reducer
kind: digimon
level: 3
color: [red]
play_cost: 3
dp: 1000
effects:
  - when: [on_play]
    process:
      - play_from_hand:
          of: you
          hand_index: picked
          cost_delta: { reduce: 2 }
"#;
    let compiled = compile_card(&load_card_str(yaml).expect("parse")).expect("compile");
    match &compiled.effects[0].body {
        CompiledDeclarativeClause::Triggered { process, .. } => match &process[0] {
            CompiledStep::PlayFromHand { cost_delta, .. } => {
                assert_eq!(*cost_delta, Some(CompiledCostDelta::Reduce(2)));
            }
            other => panic!("expected play_from_hand, got {other:?}"),
        },
        other => panic!("expected triggered clause, got {other:?}"),
    }
}
```

Add to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod phase3_reducer_costs;
```

- [ ] **Step 2: Run the failing cost-delta test**

Run:

```powershell
cargo test -p digimon-engine --test dsl cost_delta_reduce_compiles_for_play_steps -- --nocapture
```

Expected: FAIL because `{ reduce: 2 }` is not accepted by `CostDelta`.

- [ ] **Step 3: Add `reduce` to DSL and compiled IR**

In `code/digimon-dsl/src/step.rs`, add a keyword variant:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CostDelta {
    Keyword(CostDeltaKeyword),
    Literal(i32),
    Reduce { reduce: i32 },
}
```

In `code/digimon-dsl/src/compiled.rs`, add:

```rust
pub enum CompiledCostDelta {
    Free,
    Printed,
    Literal(i32),
    Reduce(i32),
}
```

In `code/digimon-dsl/src/compile.rs`, update cost-delta compile helper:

```rust
fn compile_cost_delta(delta: &crate::step::CostDelta) -> CompiledCostDelta {
    match delta {
        crate::step::CostDelta::Keyword(crate::step::CostDeltaKeyword::Free) => CompiledCostDelta::Free,
        crate::step::CostDelta::Keyword(crate::step::CostDeltaKeyword::Printed) => CompiledCostDelta::Printed,
        crate::step::CostDelta::Literal(n) => CompiledCostDelta::Literal(*n),
        crate::step::CostDelta::Reduce { reduce } => CompiledCostDelta::Reduce(*reduce),
    }
}
```

- [ ] **Step 4: Lower `CompiledCostDelta::Reduce` to engine `CostDelta::Reduce`**

In `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`, update `lower_cost_delta`:

```rust
        Some(CompiledCostDelta::Reduce(n)) => CostDelta::Reduce(*n as i16),
```

- [ ] **Step 5: Convert effect-initiated digivolve to use `cost_delta`**

Change `EffectInitiatedDigivolveArgs` and `EffectDnaDigivolveArgs` in `code/digimon-dsl/src/step.rs` from:

```rust
pub cost: i32,
```

to:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub cost_delta: Option<CostDelta>,
```

Preserve backwards compatibility by accepting legacy `cost:` in the custom compile path:

```rust
let delta = args
    .cost_delta
    .as_ref()
    .map(compile_cost_delta)
    .unwrap_or(CompiledCostDelta::Literal(args.cost.unwrap_or(0)));
```

In `CompiledStep::EffectInitiatedDigivolve` and `EffectInitiatedDnaDigivolve`, replace `cost: i32` with `cost_delta: CompiledCostDelta`. In engine lowering, call `lower_cost_delta(Some(cost_delta))` and pass the result to `effect_initiated_digivolve`; for DNA, keep the existing engine `cost: i32` call until the engine API accepts `CostDelta`, mapping `Free` to `0`, `Literal(n)` to `n`, `Reduce(n)` to `-n`, and `Printed` to the printed DNA cost metadata from Task 7.

- [ ] **Step 6: Run cost tests and play/digivolve regressions**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase3_reducer_costs -- --nocapture
cargo test -p digimon-engine --test cost_hooks -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/step/play_digivolve.rs code/digimon-engine/tests/dsl/phase3_reducer_costs.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: widen cost delta reducers"
```

---

## Task 6: Cost Reduction Formula Amounts and Pay-Cost Bodies

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs`
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Test: `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`

- [ ] **Step 1: Add failing formula amount test**

Append to `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`:

```rust
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl::compiled::{CompiledFormula, CompiledPredicate, CompiledScope};
use digimon_engine::dsl_cards::lower_cost_reduction;

#[test]
fn cost_reduction_amount_fn_uses_formula_value() {
    let effect = lower_cost_reduction::lower_with_formula(
        CardHandle(0),
        CompiledScope::FaceUp,
        None,
        None,
        false,
        Some(CompiledFormula::Literal(4)),
        vec![],
        std::sync::Arc::new(digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry::new()),
    );

    let mut game = digimon_engine::game::Game::new();
    let ctx = digimon_engine::effect_context::EffectReadContext {
        game: &game,
        source_card: CardHandle(0),
        source_permanent: None,
        player: 0,
    };

    assert_eq!((effect.cost_reduction_fn.as_ref().expect("cost fn"))(&ctx), 4);
}
```

- [ ] **Step 2: Run the failing formula amount test**

Run:

```powershell
cargo test -p digimon-engine --test dsl cost_reduction_amount_fn_uses_formula_value -- --nocapture
```

Expected: FAIL because `lower_with_formula` does not exist.

- [ ] **Step 3: Add formula-aware cost reduction lowerer**

In `code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs`, add:

```rust
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};

pub fn lower_with_formula(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    condition: Option<CompiledPredicate>,
    once_per_turn: bool,
    amount_fn: Option<digimon_dsl::compiled::CompiledFormula>,
    pay_cost: Vec<digimon_dsl::compiled::CompiledStep>,
    raw: std::sync::Arc<EngineRawRustRegistry>,
) -> Effect {
    let active_when = active_when.map(Arc::new);
    let condition = condition.map(Arc::new);
    let amount_fn = amount_fn.map(Arc::new);
    let pay_cost = Arc::new(pay_cost);
    let runtime = StepRuntime::new(raw);

    let mut builder = Effect::before_pay_cost(card).name("Cost reduction");
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if once_per_turn {
        builder = builder.once_per_turn();
    }

    builder = builder.cost_reduction_fn(move |rctx| {
        if let Some(aw) = &active_when {
            if !eval_predicate(aw, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        if let Some(c) = &condition {
            if !eval_predicate(c, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        if let Some(formula) = &amount_fn {
            if let Some(target) = rctx.source_permanent {
                formula_eval::evaluate_with_raw(formula, rctx, target, runtime.raw())
            } else {
                0
            }
        } else {
            0
        }
    });

    if !pay_cost.is_empty() {
        builder = builder.pay_cost_fn(move |ctx| {
            let mut bindings = crate::dsl_cards::bindings::Bindings::new();
            matches!(
                run_steps_with_runtime(&pay_cost, ctx, &mut bindings, &runtime),
                crate::dsl_cards::step::RunOutcome::Synchronous
            )
        });
    }

    builder.build()
}
```

Keep the existing `lower(..., amount: i32)` as a wrapper that calls `lower_with_formula(..., Some(CompiledFormula::Literal(amount)), vec![], Arc::new(EngineRawRustRegistry::new()))`.

- [ ] **Step 4: Wire declarative cost reductions to formula lowerer**

In `code/digimon-engine/src/dsl_cards/mod.rs`, when matching `CompiledDeclarativeClause::CostReduction`, emit an effect when either `amount` or `amount_fn` exists:

```rust
let amount_formula = amount_fn.clone().or_else(|| Some(CompiledFormula::Literal(*amount)));
effects.push(lower_cost_reduction::lower_with_formula(
    card,
    *scope,
    active_when.clone(),
    condition.clone(),
    *once_per_turn,
    amount_formula,
    pay_cost.clone(),
    self.raw.clone(),
));
```

- [ ] **Step 5: Run cost reducer tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase3_reducer_costs -- --nocapture
cargo test -p digimon-engine --test cost_hooks -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/tests/dsl/phase3_reducer_costs.rs
git commit -m "dsl: lower formula cost reductions"
```

---

## Task 7: DNA Cost Metadata Ingestion

**Files:**
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: `code/digimon-engine/src/dsl_bridge.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Test: `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`

- [ ] **Step 1: Add failing DNA metadata test**

Append to `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`:

```rust
#[test]
fn dna_alt_path_uses_structured_cost_metadata_when_present() {
    let yaml = r#"
card: TEST-DNA
name: DNA Reducer
kind: digimon
level: 6
color: [red, blue]
play_cost: 12
dp: 12000
alt_paths:
  - kind: dna_digivolve
    cost: printed
    materials:
      - { color: red, level: 5 }
      - { color: blue, level: 5 }
"#;
    let spec = digimon_engine::dsl::loader::load_card_str(yaml).expect("parse");
    let compiled = digimon_engine::dsl::compile_card(&spec).expect("compile");
    assert_eq!(compiled.alt_paths[0].cost, 0, "printed DNA cost should be filled by metadata before compile");
}
```

- [ ] **Step 2: Run the failing DNA metadata test**

Run:

```powershell
cargo test -p digimon-engine --test dsl dna_alt_path_uses_structured_cost_metadata_when_present -- --nocapture
```

Expected: FAIL until the loader has a structured metadata source for printed DNA costs.

- [ ] **Step 3: Add structured metadata field to card data**

In `code/digimon-engine/src/card_data.rs`, add an optional field to the card data struct:

```rust
#[serde(default)]
pub dna_digivolve_cost: Option<u8>,
```

If the existing card data struct uses camelCase or source-specific field names, add serde aliases:

```rust
#[serde(default, alias = "dnaDigivolveCost", alias = "dna_cost")]
pub dna_digivolve_cost: Option<u8>,
```

- [ ] **Step 4: Thread metadata into DSL bridge**

In `code/digimon-engine/src/dsl_bridge.rs`, when building DSL card specs from real card data, set `alt_path.cost` from `card_data.dna_digivolve_cost` for `dna_digivolve` paths whose cost is `printed`.

Use this exact behavior:

```rust
if path.kind == AltPathKind::DnaDigivolve && path.cost_is_printed() {
    if let Some(cost) = data.dna_digivolve_cost {
        path.cost = AltPathCost::Literal(cost as i32);
    }
}
```

Adapt names to the local `AltPathSpec` enum names if they differ; keep the behavior identical.

- [ ] **Step 5: Run DNA and DSL tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl phase3_reducer_costs -- --nocapture
cargo test -p digimon-engine --test dsl phase3_dna_digivolve_triggers -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/card_data.rs code/digimon-engine/src/dsl_bridge.rs code/digimon-dsl/src/compile.rs code/digimon-engine/tests/dsl/phase3_reducer_costs.rs
git commit -m "dsl: ingest dna cost metadata"
```

---

## Task 8: Exit Gate and Documentation

**Files:**
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`
- Modify: `docs/RUST_PYTHON_PARITY.md`
- Modify: `docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-4.md`

- [ ] **Step 1: Run full reducer exit suite**

Run:

```powershell
cargo fmt
cargo test -p digimon-engine --test dsl -- --nocapture
cargo test -p digimon-engine --test replacements -- --nocapture
cargo test -p digimon-engine --test selection -- --nocapture
cargo test -p digimon-engine --test cost_hooks -- --nocapture
cargo test -p digimon-engine --test cards_behavioral -- --nocapture
python code\tools\dsl_long_tail_report.py --engine code\digimon-engine
```

Expected:

```text
test result: ok
# DSL long-tail report
production_rust_card_modules=0
```

- [ ] **Step 2: Update the DSL spec reducer status**

In `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`, under §7.4, append:

```markdown
**Phase 3 reducer status (2026-04-26):** LANDED. Replacement process
bodies now lower through the shared DSL step runtime, replacement
`active_when` gates are honored, partition gates are applied, common
selection reducers (`select_any_permanent`, `select_dna_pair`) are
available, cost deltas can express printed-cost reductions, and
cost-reduction formulas/pay-cost bodies lower without `raw_rust`.
```

- [ ] **Step 3: Update parity tracker**

In `docs/RUST_PYTHON_PARITY.md`, extend the DSL migration note:

```markdown
The Phase 3 reducer leftovers are also closed: replacement bodies,
partition gates, any-permanent/DNA-pair selections, formula-backed
cost reductions, and reduce-style cost deltas now run through DSL
lowering instead of `raw_rust`.
```

- [ ] **Step 4: Update Phase 4 plan carry-forward checkboxes**

In `docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-4.md`, mark Task 0.5 Steps 1-6 complete and add:

```markdown
Reducer closeout was executed via
`docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-3-reducers.md`.
```

- [ ] **Step 5: Commit**

```powershell
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md docs/RUST_PYTHON_PARITY.md docs/superpowers/plans/2026-04-26-card-scripting-dsl-phase-4.md
git commit -m "docs: close DSL phase 3 reducer carry-forward"
```

---

## Self-Review

**Spec coverage:** §7.4 advanced clauses are covered by Tasks 1-6: replacement process lowering, partition gates, selection helpers, formula/cost reducers, and DNA cost metadata. §7.5/§7.6 remain enforced by the existing raw-rust registry and long-tail guard. §7a/§7b are intentionally out of scope and should get separate plans.

**Placeholder scan:** This plan avoids `TBD`, undefined future work steps, and generic “add tests” instructions. Each task names exact files, commands, expected failures, and expected passes.

**Type consistency:** New author-facing verbs are `cancel_leave`, `handle_replacement`, `redirect_replacement`, `substitute_permanent`, `select_any_permanent`, and `select_dna_pair`. The matching IR variants are `CancelLeave`, `HandleReplacement`, `RedirectReplacement`, `SubstitutePermanent`, `SelectAnyPermanent`, and `SelectDnaPair`. Cost delta uses `Reduce(i32)` in compiled IR and lowers to engine `CostDelta::Reduce(i16)`.

**Parallelization notes:** Tasks 1 and 5 both touch `digimon-dsl/src/step.rs`, `compiled.rs`, and `compile.rs`; do not run those two as parallel writers. Task 3 can run in parallel after Task 1 because it only touches partition lowering/tests. Task 6 depends on Task 5 for widened cost semantics and on Phase 4 raw runtime. Task 8 must run last.
