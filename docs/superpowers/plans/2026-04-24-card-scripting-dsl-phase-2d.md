# Card Scripting DSL — Phase 2d Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-result bindings + the iteration-family verbs (`ForEach`, `PerSelected`) and the multi-pick selection step (`SelectCountCappedMulti`) that PerSelected sugars over; close the `AddModifier` filter-target gap left by 2c; and fix the 2c-flagged `run_steps` limitation so steps that follow a control-flow body sequence correctly even when the inner body parks a selection. Together this unblocks ~150 cards whose process bodies pick a set of targets and apply the same effect to each, plus the long tail of "every X gets +Y DP" stat-pump cards.

**Architecture:** One new `BindingValue` carrier — `BindingValue::PermanentList(Vec<PermanentHandle>)` plus `BindingValue::CardList(Vec<CardHandle>)` — for multi-pick / iteration sets. `SelectCountCappedMulti` lowers to a parking selection whose callback writes the picks into `Bindings` as a `CardList` (the engine API hands back `Vec<CardHandle>`). `ForEach` is **synchronous**: it scans both players' battle areas, evaluates a `CompiledPredicate` against each `PermanentHandle` via the existing `eval_predicate`, and calls `run_steps` once per match with a per-iteration `Bindings` clone. `PerSelected` is the same loop but iterates over a previously-bound list. `AddModifier` filter-target reuses the same battle-area scan. The `run_steps` continuation fix introduces `RunOutcome { Synchronous, Parked }`: control-flow + iteration handlers propagate `Parked` upward, and `run_steps` stashes any tail that follows a parked control-flow step on `Game::dsl_outer_tail`; the next selection-callback drains and resumes it after its own tail completes.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

**Scope (strict):**
- **Multi-result binding:** `BindingValue::PermanentList(Vec<PermanentHandle>)` + `BindingValue::CardList(Vec<CardHandle>)` + typed accessors
- **`SelectCountCappedMulti` lowering:** parking selection over Hand/Trash → binds `CardList`
- **`ForEach` step:** synchronous battle-area iteration, per-iteration body via `run_steps`
- **`PerSelected` step:** read multi-binding by name, iterate body
- **`AddModifier` filter-target:** evaluate filter against every battle-area permanent, apply modifier to each match
- **`run_steps` continuation propagation fix:** `RunOutcome` + `Game::dsl_outer_tail` resumption
- **End-to-end fixture:** synthetic step list — `select_count_capped_multi(of: opponent, zone: trash, max: 2)` → `per_selected` body that calls `gain_memory` per pick

**Non-goals (Phase 2e+):**
- `ScheduleDelayed` — needs a new engine primitive (`ctx.schedule_delayed(when, body)`); engine has only the option-card `OptionState::Delayed` path. Belongs in its own engine-design plan.
- Remaining selection kinds: `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectOrderedPermutation`, `SelectEffectChoice`, `AsSelectingPlayer`
- Play / digivolve / placement steps (`PlayFromHand`, `PlayFromTrash`, `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`, `PlayToken`, `Hatch`, `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`, `TrashTopSecurity`)
- Wiring the formula evaluator into `add_modifier` `value` (literals only)
- `distinct_by` enforcement on `SelectCountCappedMulti`
- Predicate-respecting candidate filters at install time for selection steps (Phase 2b/2c precedent: accept-all filter; tighter filtering needs the wider read-context signature scheduled for Phase 2e)

---

## File structure

- Modify: `digimon-engine/src/dsl_cards/bindings.rs` (add `PermanentList` + `CardList` variants, accessors)
- Modify: `digimon-engine/src/dsl_cards/binding_ref.rs` (extend `ResolvedBinding` with list variants)
- Create: `digimon-engine/src/dsl_cards/step/permanent_scan.rs` (shared "enumerate every battle-area permanent matching predicate" helper)
- Create: `digimon-engine/src/dsl_cards/step/iteration.rs` (`ForEach`, `PerSelected`)
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs` (add `SelectCountCappedMulti`; every install fn drains `Game::dsl_outer_tail`)
- Modify: `digimon-engine/src/dsl_cards/step/modifiers.rs` (add filter-target arm to `AddModifier`)
- Modify: `digimon-engine/src/dsl_cards/step/control_flow.rs` (return `Option<RunOutcome>`)
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs` (`RunOutcome` enum, `run_steps` returns `RunOutcome`, registers iteration + permanent_scan, captures outer tail on `Parked`)
- Modify: `digimon-engine/src/game.rs` (add `dsl_outer_tail` field)
- Create: `digimon-engine/tests/dsl/phase2d_select_count_capped_multi.rs`
- Create: `digimon-engine/tests/dsl/phase2d_for_each.rs`
- Create: `digimon-engine/tests/dsl/phase2d_per_selected.rs`
- Create: `digimon-engine/tests/dsl/phase2d_add_modifier_filter.rs`
- Create: `digimon-engine/tests/dsl/phase2d_run_steps_propagation.rs`
- Create: `digimon-engine/tests/dsl/phase2d_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs` (register new test modules)

**Test fixture conventions** (verified against existing 2c tests — copy verbatim):
- Each test file constructs its own `DebugRunner` via `DebugRunner::builder()`; there is no shared `common.rs` helper module.
- Player IDs are raw `u8` (`0` for P0, `1` for P1) — no `PlayerId::P*` enum sugar.
- Card construction: `digimon_engine::debug_runner::make_test_card(card_id, card_name)`.
- Field placement: `runner.place_on_field(player_idx, card_name, /* opt cost override */ None)`.
- Hand source-card handle: `runner.game.players[player_idx].hand[0].handle()`.
- Permanent handle: `runner.perm_handle(player_idx, field_index)`.
- Pending selection inspection: `runner.game.pending_selection.as_ref()` → `pending.valid_action_ids` + `pending.selecting_player`.
- Resolve selection: `runner.game.resolve_selection(selecting_player, action_id).expect("...")`.
- `EffectContext::new(&mut runner.game, src_card, /* source_permanent */ None, ctx_player)`.
- `CompiledStep` variant shapes — these are the actual definitions in `digimon-dsl/src/compiled.rs`:
  - `GainMemory(i32)` (tuple variant — NOT `{ value: ... }`)
  - `LoseMemory(i32)`
  - `Draw { of: CompiledPlayerRef, count: u8 }`
  - `AddDpModifier { target: CompiledBindingRef, value: i32, expiry: String }`
  - `AddModifier { target: CompiledModifierTarget, modifier: String, value: i32, expiry: String }` (`value` is `i32`, NOT `Option<i32>`)
  - `GrantKeyword { target, keyword: String, expiry: String, value: Option<i32> }`

---

## Task 1: Multi-result binding values

Add list-typed slots to the `Bindings` map so multi-pick selections and `ForEach` iteration variables have a place to live.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/bindings.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `mod tests` block at the bottom of `digimon-engine/src/dsl_cards/bindings.rs`:

```rust
#[test]
fn permanent_list_round_trip() {
    let mut b = Bindings::new();
    let h0 = PermanentHandle { player: 0, index: 0 };
    let h1 = PermanentHandle { player: 1, index: 3 };
    b.insert_permanent_list("targets", vec![h0, h1]);
    let got = b.get_permanent_list("targets").expect("set");
    assert_eq!(got, vec![h0, h1]);
}

#[test]
fn card_list_round_trip() {
    // Build CardHandles via a real CardSource — the existing
    // `clone_preserves_slots` test in this file already constructs Permanent /
    // Card handles inline. Mirror that exact pattern; if no Card precedent
    // exists in this file, build via `CardSource::new(...).handle()`.
    use crate::card_source::CardSource;
    let cs0 = CardSource::new();
    let cs1 = CardSource::new();
    let c0 = cs0.handle();
    let c1 = cs1.handle();

    let mut b = Bindings::new();
    b.insert_card_list("picks", vec![c0, c1]);
    let got = b.get_card_list("picks").expect("set");
    assert_eq!(got, vec![c0, c1]);
}

#[test]
fn list_clone_is_deep() {
    let mut b = Bindings::new();
    let h0 = PermanentHandle { player: 0, index: 0 };
    b.insert_permanent_list("xs", vec![h0]);
    let cloned = b.clone();
    b.insert_permanent_list("xs", vec![]); // mutate original
    assert_eq!(cloned.get_permanent_list("xs").unwrap().len(), 1);
}
```

If `CardSource::new()` doesn't expose a no-arg constructor, look at how the existing `clone_preserves_slots` test (already in this file from 2b) constructs `BindingValue::Card(...)` — there is a precedent; copy it verbatim. If no `Card` precedent exists in `bindings.rs`'s tests, look at `digimon-engine/tests/dsl/phase2c_*.rs` for `runner.game.players[0].hand[0].handle()` and use that route by setting up a tiny `DebugRunner` inline (the unit-test cost is tolerable).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::bindings`
Expected: FAIL — `insert_permanent_list` / `insert_card_list` / `get_permanent_list` / `get_card_list` do not exist; `BindingValue::PermanentList` / `CardList` variants do not exist.

- [ ] **Step 3: Implement the variants and accessors**

Replace the `BindingValue` enum in `digimon-engine/src/dsl_cards/bindings.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingValue {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(u16),
    TrashIndex(u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
}
```

Removing `Copy` is required because `Vec` is not `Copy`. The compiler will flag `bindings.rs::Bindings::get`'s use of `.copied()`. Replace it:

```rust
pub fn get(&self, name: &str) -> Option<BindingValue> {
    self.slots.get(name).cloned()
}
```

Append the new accessors to the `impl Bindings` block (alongside the existing `insert_permanent` / `insert_card` / etc.):

```rust
pub fn insert_permanent_list(&mut self, name: &str, list: Vec<PermanentHandle>) {
    self.insert(name, BindingValue::PermanentList(list));
}

pub fn insert_card_list(&mut self, name: &str, list: Vec<CardHandle>) {
    self.insert(name, BindingValue::CardList(list));
}

pub fn get_permanent_list(&self, name: &str) -> Option<Vec<PermanentHandle>> {
    match self.get(name)? {
        BindingValue::PermanentList(v) => Some(v),
        _ => None,
    }
}

pub fn get_card_list(&self, name: &str) -> Option<Vec<CardHandle>> {
    match self.get(name)? {
        BindingValue::CardList(v) => Some(v),
        _ => None,
    }
}
```

The other typed getters (`get_permanent`, `get_card`, `get_hand_index`, `get_trash_index`, `get_literal`) currently destructure with `.copied()` semantics implicitly. Switch their bodies to the same `cloned()`-friendly shape (they already match the `BindingValue::X(h) => Some(h)` pattern, which works fine after the `Copy` removal because all five non-list variants still wrap `Copy` payloads — `PermanentHandle`, `CardHandle`, `u16`, `i64` are all `Copy`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::bindings`
Expected: PASS — all `bindings` unit tests including the three new ones.

Then verify the broader engine still compiles after dropping `Copy`:

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: SUCCESS. The compiler may flag `binding_ref.rs::resolve_named` (Task 2 fixes this) — if so, the build error is the trigger for Task 2's first step. Patch `binding_ref.rs` minimally to compile (replace `bindings.get(name)?` arms that take `BindingValue` by `Copy` to take by move/clone), then re-run the build and tests until green.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/bindings.rs digimon-engine/src/dsl_cards/binding_ref.rs
git commit -m "dsl phase 2d: Bindings — PermanentList + CardList variants"
```

---

## Task 2: Resolve list-typed binding refs

`ResolvedBinding` needs new variants so step handlers can pull a list value out of a `CompiledBindingRef::Named(...)`.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/binding_ref.rs`

- [ ] **Step 1: Write the failing test**

Append a `#[cfg(test)] mod tests` block to `digimon-engine/src/dsl_cards/binding_ref.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl_cards::bindings::Bindings;
    use crate::permanent::PermanentHandle;

    #[test]
    fn resolves_permanent_list() {
        let mut b = Bindings::new();
        let h = PermanentHandle { player: 0, index: 0 };
        b.insert_permanent_list("xs", vec![h]);

        let r = resolve_named("xs", &b).expect("named binding");
        match r {
            ResolvedBinding::PermanentList(v) => assert_eq!(v, vec![h]),
            other => panic!("expected PermanentList, got {other:?}"),
        }
    }
}
```

If `resolve_named` is module-private, mark it `pub(crate) fn resolve_named(...)` so the test can reach it. (The test could go through the public `resolve_binding_ref` instead, but that requires constructing a full `EffectContext` — `resolve_named` is the right unit boundary.)

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::binding_ref`
Expected: FAIL — `ResolvedBinding::PermanentList` does not exist.

- [ ] **Step 3: Implement**

Update `digimon-engine/src/dsl_cards/binding_ref.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBinding {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(u16),
    TrashIndex(u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
}
```

(Drop `Copy` from the derive list. Add `Eq` if it wasn't already there.)

Extend `resolve_named`:

```rust
pub(crate) fn resolve_named(name: &str, bindings: &Bindings) -> Option<ResolvedBinding> {
    match bindings.get(name)? {
        BindingValue::Permanent(h) => Some(ResolvedBinding::Permanent(h)),
        BindingValue::Card(h) => Some(ResolvedBinding::Card(h)),
        BindingValue::HandIndex(i) => Some(ResolvedBinding::HandIndex(i)),
        BindingValue::TrashIndex(i) => Some(ResolvedBinding::TrashIndex(i)),
        BindingValue::Literal(v) => Some(ResolvedBinding::Literal(v)),
        BindingValue::PermanentList(v) => Some(ResolvedBinding::PermanentList(v)),
        BindingValue::CardList(v) => Some(ResolvedBinding::CardList(v)),
    }
}
```

Downstream callers of `resolve_binding_ref` in `step/zone_moves.rs`, `step/permanent_mutations.rs`, and `step/modifiers.rs` use `if let Some(ResolvedBinding::Permanent(h)) = ...` — that pattern still works after the `Copy` removal because `PermanentHandle` itself is still `Copy` and the destructure binds it by value. No edits required at those sites unless the compiler complains.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::binding_ref`
Expected: PASS.

Run the full DSL integration suite to confirm no regression:

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/binding_ref.rs
git commit -m "dsl phase 2d: ResolvedBinding — PermanentList + CardList variants"
```

---

## Task 3: Battle-area predicate-scan helper

Both `ForEach` and `AddModifier` filter-target need to enumerate all battle-area permanents matching a `CompiledPredicate`. Centralise it.

**Files:**
- Create: `digimon-engine/src/dsl_cards/step/permanent_scan.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs` (add `pub mod permanent_scan;`)

- [ ] **Step 1: Write the helper + smoke test**

Create `digimon-engine/src/dsl_cards/step/permanent_scan.rs`:

```rust
//! Enumerate battle-area permanents matching a `CompiledPredicate`.
//! Used by `ForEach` and `AddModifier { target: Filter(...) }`.
//! `PerSelected` does NOT go through this helper — it iterates a
//! pre-bound `PermanentList` / `CardList` directly (the binding was
//! produced by an earlier `select_count_capped_multi`).
//!
//! Iteration order: P0's battle_area in ascending index, then P1's. Stable
//! and turn-independent (callers that need turn-relative order should
//! re-sort).

use digimon_dsl::compiled::CompiledPredicate;

use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub fn scan(ctx: &EffectContext<'_>, pred: &CompiledPredicate) -> Vec<PermanentHandle> {
    let rctx = ctx.as_read();
    let mut out = Vec::new();
    for player_idx in 0..ctx.game.players.len() {
        let player = &ctx.game.players[player_idx];
        for (i, _perm) in player.battle_area.iter().enumerate() {
            let h = PermanentHandle { player: player_idx as u8, index: i as u8 };
            if eval_predicate(pred, &rctx, PredicateSubject::Permanent(h)) {
                out.push(h);
            }
        }
    }
    out
}
```

(No standalone unit test in this task — the helper is a thin wrapper around `eval_predicate`. Coverage comes via Tasks 4 and 8 which exercise it through `ForEach` and `AddModifier` filter-target.)

Register the module in `digimon-engine/src/dsl_cards/step/mod.rs` next to the other `pub mod` lines:

```rust
pub mod permanent_scan;
```

- [ ] **Step 2: Confirm it compiles**

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: SUCCESS.

If `ctx.game.players` is private or named differently (the field is publicly visible in the existing 2c tests via `runner.game.players[0]`, so this should hold), check the actual field name in `digimon-engine/src/game.rs` and adjust.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/permanent_scan.rs digimon-engine/src/dsl_cards/step/mod.rs
git commit -m "dsl phase 2d: permanent_scan helper (battle-area predicate enumeration)"
```

---

## Task 4: `ForEach` synchronous step

Iterate over the battle-area scan result; run the body once per match with the iteration variable bound.

**Files:**
- Create: `digimon-engine/src/dsl_cards/step/iteration.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs` (`RunOutcome` enum + `pub mod iteration;` + dispatch)
- Create: `digimon-engine/tests/dsl/phase2d_for_each.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add `RunOutcome` enum to `step/mod.rs`**

This enum is referenced by Tasks 4, 7, and the iteration handler. Land it now, before Task 4's test, so subsequent tasks compile against the final shape.

In `digimon-engine/src/dsl_cards/step/mod.rs`, just above the `run_steps` definition:

```rust
/// Whether a step ran synchronously to completion or installed a parked
/// selection. Task 7 propagates this outward across nested `run_steps`
/// re-entries so a parked selection inside an `If` / `ForEach` body
/// suspends the outer slice instead of letting subsequent outer steps
/// race ahead of the resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Synchronous,
    Parked,
}
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2d_for_each.rs`:

```rust
//! Phase 2d Task 4: ForEach iterates over a battle-area predicate scan
//! and runs the body per match.

use digimon_dsl::compiled::{CompiledCardKind, CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn for_each_runs_body_per_battle_area_match() {
    // Two test digimon on P0's field. ForEach { kind: digimon }: gain_memory(1).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .hand(0, &["SRC", "D1", "D2"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(0, "D2", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 2);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::ForEach {
        over: pred,
        bind_as: "tgt".to_string(),
        body: vec![CompiledStep::GainMemory(1)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "ForEach should have run gain_memory(1) once per matching permanent (2 of them)"
    );
}
```

Wire into `digimon-engine/tests/dsl/main.rs` (append to the bottom of the existing `mod` list):

```rust
mod phase2d_for_each;
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_for_each`
Expected: FAIL — `CompiledStep::ForEach` is not yet handled by the dispatcher; memory stays unchanged.

- [ ] **Step 4: Implement**

Create `digimon-engine/src/dsl_cards/step/iteration.rs`:

```rust
//! Iteration steps (Phase 2d): ForEach + PerSelected.
//!
//! These live on the `run_steps` path (not `run_step`) because their
//! per-iteration bodies may park selections. The dispatcher re-enters
//! `run_steps` once per iteration and propagates a `RunOutcome::Parked`
//! upward via Task 7's continuation plumbing.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::permanent_scan::scan;
use crate::dsl_cards::step::{run_steps, RunOutcome};
use crate::effect_context::EffectContext;

/// Returns `Some(RunOutcome)` if `step` is an iteration verb. Returns
/// `None` if `step` is not an iteration verb (the caller continues
/// dispatching).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::ForEach { over, bind_as, body } => {
            let matches = scan(ctx, over);
            for handle in matches {
                let mut iter_bindings = bindings.clone();
                iter_bindings.insert_permanent(bind_as, handle);
                let outcome = run_steps(body, ctx, &mut iter_bindings);
                if matches!(outcome, RunOutcome::Parked) {
                    // v1 semantics: a parked iteration aborts remaining
                    // iterations. Faithful per-iteration resumption is
                    // a future-phase enhancement.
                    return Some(RunOutcome::Parked);
                }
            }
            Some(RunOutcome::Synchronous)
        }
        CompiledStep::PerSelected { selection, bind_as, body } => {
            let bref = CompiledBindingRef::Named(selection.clone());
            match resolve_binding_ref(&bref, ctx, bindings) {
                Some(ResolvedBinding::PermanentList(v)) => {
                    for h in v {
                        let mut iter_bindings = bindings.clone();
                        iter_bindings.insert_permanent(bind_as, h);
                        if matches!(run_steps(body, ctx, &mut iter_bindings), RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                Some(ResolvedBinding::CardList(v)) => {
                    for c in v {
                        let mut iter_bindings = bindings.clone();
                        iter_bindings.insert_card(bind_as, c);
                        if matches!(run_steps(body, ctx, &mut iter_bindings), RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                _ => {} // Missing or wrong-typed binding → silent no-op (2b/2c convention).
            }
            Some(RunOutcome::Synchronous)
        }
        _ => None,
    }
}
```

Register the module in `digimon-engine/src/dsl_cards/step/mod.rs` and convert `run_steps` to return `RunOutcome` (the Task 7 final shape — landing it now keeps the next four tasks consistent):

```rust
pub mod iteration;

// ... (resolve_player + map_stack_position unchanged) ...

pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> RunOutcome {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];

        // Control flow: dispatched via dedicated handler (Task 7 makes it
        // return Option<RunOutcome>). Task 4 wires the iteration handler
        // here too with the same shape.
        if let Some(outcome) = control_flow::try_run(step, ctx, bindings) {
            if matches!(outcome, RunOutcome::Parked) {
                // Task 7: capture outer tail and resume after inner park.
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }
        if let Some(outcome) = iteration::try_run(step, ctx, bindings) {
            if matches!(outcome, RunOutcome::Parked) {
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        // Selection steps install the remainder as their callback and return.
        if selections::try_install(step, &steps[i + 1..], ctx, bindings.clone()) {
            return RunOutcome::Parked;
        }

        // Synchronous families.
        run_step(step, ctx, bindings);
        i += 1;
    }
    RunOutcome::Synchronous
}
```

`control_flow::try_run` currently returns `bool`. Convert it to `Option<RunOutcome>` in this same step (it's only a four-line body, see Task 7 for the exact rewrite — paste that version now to keep the dispatcher consistent). Without the conversion `run_steps` won't compile.

The `RunOutcome` returned by `run_steps` is currently consumed by no caller (the existing `lower_triggered.rs` etc. discard it). That stays compatible; Task 7 wires the outer-tail capture.

Update existing call-sites in `digimon-engine/src/dsl_cards/lower_triggered.rs` (and any other `run_steps` caller) to ignore the return: `let _ = run_steps(...);` if needed. The compiler will surface every call-site requiring a tweak.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_for_each`
Expected: PASS.

Then full regression:
Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/iteration.rs digimon-engine/src/dsl_cards/step/mod.rs digimon-engine/src/dsl_cards/step/control_flow.rs digimon-engine/src/dsl_cards/lower_triggered.rs digimon-engine/tests/dsl/phase2d_for_each.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2d: ForEach + iteration dispatcher (synchronous; RunOutcome scaffolding)"
```

---

## Task 5: `SelectCountCappedMulti` lowering

PerSelected needs a producer of multi-pick bindings. Wire `select_count_capped_multi` to install the parking selection and bind the result list.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2d_select_count_capped_multi.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase2d_select_count_capped_multi.rs`:

```rust
//! Phase 2d Task 5: SelectCountCappedMulti installs a parking selection,
//! its callback writes the picks into Bindings as CardList, and a
//! follow-on PerSelected step iterates the picks.

use digimon_dsl::compiled::{
    CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn multi_pick_binds_card_list_for_per_selected() {
    // P0 owns the effect. P1 has 3 cards in trash; we pick up to 2 and
    // gain 1 memory per pick.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("T1", "T1"))
        .add_card(make_test_card("T2", "T2"))
        .add_card(make_test_card("T3", "T3"))
        .hand(0, &["SRC"])
        .trash(1, &["T1", "T2", "T3"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            max: 2,
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 2".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::PerSelected {
            selection: "picks".to_string(),
            bind_as: "p".to_string(),
            body: vec![CompiledStep::GainMemory(1)],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // The select_count_capped_multi step is multi-action (each pick is a
    // separate action_id, plus a "done" submit). Mirror the resolution
    // pattern from phase2c_end_to_end.rs: read pending_selection,
    // resolve_selection() once per pick, then again with the submit
    // action_id.
    //
    // The exact submit-action-id encoding is owned by the engine; copy
    // the pattern from existing tests under `digimon-engine/tests/` that
    // already exercise count-capped multi (search for
    // `select_count_capped_multi`). Do NOT invent a new resolution
    // ceremony.
    resolve_count_capped_picks(&mut runner, &[0, 1]);

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "PerSelected over the 2 picks should have run gain_memory(1) twice"
    );
}

/// Resolve a `SelectCountCappedMulti` by picking the action IDs at each
/// of the supplied positions in the candidate list, then submitting.
///
/// IMPLEMENTATION: this helper must mirror the existing engine test
/// pattern. Find the closest precedent under
/// `digimon-engine/tests/` (grep for `select_count_capped_multi`) and
/// inline the same loop here. Likely shape:
///
///   1. For each pick:
///      - read `runner.game.pending_selection.as_ref().unwrap()`
///      - the candidate at offset N is `pending.valid_action_ids[N]`
///      - call `runner.game.resolve_selection(selecting_player, that_id)`
///   2. After all picks: re-read pending_selection (the engine re-arms
///      itself with the running accum + a submit action), then submit.
///
/// If no engine test exists yet that exercises multi-pick resolution
/// from outside, the resolution mechanics are documented inline in
/// `digimon-engine/src/effect_context/selections.rs` near
/// `install_count_capped_step`. Read that fn before writing this helper.
fn resolve_count_capped_picks(_runner: &mut DebugRunner, _picks: &[usize]) {
    // Implementation per the comment above. Failure mode if mis-copied:
    // pending_selection stays installed at end of test, or memory delta
    // is wrong. Both surface immediately in the assertion.
    todo!("see comment — copy from existing engine test pattern");
}
```

The `todo!()` placeholder is intentional: the helper must be implemented to match whatever resolution-loop pattern is already used by the engine for `select_count_capped_multi`. **The agent executing this task MUST grep the engine codebase before writing the helper** — search in this order:
1. `grep -rn "select_count_capped_multi" digimon-engine/tests/` — find an existing integration test that exercises the API end-to-end.
2. `grep -rn "install_count_capped_step\|count_capped" digimon-engine/src/` — locate the install fn and read its callback structure to understand the action-id encoding.
3. If no existing test exercises full resolution, read the source comments in `digimon-engine/src/effect_context/selections.rs::install_count_capped_step` (it documents the action-id ranges).

If after that research the helper still cannot be implemented correctly, **stop and surface the gap** rather than guessing — multi-pick resolution requires understanding `accum` semantics that this plan does not re-document.

Wire into `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2d_select_count_capped_multi;
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_select_count_capped_multi`
Expected: FAIL — `SelectCountCappedMulti` is not yet handled by `selections::try_install` (the parking selection never installs and `PerSelected` finds no binding) AND/OR the `todo!()` panic if the test reaches the resolve helper. Either failure mode is acceptable for this step.

- [ ] **Step 3: Implement the lowering**

Add to the `match step` block in `digimon-engine/src/dsl_cards/step/selections.rs::try_install`:

```rust
CompiledStep::SelectCountCappedMulti {
    of, zone, max, bind_as, prompt, optional_zero, ..
} => {
    install_select_count_capped_multi(
        ctx,
        *of,
        *zone,
        *max,
        bind_as.clone(),
        prompt.clone(),
        *optional_zero,
        tail.to_vec(),
        bindings,
    );
    true
}
```

Add the install function at the bottom of the file:

```rust
fn install_select_count_capped_multi(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zone: digimon_dsl::compiled::CompiledZone,
    max: u8,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    use crate::effect_context::selections::CountCappedZone;
    use digimon_dsl::compiled::CompiledZone as CZ;

    let target_player = resolve_player(ctx, of);
    let engine_zone = match zone {
        CZ::Hand => CountCappedZone::Hand,
        CZ::Trash => CountCappedZone::Trash,
        // Phase 2d scope: only Hand/Trash supported (matches the engine
        // API shape). Other zones silently no-op for now.
        _ => return,
    };
    let tail = std::sync::Arc::new(tail);
    ctx.select_count_capped_multi(
        target_player,
        engine_zone,
        max,
        &prompt,
        optional_zero,
        |_game, _card| true, // Phase 2b/2c precedent: accept-all filter.
        move |cb_ctx, picks: Vec<crate::card_source::CardHandle>| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, picks);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Task 7: drain outer-tail captured by run_steps when this
            // selection was installed inside a control-flow body.
            if let Some((outer_tail, mut outer_b)) = cb_ctx.game.dsl_outer_tail.take() {
                run_steps(&outer_tail, cb_ctx, &mut outer_b);
            }
        },
    );
}
```

If `digimon-dsl::compiled::CompiledZone` does not have variants named `Hand` and `Trash` exactly, look up the actual variant names in `digimon-dsl/src/compiled.rs` and adjust the match arms.

The `cb_ctx.game.dsl_outer_tail.take()` line references the field added in Task 7. Until Task 7 lands, that compiles only if you stub the field on `Game` first — easiest path: do Task 7 Step 3 (add the field + initializer) **before** running the build for this task. Or comment out that drain block here and add it back when Task 7 lands. The plan as written assumes the field is present.

- [ ] **Step 4: Implement the resolve helper**

Implement `resolve_count_capped_picks` in the test file by following the research path documented in Step 1's comment block. The helper structure should be a `for pick_offset in picks` loop that pulls the action ID from `pending.valid_action_ids[*pick_offset]`, calls `runner.game.resolve_selection(...)`, and finally submits.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_select_count_capped_multi`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs digimon-engine/tests/dsl/phase2d_select_count_capped_multi.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2d: SelectCountCappedMulti — parking selection binds CardList"
```

---

## Task 6: `PerSelected` end-to-end coverage

Tasks 4 + 5 implemented `PerSelected` and the multi-pick producer; this task is dedicated coverage for the combination.

**Files:**
- Create: `digimon-engine/tests/dsl/phase2d_per_selected.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the test**

Create `digimon-engine/tests/dsl/phase2d_per_selected.rs`:

```rust
//! Phase 2d Task 6: PerSelected over a CardList from
//! SelectCountCappedMulti. Pattern: "for each card you picked, gain
//! memory and draw 1".

use digimon_dsl::compiled::{
    CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn per_selected_drives_body_once_per_pick() {
    // P1 has 3 trash cards. P0 picks all 3, then per-pick gains 1 memory
    // and draws 1 card.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("T1", "T1"))
        .add_card(make_test_card("T2", "T2"))
        .add_card(make_test_card("T3", "T3"))
        .add_card(make_test_card("DRAW1", "DRAW1"))
        .add_card(make_test_card("DRAW2", "DRAW2"))
        .add_card(make_test_card("DRAW3", "DRAW3"))
        .hand(0, &["SRC"])
        .trash(1, &["T1", "T2", "T3"])
        .deck(0, &["DRAW1", "DRAW2", "DRAW3"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();

    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            max: 3,
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 3".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::PerSelected {
            selection: "picks".to_string(),
            bind_as: "p".to_string(),
            body: vec![
                CompiledStep::GainMemory(1),
                CompiledStep::Draw {
                    of: CompiledPlayerRef::You,
                    count: 1,
                },
            ],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Reuse the resolve helper from Task 5's test — copy the same
    // implementation here (test files don't share modules without
    // explicit `#[path]` imports).
    resolve_count_capped_picks(&mut runner, &[0, 1, 2]);

    assert_eq!(runner.game.memory, memory_before + 3, "3 picks → +3 memory");
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 3,
        "3 picks → +3 cards drawn"
    );
}

// ── Resolve helper (paste of Task 5's helper) ──
fn resolve_count_capped_picks(runner: &mut digimon_engine::debug_runner::DebugRunner, picks: &[usize]) {
    // Implementation mirrors phase2d_select_count_capped_multi.rs::resolve_count_capped_picks.
    // See that file for the research path.
    let _ = (runner, picks);
    todo!("see Task 5 helper — paste verbatim")
}
```

If the helper from Task 5 is small enough, prefer extracting it into a small `tests/dsl/phase2d_helpers.rs` module re-exported by `main.rs`:

```rust
// in main.rs
mod phase2d_helpers;
```

and have both `phase2d_select_count_capped_multi.rs` + `phase2d_per_selected.rs` import it via `use crate::phase2d_helpers::resolve_count_capped_picks;`. This avoids a copy-paste drift hazard.

Wire into `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2d_per_selected;
```

- [ ] **Step 2: Run the test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_per_selected`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/dsl/phase2d_per_selected.rs digimon-engine/tests/dsl/phase2d_helpers.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2d: PerSelected over CardList — end-to-end coverage"
```

---

## Task 7: `run_steps` continuation propagation

The 2c plan flagged this for 2d explicitly: today, steps that follow an `Optional` / `If` whose body parks run *immediately* on outer return — wrong if the author meant them to sequence after the inner selection resolves. Wire the `RunOutcome` shape (already added in Task 4) end-to-end and stash an outer tail on `Game` that selection callbacks drain.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/control_flow.rs` (return `Option<RunOutcome>`)
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs` (`run_steps` captures outer tail on `Parked`)
- Modify: `digimon-engine/src/game.rs` (add `dsl_outer_tail` field)
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs` (every install fn drains `dsl_outer_tail` after its inner tail)
- Create: `digimon-engine/tests/dsl/phase2d_run_steps_propagation.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase2d_run_steps_propagation.rs`:

```rust
//! Phase 2d Task 7: a select inside an `if` body must defer the steps
//! AFTER the `if` until the inner select resolves.
//!
//! Pattern: if (your_turn) { select_opponent_permanent; delete it } then
//! gain_memory(5). The +5 memory must NOT happen until after the delete.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn outer_steps_wait_for_inner_park() {
    // P0 turn player, P1 has one field permanent.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TGT", "TGT"))
        .hand(0, &["SRC"])
        .hand(1, &["TGT"])
        .build();
    runner.place_on_field(1, "TGT", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::If {
            condition: CompiledPredicate {
                your_turn: Some(true),
                ..CompiledPredicate::default()
            },
            then: vec![
                CompiledStep::SelectOpponentPermanent {
                    filter: CompiledPredicate::default(),
                    bind_as: Some("tgt".to_string()),
                    prompt: "Pick".to_string(),
                    prompt_key: None,
                    optional: false,
                },
                CompiledStep::DeletePermanent {
                    target: CompiledBindingRef::Named("tgt".to_string()),
                },
            ],
            else_branch: vec![],
        },
        CompiledStep::GainMemory(5),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // BEFORE the player resolves the selection: memory unchanged, target alive.
    assert_eq!(
        runner.game.memory, memory_before,
        "gain_memory(5) must not run until inner selection resolves"
    );
    assert_eq!(runner.game.players[1].battle_area.len(), 1, "target alive");

    // Resolve the field-target selection on P1's only permanent.
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("inner select_opponent_permanent should have parked");
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection should succeed");

    // AFTER resolution: target deleted AND memory bumped.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "DeletePermanent should have fired in the inner tail"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 5,
        "outer tail (gain_memory(5)) should have run after inner resolved"
    );
}
```

Wire into `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2d_run_steps_propagation;
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_run_steps_propagation`
Expected: FAIL — current `run_steps` advances past the `If` after dispatching its body, so `gain_memory(5)` runs early. The "before resolution" memory assertion (`memory == memory_before`) trips first.

- [ ] **Step 3: Add the `dsl_outer_tail` field to `Game`**

In `digimon-engine/src/game.rs`, add a field to the `Game` struct:

```rust
/// Phase 2d Task 7: when a control-flow or iteration step's body parks
/// a selection, the steps that follow the control-flow step in the
/// OUTER slice are captured here. Selection-install callbacks drain
/// this after their own tail completes, resuming the outer slice.
///
/// `None` outside of a parked control-flow continuation. Always cleared
/// at the bottom of the selection callback that drained it.
///
/// **Invariant: at most one outstanding outer continuation at a time.**
/// `run_steps` MUST `debug_assert!(self.dsl_outer_tail.is_none())` before
/// writing — overwriting a `Some` value would silently drop a parked
/// outer slice and abort the user's still-pending sequence. Today the
/// dispatcher guarantees this by never re-entering `run_steps` from
/// within a selection callback before that callback's drain runs (the
/// callback drains and then the outer slice is gone), but a future
/// change that allows nested parks (e.g. an `Optional` body whose
/// inner `If` body itself parks) will need to either (a) make this a
/// `Vec<(_, _)>` stack, or (b) refuse the second park with a clear
/// validation error. Don't silently overwrite.
pub dsl_outer_tail: Option<(
    Vec<digimon_dsl::compiled::CompiledStep>,
    crate::dsl_cards::bindings::Bindings,
)>,
```

Initialize to `None` in every `Game` constructor (`Game::new`, `Default`, any test-only builder). The compiler will list the call sites once the field is added; visit each.

- [ ] **Step 4: Convert `control_flow::try_run` to `Option<RunOutcome>`**

Edit `digimon-engine/src/dsl_cards/step/control_flow.rs`:

```rust
//! Control-flow step lowering (Phase 2c: Optional + If; Phase 2d: returns
//! Option<RunOutcome> so a parked inner body suspends the outer slice).

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::step::{run_steps, RunOutcome};
use crate::effect_context::EffectContext;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::Optional(body) => Some(run_steps(body, ctx, bindings)),
        CompiledStep::If { condition, then, else_branch } => {
            let cond_holds = {
                let rctx = ctx.as_read();
                eval_predicate(condition, &rctx, PredicateSubject::None)
            };
            let body = if cond_holds { then } else { else_branch };
            Some(run_steps(body, ctx, bindings))
        }
        _ => None,
    }
}
```

- [ ] **Step 5: Wire outer-tail capture in `run_steps`**

Edit `digimon-engine/src/dsl_cards/step/mod.rs` `run_steps` to stash the outer tail when an inner control-flow / iteration step returns `Parked`:

```rust
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> RunOutcome {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];

        if let Some(outcome) = control_flow::try_run(step, ctx, bindings) {
            if matches!(outcome, RunOutcome::Parked) {
                let outer_tail = steps[i + 1..].to_vec();
                if !outer_tail.is_empty() {
                    // Invariant: at most one outstanding outer continuation.
                    // See Game::dsl_outer_tail doc for the full rationale.
                    debug_assert!(
                        ctx.game.dsl_outer_tail.is_none(),
                        "dsl_outer_tail overwrite: an earlier outer continuation \
                         was never drained — likely a nested-park bug",
                    );
                    ctx.game.dsl_outer_tail = Some((outer_tail, bindings.clone()));
                }
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        if let Some(outcome) = iteration::try_run(step, ctx, bindings) {
            if matches!(outcome, RunOutcome::Parked) {
                let outer_tail = steps[i + 1..].to_vec();
                if !outer_tail.is_empty() {
                    debug_assert!(
                        ctx.game.dsl_outer_tail.is_none(),
                        "dsl_outer_tail overwrite: an earlier outer continuation \
                         was never drained — likely a nested-park bug",
                    );
                    ctx.game.dsl_outer_tail = Some((outer_tail, bindings.clone()));
                }
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        if selections::try_install(step, &steps[i + 1..], ctx, bindings.clone()) {
            return RunOutcome::Parked;
        }

        run_step(step, ctx, bindings);
        i += 1;
    }
    RunOutcome::Synchronous
}
```

- [ ] **Step 6: Drain `dsl_outer_tail` in every selection-install callback**

Edit `digimon-engine/src/dsl_cards/step/selections.rs`. Append a drain block to the bottom of every selection callback (4 install fns from 2b + the 1 from Task 5). Pattern, applied to `install_select_hand`:

```rust
ctx.select_hand(
    target_player,
    &prompt,
    optional,
    |_game, _idx| true,
    move |cb_ctx, idx| {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_hand_index(name, idx as u16);
        }
        run_steps(&tail, cb_ctx, &mut b);
        // Phase 2d Task 7: drain outer tail captured by run_steps when
        // this selection was installed inside a control-flow body.
        if let Some((outer_tail, mut outer_b)) = cb_ctx.game.dsl_outer_tail.take() {
            run_steps(&outer_tail, cb_ctx, &mut outer_b);
        }
    },
);
```

Apply the same drain block to `install_select_trash`, `install_select_own_permanent`, `install_select_opponent_permanent`, AND `install_select_count_capped_multi` (already drafted with the drain in Task 5).

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_run_steps_propagation`
Expected: PASS.

Then full regression — confirm no 2b/2c test relied on the wrong behaviour:

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/mod.rs digimon-engine/src/dsl_cards/step/control_flow.rs digimon-engine/src/dsl_cards/step/selections.rs digimon-engine/src/game.rs digimon-engine/tests/dsl/phase2d_run_steps_propagation.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2d: run_steps — propagate Parked, capture + drain outer tail"
```

---

## Task 8: `AddModifier` filter-target

Apply a modifier to every battle-area permanent matching a filter — closes the explicit Phase 2c gap.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/modifiers.rs`
- Create: `digimon-engine/tests/dsl/phase2d_add_modifier_filter.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase2d_add_modifier_filter.rs`:

```rust
//! Phase 2d Task 8: AddModifier with CompiledModifierTarget::Filter
//! evaluates the predicate against every battle-area permanent and
//! applies the modifier to each match.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledModifierTarget, CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::ModifierType;

#[test]
fn add_modifier_filter_targets_every_match() {
    // P0 has 2 digimon, P1 has 1. Filter: { kind: digimon }. Expect all 3
    // to receive the CannotBeAffected modifier with EndOfTurn expiry.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .add_card(make_test_card("D3", "D3"))
        .hand(0, &["SRC"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(0, "D2", None);
    runner.place_on_field(1, "D3", None);

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::AddModifier {
        target: CompiledModifierTarget::Filter(CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            ..CompiledPredicate::default()
        }),
        modifier: "CannotBeAffected".to_string(),
        value: 0,
        expiry: "EndOfTurn".to_string(),
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Each battle-area permanent should now carry CannotBeAffected.
    let mut total = 0;
    for player_idx in 0..runner.game.players.len() {
        for i in 0..runner.game.players[player_idx].battle_area.len() {
            let h = runner.perm_handle(player_idx as u8, i as u8);
            if runner.game.has_modifier(h, ModifierType::CannotBeAffected) {
                total += 1;
            }
        }
    }
    assert_eq!(total, 3, "all 3 digimon should carry CannotBeAffected");
}
```

If `runner.game.has_modifier(h, ModifierType::X)` is named differently — check `digimon-engine/src/modifiers.rs` and `phase2c_modifiers.rs` for the actual API. Likely candidates: `game.modifiers.has(h, kind)`, `game.modifier_registry.contains(...)`, or the `ModifierRegistry::active_modifiers(h, ...)` accessor. Use whatever idiom 2c's `phase2c_modifiers.rs` uses to assert modifier presence; do not invent a new accessor.

Wire into `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2d_add_modifier_filter;
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_add_modifier_filter`
Expected: FAIL — the `Filter` arm in `modifiers::try_run` currently returns early without applying anything.

- [ ] **Step 3: Implement**

Edit `digimon-engine/src/dsl_cards/step/modifiers.rs`. Replace the `AddModifier` arm:

```rust
CompiledStep::AddModifier { target, modifier, value, expiry } => {
    let Some(expiry) = lookup_expiry(expiry) else { return true; };
    let Some(modifier_ty) = crate::dsl_cards::modifier_map::lookup_modifier_type(modifier) else {
        return true;
    };
    match target {
        CompiledModifierTarget::Binding(b) => {
            if let Some(ResolvedBinding::Permanent(h)) =
                resolve_binding_ref(b, ctx, bindings)
            {
                ctx.add_modifier(h, modifier_ty, *value, expiry);
            }
        }
        CompiledModifierTarget::Filter(pred) => {
            let matches = crate::dsl_cards::step::permanent_scan::scan(ctx, pred);
            for h in matches {
                ctx.add_modifier(h, modifier_ty, *value, expiry);
            }
        }
    }
    true
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_add_modifier_filter`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/modifiers.rs digimon-engine/tests/dsl/phase2d_add_modifier_filter.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2d: AddModifier — filter-target arm (battle-area scan)"
```

---

## Task 9: End-to-end fixture

Tie the new pieces together: synthetic step list exercising multi-pick + per_selected + for_each + filter-target add_modifier in one run.

**Files:**
- Create: `digimon-engine/tests/dsl/phase2d_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the test**

Create `digimon-engine/tests/dsl/phase2d_end_to_end.rs`:

```rust
//! Phase 2d Task 9: full pipeline exercising every Phase 2d primitive in
//! a single step list.
//!
//! Step list:
//!   - select_count_capped_multi { of: opponent, zone: trash, max: 2 } → bind picks
//!   - per_selected { selection: picks, body: [draw 1, gain_memory 1] }
//!   - for_each { over: { kind: digimon, owner: you }, body: [add_dp_modifier +1000 EndOfTurn] }

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledOwnerRef, CompiledPlayerRef, CompiledPredicate,
    CompiledStep, CompiledZone,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn multi_pick_then_per_selected_then_for_each_round_trip() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .add_card(make_test_card("T1", "T1"))
        .add_card(make_test_card("T2", "T2"))
        .add_card(make_test_card("DR1", "DR1"))
        .add_card(make_test_card("DR2", "DR2"))
        .hand(0, &["SRC"])
        .trash(1, &["T1", "T2"])
        .deck(0, &["DR1", "DR2"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(0, "D2", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();

    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            max: 2,
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 2".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::PerSelected {
            selection: "picks".to_string(),
            bind_as: "p".to_string(),
            body: vec![
                CompiledStep::Draw { of: CompiledPlayerRef::You, count: 1 },
                CompiledStep::GainMemory(1),
            ],
        },
        CompiledStep::ForEach {
            over: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                owner: Some(CompiledOwnerRef::You),
                ..CompiledPredicate::default()
            },
            bind_as: "tgt".to_string(),
            body: vec![CompiledStep::AddDpModifier {
                target: CompiledBindingRef::Named("tgt".to_string()),
                value: 1000,
                expiry: "EndOfTurn".to_string(),
            }],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Resolve the multi-pick — pick both candidates.
    resolve_count_capped_picks(&mut runner, &[0, 1]);

    // 2 picks → +2 memory + 2 cards drawn.
    assert_eq!(runner.game.memory, memory_before + 2);
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 2);

    // ForEach over your digimon (D1, D2) → both should have +1000 DP this turn.
    for i in 0..runner.game.players[0].battle_area.len() {
        let h = runner.perm_handle(0, i as u8);
        let dp = runner.game.effective_dp(h).expect("digimon DP");
        let base = make_test_card("D1", "D1").dp.unwrap_or(0);
        assert_eq!(dp, base + 1000, "perm {i} should have +1000 DP from ForEach");
    }
}

// Paste of phase2d_helpers::resolve_count_capped_picks — see Task 5 / Task 6.
fn resolve_count_capped_picks(runner: &mut DebugRunner, picks: &[usize]) {
    let _ = (runner, picks);
    todo!("see Task 5 helper — paste verbatim or import from phase2d_helpers")
}
```

If `CompiledOwnerRef::You` is not the actual variant name, look it up in `digimon-dsl/src/compiled.rs` (search for `CompiledOwner` / `Owner`). The name might be `OwnerRef::You` or live as a `bool` field on the predicate (check the predicate struct definition before authoring this step).

If `make_test_card("D1", "D1").dp.unwrap_or(0)` returns the wrong base DP (i.e. `make_test_card` defaults are not what the test assumes), inspect `make_test_card` in `digimon-engine/src/debug_runner.rs` to see the default DP and use that constant directly in the assertion.

Wire into `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2d_end_to_end;
```

- [ ] **Step 2: Run the test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2d_end_to_end`
Expected: PASS — all primitives implemented by this point.

- [ ] **Step 3: Run the full regression suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/tests/dsl/phase2d_end_to_end.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2d: end-to-end — multi-pick + per_selected + for_each round-trip"
```

---

## Task 10: Parity tracker note

**Files:**
- Modify: `docs/RUST_PYTHON_PARITY.md`

- [ ] **Step 1: Update the parity tracker**

Open `docs/RUST_PYTHON_PARITY.md`. Find the chronological "Recent changes" / phase-status section (whatever the existing convention is — DO NOT introduce a new top-level section). Append:

```
- 2026-04-25 — DSL Phase 2d shipped: ForEach, PerSelected,
  SelectCountCappedMulti (Hand/Trash zones), AddModifier filter-target,
  run_steps continuation propagation (RunOutcome + Game::dsl_outer_tail).
  Defers ScheduleDelayed (needs ctx.schedule_delayed engine primitive),
  remaining selection kinds (Reveal/Security/Material/UnionZone/
  OrderedPermutation/EffectChoice/AsSelectingPlayer), play/digivolve/
  placement steps, and formula values in modifier value fields to 2e.
```

If the file has no such section, look at how prior phases (2b, 2c) were recorded — mirror that format. The git history of `docs/RUST_PYTHON_PARITY.md` will show the precedent.

- [ ] **Step 2: Commit**

```bash
git add docs/RUST_PYTHON_PARITY.md
git commit -m "dsl phase 2d: parity tracker note"
```

---

## Out-of-scope — what comes next

After 2d, the Phase 2 punch list still owes:

1. **`ScheduleDelayed`** — needs `EffectContext::schedule_delayed(when, body)` engine primitive. Today, only the Option-card flow installs `OptionState::Delayed`; an arbitrary process-body verb to schedule a one-shot delayed trigger does not exist. Belongs in its own engine-design plan — call it `2026-04-XX-rust-engine-schedule-delayed-design.md`.
2. **Remaining selection kinds** — `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectOrderedPermutation`, `SelectEffectChoice`, `AsSelectingPlayer`. Each maps 1:1 to an existing `EffectContext::select_*` API and is a near-mechanical Phase 2e slice.
3. **Play / digivolve / placement steps** — `PlayFromHand`, `PlayFromTrash`, `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`, `PlayToken`, `Hatch`, `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`, `TrashTopSecurity`. Several depend on Tier-1 engine gaps (`play_from_hand_free`, `play_from_trash_free`, `play_from_materials`, `effect_initiated_dna_digivolve`).
4. **`distinct_by` enforcement on `SelectCountCappedMulti`** — DigiXros-style "no two picks may share `card_number`". Needs the candidate-filter callback to read the running accumulator. Phase 2e.
5. **Formula values in `add_dp_modifier` / `add_modifier`** — currently literals only; wire the formula evaluator. Phase 2e.
6. **Predicate-respecting selection candidate filters** — Phase 2b/2c/2d all use accept-all install-time filters because the engine `select_*` filter signature is `Fn(&Game, ...) -> bool`, not `Fn(&EffectReadContext, ...)`. Widening the signature unlocks tighter candidate sets at install time and is a prerequisite for several real cards.

Phase 2's exit criteria (≥500 hand-written cards retired, §10 worked examples compiling, parity sweep) fire once items 2 + 3 + 5 land.
