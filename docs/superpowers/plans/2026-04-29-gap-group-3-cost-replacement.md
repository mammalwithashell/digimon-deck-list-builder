# Gap Group 3 Cost and Replacement Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the cost-payment and replacement-context gaps that block cards whose printed text says "by doing X", "if an effect would", "Partition", "Delay to prevent", or "end the attack".

**Architecture:** Generalize effect activation costs so queued triggered effects can decline before process execution, then thread replacement cause/controller context through replacement predicates before exposing prevention choices.

**Tech Stack:** Rust engine in `code/digimon-engine`, Rust integration tests under `code/digimon-engine/tests`, existing `Effect`, `EffectContext`, `EffectReadContext`, `Game`, `ReplacementContext`, pending-selection, Option flow, and combat state-machine APIs.

---

## Current Baseline

This plan starts from the post-Group-2 engine state:

- `Effect::pay_cost_fn` already exists and is invoked for triggered effects in `Game::run_queued_effect_inner`.
- Existing tests in `code/digimon-engine/tests/cost_hooks/pay_cost_triggered.rs` prove synchronous triggered pay-cost behavior.
- `effect_queue.rs` still treats `pay_cost_fn` as synchronous and explicitly documents pending selections inside `pay_cost_fn` as unsupported.
- `ReplacementContext` already carries `cause`, `subject`, `original_destination`, and `outcome`.
- Replacement parking for nested selections already exists in `replacement.rs`.
- DSL replacement lowering exists in `code/digimon-engine/src/dsl_cards/lower_replacement.rs`, but its `active_when` predicates are not fully wired through replacement cause/controller context.

This plan extends the existing substrate. It does not replace the current `pay_cost_fn` field or rebuild replacement dispatch.

## File Structure

### Engine Files

- Modify: `code/digimon-engine/src/effect.rs`
  - Keep `PayCostFn = Fn(&mut EffectContext) -> bool`.
  - Update comments so selection-gated pay costs are a supported contract.
  - Add a short `.pay_cost(...)` builder alias that forwards to `.pay_cost_fn(...)`.

- Modify: `code/digimon-engine/src/game.rs`
  - Store a parked queued-effect continuation after a pay-cost hook installs a `PendingSelection`.
  - Store whether that parked pay-cost continuation was declined.

- Modify: `code/digimon-engine/src/effect_queue.rs`
  - Split queued-effect execution into pre-cost and post-cost phases.
  - Resume post-cost execution after selection resolution.
  - Preserve condition checks, max-per-turn recording, source attribution, trigger context, and queue-draining order.

- Modify: `code/digimon-engine/src/effect_context/mod.rs`
  - Add `EffectContext::decline_pending_pay_cost()`.
  - Add helper methods used by Partition, Delay prevention, and attack cancellation.

- Modify: `code/digimon-engine/src/effect_context/selections.rs`
  - Reuse existing source-selection primitives for optional pay-cost selections.
  - Add no new action IDs unless a test proves existing `SourceMulti` cannot represent the prompt.

- Modify: `code/digimon-engine/src/replacement.rs`
  - Add replacement predicate context accessors for cause, source controller, subject controller, and source-vs-subject ownership.
  - Ensure optional replacement prompts are only installed after predicates pass.

- Modify: `code/digimon-engine/src/combat.rs`
  - Add a card-effect-facing attack cancellation path that returns the attack state machine to the caller without battle or security damage.

- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
  - Lower `active_when` onto replacement predicate evaluation using replacement context.
  - Keep source-permanent self-scope behavior unless the DSL explicitly asks for observer-scoped replacement.

### Test Files

- Create: `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`
- Modify: `code/digimon-engine/tests/cost_hooks/main.rs`
- Create: `code/digimon-engine/tests/replacements/context_predicates.rs`
- Create: `code/digimon-engine/tests/replacements/partition.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`
- Modify: `code/digimon-engine/tests/option_flow/replacement_integration.rs`
- Create: `code/digimon-engine/tests/replacements/attack_cancel.rs`

### Documentation Files

- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

---

## Implementation Rules

- Keep `PayCostFn` returning `bool`; returning `false` still means the cost is not paid and `process` does not run.
- When `pay_cost_fn` returns `true` and installed `game.pending_selection`, the engine parks the queued effect and does not run `process` until the selection chain resolves.
- If a selection callback calls `ctx.decline_pending_pay_cost()` or `game.decline_pending_pay_cost()`, the parked effect is discarded and `process` does not run.
- Max-per-turn counters are recorded only when the pay cost completed and immediately before `process`, matching the current synchronous path.
- Replacement predicates must execute before optional accept/decline prompts.
- Replacement source/controller context must be visible to native effects and DSL-lowered `active_when`.
- Attack cancellation must leave no stale `pending_attack`, must not perform battle, and must not perform security checks.

---

### Task 1: Pay-Cost Selection Continuation

**Files:**
- Create: `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`
- Modify: `code/digimon-engine/tests/cost_hooks/main.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-engine/src/effect.rs`

- [ ] **Step 1: Register the new cost-hook test module**

Add this line to `code/digimon-engine/tests/cost_hooks/main.rs`:

```rust
mod pay_cost_selection;
```

- [ ] **Step 2: Write the failing mandatory-selection pay-cost test**

Create `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::selection::SelectionKind;

struct PayCostSelectsSourceThenRunsProcess {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for PayCostSelectsSourceThenRunsProcess {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("pay cost selects source, then process")
            .pay_cost(|ctx| {
                cost_log.lock().unwrap().push("pay_cost");
                ctx.select_own_sources(
                    "Trash 1 source to pay this effect cost",
                    1,
                    1,
                    |_game, source_ref| source_ref.card.card_id == "SRC",
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        ctx.game.trash_source_ref(refs[0]);
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

#[test]
fn triggered_pay_cost_source_selection_parks_process_until_selection_resolves() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(PayCostSelectsSourceThenRunsProcess { log: log.clone() }),
    );

    let host = r.place_on_field(0, "HOST", Some(0));
    r.add_source(host, "SRC");

    let memory_before = r.memory();
    r.play(0, 0);

    assert_eq!(
        *log.lock().unwrap(),
        vec!["pay_cost"],
        "process must not run before the source-selection cost resolves"
    );
    assert!(r.game.pending_pay_cost_effect.is_some());
    let pending = r.game.pending_selection.as_ref().expect("source selection installed");
    assert!(matches!(pending.kind, SelectionKind::SourceMulti { min: 1, max: 1, picked: 0 }));
    assert_eq!(
        r.memory(),
        memory_before - 3,
        "only printed play cost is paid before the queued effect process resumes"
    );

    let action = encode_source_select(0, 0);
    r.game
        .resolve_selection(0, action)
        .expect("source-selection action resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["pay_cost", "process"]);
    assert_eq!(
        r.memory(),
        memory_before,
        "play cost -3 plus process gain +3 after the pay-cost selection resolves"
    );
    assert_eq!(r.trash_size(0), 1, "the selected source was paid to trash");
}

#[test]
fn mandatory_pay_cost_selection_rejects_pass() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(PayCostSelectsSourceThenRunsProcess { log }),
    );

    let host = r.place_on_field(0, "HOST", Some(0));
    r.add_source(host, "SRC");
    r.play(0, 0);

    let err = r
        .game
        .resolve_selection(0, PASS)
        .expect_err("mandatory pay-cost source selection must reject PASS");
    assert_eq!(format!("{err:?}"), "InvalidAction");
    assert!(r.game.pending_selection.is_some());
    assert!(r.game.pending_pay_cost_effect.is_some());
}
```

The test names the new public fields and helper methods the implementation must provide:

- `Game::pending_pay_cost_effect`
- `Game::trash_source_ref(...)`
- `EffectBuilder::pay_cost(...)`

- [ ] **Step 3: Run the new test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost_selection --nocapture
```

Expected: FAIL because `pay_cost`, `pending_pay_cost_effect`, and `trash_source_ref` are not available, and because the queue currently continues into `process` immediately after a pay-cost hook installs a selection.

- [ ] **Step 4: Add the builder alias and update the effect comment**

In `code/digimon-engine/src/effect.rs`, add this method next to `pay_cost_fn`:

```rust
    /// Alias for `.pay_cost_fn(...)`. Use this name for printed triggered
    /// effects whose text says "by doing X".
    pub fn pay_cost<F>(self, f: F) -> Self
    where
        F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static,
    {
        self.pay_cost_fn(f)
    }
```

Replace the `pay_cost_fn` comment that says pending selections are unsupported with this contract:

```rust
    /// If this hook installs a pending selection and returns `true`, queued
    /// triggered effects park after the cost hook and resume `process` only
    /// after the selection chain resolves. Selection callbacks may call
    /// `decline_pending_pay_cost` to discard the parked effect before process.
```

- [ ] **Step 5: Add queued pay-cost continuation state**

In `code/digimon-engine/src/game.rs`, add this struct near other parked-state structs:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingPayCostEffect {
    pub card_id: CardId,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
    pub effect_slot: u8,
    pub trigger_context: Option<TriggerContext>,
}
```

Add these fields to `Game`:

```rust
    pub pending_pay_cost_effect: Option<PendingPayCostEffect>,
    pub pending_pay_cost_declined: bool,
```

Initialize them in `Game::new` and every test constructor path that manually constructs `Game`:

```rust
            pending_pay_cost_effect: None,
            pending_pay_cost_declined: false,
```

Add this helper in the `impl Game` block:

```rust
    pub fn decline_pending_pay_cost(&mut self) {
        self.pending_pay_cost_declined = true;
    }
```

- [ ] **Step 6: Add source-ref trash helper**

In `code/digimon-engine/src/game.rs`, add:

```rust
    pub fn trash_source_ref(&mut self, source_ref: SourceSelectionRef) -> Option<CardHandle> {
        let owner = source_ref.permanent.player;
        let idx = source_ref.permanent.index as usize;
        let source_idx = source_ref.source_index as usize;
        let card = {
            let player = self.player_mut(owner);
            let permanent = player.battle_area.get_mut(idx)?;
            if source_idx >= permanent.card_sources.len() {
                return None;
            }
            permanent.card_sources.remove(source_idx)
        };
        self.player_mut(owner).trash.push(card);
        Some(card)
    }
```

Add required imports at the top of `game.rs`:

```rust
use crate::selection::SourceSelectionRef;
```

- [ ] **Step 7: Park the queued effect when pay-cost installed a selection**

In `code/digimon-engine/src/effect_queue.rs`, add a private conversion helper near `run_queued_effect_inner`:

```rust
fn pending_pay_cost_from_queued(qe: QueuedEffect) -> PendingPayCostEffect {
    PendingPayCostEffect {
        card_id: qe.card_id,
        source_card: qe.source_card,
        source_permanent: qe.source_permanent,
        controller: qe.controller,
        effect_slot: qe.effect_slot,
        trigger_context: qe.trigger_context,
    }
}
```

After `pay_cost(&mut ctx)` returns `true`, insert this before max-per-turn handling:

```rust
            if self.pending_selection.is_some() {
                self.pending_pay_cost_declined = false;
                self.pending_pay_cost_effect = Some(pending_pay_cost_from_queued(qe));
                return;
            }
```

This parks the effect before max-per-turn recording and before `process`.

- [ ] **Step 8: Split post-cost execution into a helper**

Still in `effect_queue.rs`, extract the current max-per-turn and process block from `run_queued_effect_inner` into:

```rust
    fn run_queued_effect_after_pay_cost(&mut self, pending: PendingPayCostEffect) {
        let qe = QueuedEffect {
            card_id: pending.card_id,
            source_card: pending.source_card,
            source_permanent: pending.source_permanent,
            controller: pending.controller,
            effect_slot: pending.effect_slot,
            trigger_context: pending.trigger_context,
            security_reveal_id: None,
        };

        let prev_effect_source = self.effect_source_player;
        let prev_trigger_context = self.current_trigger_context;
        self.effect_source_player = Some(qe.controller);
        self.current_trigger_context = qe.trigger_context;

        self.run_queued_effect_process_only(qe);

        self.current_trigger_context = prev_trigger_context;
        self.effect_source_player = prev_effect_source;
    }
```

Add `run_queued_effect_process_only` by moving the existing max-per-turn and process logic into a helper that does not run condition or pay-cost again:

```rust
    fn run_queued_effect_process_only(&mut self, qe: QueuedEffect) {
        let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
            return;
        };
        let Some(effect) = effects.get(qe.effect_slot as usize) else {
            return;
        };

        if effect.max_per_turn > 0 {
            let key = crate::once_per_turn::OnceKey::for_effect(
                qe.controller,
                qe.card_id.clone(),
                qe.effect_slot,
            );
            self.once_per_turn.record(key);
        }

        if let Some(proc_fn) = &effect.process {
            let mut ctx = EffectContext::new(self, qe.source_card, qe.source_permanent, qe.controller);
            proc_fn(&mut ctx);
        }
    }
```

Update `run_queued_effect_inner` so the synchronous path calls `self.run_queued_effect_process_only(qe)` after pay-cost succeeds.

- [ ] **Step 9: Resume the parked pay-cost continuation after selections resolve**

In `resolve_generic_selection`, immediately after scheduled-effect resume and before the final `drain_effect_queue()` call, add:

```rust
        if self.pending_selection.is_none() {
            self.resume_pending_pay_cost_effect();
        }
```

Add the helper in `effect_queue.rs`:

```rust
    fn resume_pending_pay_cost_effect(&mut self) {
        let Some(pending) = self.pending_pay_cost_effect.take() else {
            return;
        };
        if self.pending_pay_cost_declined {
            self.pending_pay_cost_declined = false;
            return;
        }
        self.run_queued_effect_after_pay_cost(pending);
    }
```

- [ ] **Step 10: Run focused tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost_selection --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost_triggered --nocapture
```

Expected: PASS. The old triggered pay-cost tests must keep passing.

- [ ] **Step 11: Commit**

```bash
git add code/digimon-engine/src/effect.rs code/digimon-engine/src/game.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/tests/cost_hooks/main.rs code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs
git commit -m "feat: resume triggered pay costs after selections"
```

---

### Task 2: Optional Pay-Cost Decline Path

**Files:**
- Modify: `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`

- [ ] **Step 1: Add failing tests for optional cost decline and accept**

Append to `pay_cost_selection.rs`:

```rust
struct OptionalPayCostSourceSelection {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for OptionalPayCostSourceSelection {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let decline_log = self.log.clone();
        let accept_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("optional pay cost source selection")
            .pay_cost(move |ctx| {
                let decline_log = decline_log.clone();
                let accept_log = accept_log.clone();
                ctx.select_count_capped_multi(
                    ctx.player,
                    digimon_engine::effect_context::selections::CountCappedZone::Material(
                        ctx.source_permanent.expect("test registers this as a field trigger source"),
                    ),
                    1,
                    "You may trash 1 source to run this effect",
                    true,
                    |_game, card| card.card_id == "SRC",
                    move |ctx, picked| {
                        if picked.is_empty() {
                            decline_log.lock().unwrap().push("decline_cost");
                            ctx.decline_pending_pay_cost();
                            return;
                        }
                        accept_log.lock().unwrap().push("accept_cost");
                        for card in picked {
                            ctx.game.player_mut(ctx.player).trash.push(card);
                        }
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(2);
            })
            .build()]
    }
}

#[test]
fn optional_pay_cost_decline_skips_process() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC-HOST", "Source Host"))
        .add_card(make_test_card("SRC", "Source"))
        .memory(0)
        .start();
    r.register_effect(
        "SRC-HOST",
        Arc::new(OptionalPayCostSourceSelection { log: log.clone() }),
    );
    let host = r.place_on_field(0, "SRC-HOST", Some(0));
    r.add_source(host, "SRC");

    r.game.enqueue_triggered_for_permanent(
        digimon_engine::effect::EffectTiming::EndOfYourTurn,
        host,
        None,
    );
    r.game.drain_effect_queue();

    assert!(r.game.pending_pay_cost_effect.is_some());
    r.game.resolve_selection(0, PASS).expect("decline optional cost");

    assert_eq!(*log.lock().unwrap(), vec!["decline_cost"]);
    assert_eq!(r.memory(), 0, "process gain must not run after cost decline");
    assert!(r.game.pending_pay_cost_effect.is_none());
}
```

If `select_count_capped_multi` cannot target source material directly at this point, keep the behavior but use `select_own_sources` with `min = 0` and `max = 1`; the expected PASS behavior remains the same.

- [ ] **Step 2: Run the optional decline test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- optional_pay_cost_decline_skips_process --nocapture
```

Expected: FAIL because `EffectContext::decline_pending_pay_cost` does not exist or because the continuation resumes process after PASS.

- [ ] **Step 3: Add the effect-context decline helper**

In `code/digimon-engine/src/effect_context/mod.rs`, add:

```rust
    pub fn decline_pending_pay_cost(&mut self) {
        self.game.decline_pending_pay_cost();
    }
```

- [ ] **Step 4: Ensure the continuation helper respects decline after nested drains**

In `resume_pending_pay_cost_effect`, keep this exact order:

```rust
    fn resume_pending_pay_cost_effect(&mut self) {
        let Some(pending) = self.pending_pay_cost_effect.take() else {
            return;
        };

        let declined = self.pending_pay_cost_declined;
        self.pending_pay_cost_declined = false;

        if declined {
            return;
        }

        self.run_queued_effect_after_pay_cost(pending);
    }
```

- [ ] **Step 5: Run cost-hook regression tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/game.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs
git commit -m "feat: allow optional triggered cost decline"
```

---

### Task 3: Replacement Cause and Controller Predicates

**Files:**
- Create: `code/digimon-engine/tests/replacements/context_predicates.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/replacement.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

- [ ] **Step 1: Register the replacement predicate test module**

Add to `code/digimon-engine/tests/replacements/main.rs`:

```rust
mod context_predicates;
```

- [ ] **Step 2: Write failing native predicate tests**

Create `code/digimon-engine/tests/replacements/context_predicates.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::replacement::ReplacementCause;

struct CancelOnlyOpponentEffects(Arc<Mutex<u32>>);

impl CardEffect for CancelOnlyOpponentEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .name("cancel only opponent effects")
            .replacement_condition(|ctx, _subject| {
                ctx.replacement_cause() == Some(ReplacementCause::OpponentEffect)
                    && ctx.replacement_source_controller() != Some(ctx.player())
            })
            .replacement_process(move |rctx| {
                *count.lock().unwrap() += 1;
                rctx.cancel();
            })
            .build()]
    }
}

#[test]
fn replacement_condition_can_gate_on_opponent_effect_cause() {
    let count = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("GATE", "Gate"))
        .start();
    r.register_effect("GATE", Arc::new(CancelOnlyOpponentEffects(count.clone())));

    let gate = r.place_on_field(0, "GATE", Some(0));

    r.game
        .delete_permanent_with_cause(gate, ReplacementCause::OwnEffect);
    assert_eq!(*count.lock().unwrap(), 0);
    assert_eq!(r.battle_area_len(0), 0, "own-effect deletion is not cancelled");

    let gate = r.place_on_field(0, "GATE", Some(0));
    r.game
        .delete_permanent_with_cause(gate, ReplacementCause::OpponentEffect);
    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(r.battle_area_len(0), 1, "opponent-effect deletion is cancelled");
}
```

- [ ] **Step 3: Run the native predicate test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context_predicates --nocapture
```

Expected: FAIL because `EffectReadContext` does not expose `replacement_cause`, `replacement_source_controller`, or `player`.

- [ ] **Step 4: Add replacement context fields to read contexts**

In `effect_context/mod.rs`, extend `EffectReadContext`:

```rust
    replacement_cause: Option<ReplacementCause>,
    replacement_source_controller: Option<PlayerId>,
    replacement_subject_controller: Option<PlayerId>,
```

Add accessors:

```rust
    pub fn player(&self) -> PlayerId {
        self.player
    }

    pub fn replacement_cause(&self) -> Option<ReplacementCause> {
        self.replacement_cause
    }

    pub fn replacement_source_controller(&self) -> Option<PlayerId> {
        self.replacement_source_controller
    }

    pub fn replacement_subject_controller(&self) -> Option<PlayerId> {
        self.replacement_subject_controller
    }
```

- [ ] **Step 5: Populate replacement predicate context in the dispatcher**

In `replacement.rs`, before evaluating each `replacement_condition`, build `EffectReadContext` with:

```rust
replacement_cause: Some(ctx.cause),
replacement_source_controller: game.effect_source_player,
replacement_subject_controller: ctx.subject.controller(game),
```

Add this helper on `ReplacementSubject`:

```rust
impl ReplacementSubject {
    pub fn controller(self, game: &Game) -> Option<PlayerId> {
        match self {
            ReplacementSubject::Permanent(handle) => game.permanent(handle).map(|_| handle.player),
            ReplacementSubject::Card(card) => game.owner_of_card(card),
            ReplacementSubject::Unknown => None,
        }
    }
}
```

If `owner_of_card` is not currently available, add:

```rust
    pub fn owner_of_card(&self, card: CardHandle) -> Option<PlayerId> {
        self.players
            .iter()
            .enumerate()
            .find_map(|(idx, player)| player.contains_card(card).then_some(idx as PlayerId))
    }
```

Implement `PlayerState::contains_card` by checking hand, deck, trash, security, breeding, battle permanents, and card sources.

- [ ] **Step 6: Lower DSL replacement active_when predicates against replacement context**

In `lower_replacement.rs`, replace the current ignored-`active_when` branch with:

```rust
let active_when = replacement.active_when.clone();
builder = builder.replacement_condition(move |ctx, subject| {
    if !replacement_subject_matches_source(ctx, subject) {
        return false;
    }
    active_when
        .as_ref()
        .map(|predicate| crate::dsl_cards::predicate::eval_replacement_predicate(ctx, predicate))
        .unwrap_or(true)
});
```

Add DSL predicate evaluation support:

```rust
pub fn eval_replacement_predicate(ctx: &EffectReadContext, predicate: &CompiledPredicate) -> bool {
    match predicate {
        CompiledPredicate::ReplacementCause(cause) => ctx.replacement_cause() == Some(*cause),
        CompiledPredicate::ReplacementSourceIsOpponent => {
            ctx.replacement_source_controller().is_some()
                && ctx.replacement_source_controller() != Some(ctx.player())
        }
        CompiledPredicate::ReplacementSubjectIsMine => {
            ctx.replacement_subject_controller() == Some(ctx.player())
        }
        other => eval_predicate(ctx, other),
    }
}
```

Add the matching enum variants and YAML parser support in the DSL crate if `CompiledPredicate` is generated outside `digimon-engine`.

- [ ] **Step 7: Add a DSL fixture test using EX9-032, EX7-027, and BT22-036 semantics**

Append to `context_predicates.rs`:

```rust
#[test]
fn dsl_replacement_active_when_filters_own_and_opponent_causes() {
    // Fixture names mirror the roadmap cards:
    // EX9-032 / EX7-027 / BT22-036 need replacement effects that care
    // whether the deletion was caused by your effect, an opponent effect,
    // or battle.
    let mut r = DebugRunner::builder()
        .load_dsl_card("EX9-032")
        .load_dsl_card("EX7-027")
        .load_dsl_card("BT22-036")
        .start();

    let gate = r.place_on_field(0, "EX9-032", Some(0));

    r.game
        .delete_permanent_with_cause(gate, ReplacementCause::OwnEffect);
    assert_eq!(r.battle_area_len(0), 0, "own effect passes through active_when gate");

    let gate = r.place_on_field(0, "EX9-032", Some(0));
    r.game
        .delete_permanent_with_cause(gate, ReplacementCause::OpponentEffect);
    assert_eq!(r.battle_area_len(0), 1, "opponent effect satisfies active_when gate");
}
```

If these card YAML files are not present, add the smallest YAML fixture files under the current DSL test fixture directory with these card IDs and replacement `active_when` clauses. The test must keep the printed fixture IDs so future card implementation can replace the synthetic fixture without renaming the regression.

- [ ] **Step 8: Run replacement tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context_predicates --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- deletion_replacements --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement --nocapture
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add code/digimon-engine/src/effect.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/replacement.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/replacements/main.rs code/digimon-engine/tests/replacements/context_predicates.rs
git commit -m "feat: expose replacement context predicates"
```

---

### Task 4: Partition Source Enforcement and Selection

**Files:**
- Create: `code/digimon-engine/tests/replacements/partition.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/src/replacement.rs`

- [ ] **Step 1: Register the Partition test module**

Add to `code/digimon-engine/tests/replacements/main.rs`:

```rust
mod partition;
```

- [ ] **Step 2: Write the failing BT16-025 Partition source test**

Create `code/digimon-engine/tests/replacements/partition.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, Zone};
use digimon_engine::replacement::ReplacementCause;

fn colored_card(id: &str, color: CardColor, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![color];
    card.level = Some(level);
    card
}

struct PaildramonPartition(Arc<Mutex<u32>>);

impl CardEffect for PaildramonPartition {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .name("BT16-025 Partition")
            .replacement_condition(|ctx, _subject| {
                ctx.replacement_cause() == Some(ReplacementCause::OpponentEffect)
            })
            .replacement_process(move |rctx| {
                *count.lock().unwrap() += 1;
                rctx.effect.select_partition_sources(
                    rctx.subject.permanent().expect("Partition subject is a permanent"),
                    "Partition BT16-025",
                    vec![
                        digimon_engine::effect_context::PartitionRequirement::new(
                            "Blue Lv.4",
                            |game, source| game.card(source.card).is_color(CardColor::Blue)
                                && game.card(source.card).level == Some(4),
                        ),
                        digimon_engine::effect_context::PartitionRequirement::new(
                            "Green Lv.4",
                            |game, source| game.card(source.card).is_color(CardColor::Green)
                                && game.card(source.card).level == Some(4),
                        ),
                    ],
                    move |ctx, selected| {
                        ctx.play_selected_sources_without_cost(selected);
                        ctx.cancel_current_replacement();
                    },
                );
            })
            .build()]
    }
}

#[test]
fn bt16_025_partition_requires_one_each_matching_source() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-L4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-L4", CardColor::Green, 4))
        .add_card(colored_card("RED-L4", CardColor::Red, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition(fired.clone())));

    let host = r.place_on_field(0, "BT16-025", Some(0));
    r.add_source(host, "BLUE-L4");
    r.add_source(host, "GREEN-L4");
    r.add_source(host, "RED-L4");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert_eq!(*fired.lock().unwrap(), 1);
    assert!(r.game.pending_selection.is_some(), "Partition source prompt is exposed");

    r.game
        .resolve_selection(0, encode_source_select(0, 0))
        .expect("select blue Lv.4 source");
    r.game
        .resolve_selection(0, encode_source_select(0, 0))
        .expect("select green Lv.4 source after blue is removed from candidate list");

    assert_eq!(r.battle_area_len(0), 3, "host survives and two sources are played");
    assert_eq!(r.trash_size(0), 0, "Partition selected sources are played, not trashed");
}

#[test]
fn bt16_025_partition_decline_allows_deletion() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-L4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-L4", CardColor::Green, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition(fired)));

    let host = r.place_on_field(0, "BT16-025", Some(0));
    r.add_source(host, "BLUE-L4");
    r.add_source(host, "GREEN-L4");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    r.game.resolve_selection(0, PASS).expect("decline Partition");

    assert_eq!(r.battle_area_len(0), 0, "declining Partition allows deletion");
    assert_eq!(r.trash_size(0), 3, "host and sources go to trash");
}
```

- [ ] **Step 3: Run the Partition test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
```

Expected: FAIL because `PartitionRequirement`, `select_partition_sources`, `play_selected_sources_without_cost`, `cancel_current_replacement`, and `ReplacementSubject::permanent` do not exist.

- [ ] **Step 4: Add Partition requirement and subject helpers**

In `effect_context/mod.rs`, add:

```rust
pub struct PartitionRequirement {
    pub label: &'static str,
    pub matches: Box<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>,
}

impl PartitionRequirement {
    pub fn new<F>(label: &'static str, matches: F) -> Self
    where
        F: Fn(&Game, SourceSelectionRef) -> bool + Send + Sync + 'static,
    {
        Self {
            label,
            matches: Box::new(matches),
        }
    }
}
```

In `replacement.rs`, add:

```rust
impl ReplacementSubject {
    pub fn permanent(self) -> Option<PermanentHandle> {
        match self {
            ReplacementSubject::Permanent(handle) => Some(handle),
            _ => None,
        }
    }
}
```

- [ ] **Step 5: Add the Partition selection helper**

In `effect_context/selections.rs`, implement:

```rust
impl<'a> EffectContext<'a> {
    pub fn select_partition_sources<C>(
        &mut self,
        host: PermanentHandle,
        prompt: &str,
        requirements: Vec<PartitionRequirement>,
        callback: C,
    ) where
        C: FnOnce(&mut EffectContext<'_>, Vec<SourceSelectionRef>) + Send + Sync + 'static,
    {
        let required_count = requirements.len() as u8;
        self.select_own_sources(
            prompt,
            required_count,
            required_count,
            move |game, source_ref| {
                source_ref.permanent == host
                    && requirements
                        .iter()
                        .any(|requirement| (requirement.matches)(game, source_ref))
            },
            move |ctx, selected| {
                let all_requirements_met = requirements.iter().all(|requirement| {
                    selected
                        .iter()
                        .any(|source_ref| (requirement.matches)(ctx.game, *source_ref))
                });
                if all_requirements_met {
                    callback(ctx, selected);
                }
            },
        );
    }
}
```

If duplicate source selection can satisfy two requirements with one card, replace the final `all_requirements_met` with a bipartite matching helper over selected sources and requirement indices. Use that helper in this same task so a dual-color source cannot improperly satisfy two "1 each" slots by itself.

- [ ] **Step 6: Add source play and replacement cancel helpers**

In `effect_context/mod.rs`, add:

```rust
    pub fn play_selected_sources_without_cost(&mut self, selected: Vec<SourceSelectionRef>) {
        for source_ref in selected {
            if let Some(card) = self.game.remove_source_ref(source_ref) {
                self.game.play_card_from_effect_without_cost(self.player, card);
            }
        }
    }

    pub fn cancel_current_replacement(&mut self) {
        self.game.cancel_parked_replacement();
    }
```

Add corresponding `Game` helpers in `game.rs` or `replacement.rs`:

```rust
    pub fn remove_source_ref(&mut self, source_ref: SourceSelectionRef) -> Option<CardHandle> {
        let owner = source_ref.permanent.player;
        let idx = source_ref.permanent.index as usize;
        let source_idx = source_ref.source_index as usize;
        self.player_mut(owner)
            .battle_area
            .get_mut(idx)?
            .card_sources
            .get(source_idx)?;
        Some(
            self.player_mut(owner)
                .battle_area
                .get_mut(idx)?
                .card_sources
                .remove(source_idx),
        )
    }

    pub fn play_card_from_effect_without_cost(&mut self, player: PlayerId, card: CardHandle) {
        self.place_card_in_battle_area_from_effect(player, card, true);
    }
```

`cancel_parked_replacement` must set the currently parked replacement outcome to `ReplacementOutcome::Cancelled` and allow the parked replacement drain to complete through the existing replacement resume hook.

- [ ] **Step 7: Run Partition and nested replacement tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- nested_select --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/replacement.rs code/digimon-engine/tests/replacements/main.rs code/digimon-engine/tests/replacements/partition.rs
git commit -m "feat: add partition source replacement flow"
```

---

### Task 5: Delay-As-Replacement Prevention

**Files:**
- Modify: `code/digimon-engine/tests/option_flow/replacement_integration.rs`
- Modify: `code/digimon-engine/src/replacement.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

- [ ] **Step 1: Add the failing BT17-097 Delay prevention regression**

Append to `code/digimon-engine/tests/option_flow/replacement_integration.rs`:

```rust
#[test]
fn bt17_097_delay_prevents_deletion_and_digivolves_from_hand() {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS};

    let mut r = DebugRunner::builder()
        .load_dsl_card("BT17-097")
        .add_card(make_test_card("FREE-TARGET", "Free Target"))
        .add_card(make_test_card("IMPERIAL-HAND", "Imperial Hand"))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();

    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    r.place_delay_option(0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(r.game.pending_selection.is_some(), "Delay prevention prompt is exposed");
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("select Imperialdramon-like hand target");

    assert_eq!(r.battle_area_len(0), 2, "target survived and hand card digivolved or played");
    assert_eq!(r.trash_contains(0, "BT17-097"), true, "Delay option paid itself to trash");

    let target = r.find_battle_permanent(0, "FREE-TARGET").expect("target still present");
    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    r.game.resolve_selection(0, PASS).expect("decline when no Delay remains");
    assert!(r.find_battle_permanent(0, "FREE-TARGET").is_none());
}
```

Keep the fixture card ID `BT17-097` in the test even if the first pass uses a synthetic DSL file.

- [ ] **Step 2: Run the Delay prevention test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- bt17_097_delay_prevents_deletion_and_digivolves_from_hand --nocapture
```

Expected: FAIL because Delay-as-replacement is not lowered or does not park the replacement until the hand selection resolves.

- [ ] **Step 3: Add Delay replacement lowering**

In `lower_replacement.rs`, support a replacement process step with this shape:

```yaml
replacement:
  timing: when_would_be_deleted
  active_when:
    all:
      - subject_trait: Free
      - replacement_cause: opponent_effect
  cost:
    delay_self: true
  choose:
    from: hand
    card_filter:
      trait: Imperialdramon
    min: 1
    max: 1
  outcome:
    prevent
  then:
    - digivolve_without_cost:
        target: replacement_subject
        card: chosen
```

Lower it into:

```rust
builder = builder.replacement_process(move |rctx| {
    if !rctx.effect.trash_delay_source() {
        return;
    }

    let subject = rctx.subject;
    rctx.effect.select_hand(
        "Choose a card for BT17-097",
        1,
        1,
        |game, card| game.card(card).has_trait("Imperialdramon"),
        move |ctx, chosen| {
            let Some(card) = chosen.first().copied() else {
                ctx.game.resume_parked_replacement_unhandled();
                return;
            };
            ctx.digivolve_replacement_subject_without_cost(subject, card);
            ctx.cancel_current_replacement();
        },
    );
});
```

- [ ] **Step 4: Add native helpers used by Delay replacement**

In `effect_context/mod.rs`, add:

```rust
    pub fn trash_delay_source(&mut self) -> bool {
        let Some(source) = self.source_permanent else {
            return false;
        };
        self.game.delete_permanent_with_cause(source, ReplacementCause::Cost);
        true
    }

    pub fn digivolve_replacement_subject_without_cost(
        &mut self,
        subject: ReplacementSubject,
        card: CardHandle,
    ) {
        if let Some(target) = subject.permanent() {
            self.game.digivolve_without_cost_from_hand(self.player, target, card);
        }
    }
```

- [ ] **Step 5: Ensure no target or declined selection leaves replacement unhandled**

Add this helper in `replacement.rs`:

```rust
    pub fn resume_parked_replacement_unhandled(&mut self) {
        if let Some(parked) = self.parked_replacement.as_mut() {
            parked.outcome = ReplacementOutcome::NotReplaced;
        }
    }
```

Use it in the hand-selection decline callback so the original deletion proceeds.

- [ ] **Step 6: Run option flow regression tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- nested_select --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/replacement.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/option_flow/replacement_integration.rs
git commit -m "feat: add delay prevention replacement flow"
```

---

### Task 6: Attack Cancellation Return Path

**Files:**
- Create: `code/digimon-engine/tests/replacements/attack_cancel.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

- [ ] **Step 1: Register the attack-cancel regression module**

Add to `code/digimon-engine/tests/replacements/main.rs`:

```rust
mod attack_cancel;
```

- [ ] **Step 2: Write the failing EX10-003 attack cancellation regression**

Create `code/digimon-engine/tests/replacements/attack_cancel.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};

struct TumblemonCancelAttack(Arc<Mutex<u32>>);

impl CardEffect for TumblemonCancelAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.0.clone();
        vec![Effect::when_opponent_attacks(card)
            .name("EX10-003 Tumblemon attack cancel")
            .pay_cost(move |ctx| {
                let fired = fired.clone();
                ctx.select_own_sources(
                    "Trash 3 Mineral/Rock sources to end the attack",
                    3,
                    3,
                    |game, source_ref| {
                        game.card(source_ref.card).has_trait("Mineral")
                            || game.card(source_ref.card).has_trait("Rock")
                    },
                    move |ctx, refs| {
                        for source_ref in refs {
                            ctx.game.trash_source_ref(source_ref);
                        }
                        *fired.lock().unwrap() += 1;
                    },
                );
                true
            })
            .process(|ctx| {
                ctx.cancel_pending_attack();
            })
            .build()]
    }
}

#[test]
fn ex10_003_pay_cost_can_end_pending_attack() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("EX10-003", "Tumblemon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SRC1", "Mineral Source 1"))
        .add_card(make_test_card("SRC2", "Mineral Source 2"))
        .add_card(make_test_card("SRC3", "Rock Source 3"))
        .security(0, &["SECURITY"])
        .start();
    r.register_effect("EX10-003", Arc::new(TumblemonCancelAttack(fired.clone())));

    let blocker = r.place_on_field(0, "EX10-003", Some(0));
    r.add_source(blocker, "SRC1");
    r.add_source(blocker, "SRC2");
    r.add_source(blocker, "SRC3");
    let attacker = r.place_on_field(1, "ATTACKER", Some(0));

    r.attack_player(1, attacker);

    assert!(r.game.pending_selection.is_some(), "Tumblemon pay-cost prompt is exposed");
    r.game.resolve_selection(0, encode_source_select(0, 0)).unwrap();
    r.game.resolve_selection(0, encode_source_select(0, 0)).unwrap();
    r.game.resolve_selection(0, encode_source_select(0, 0)).unwrap();

    assert_eq!(*fired.lock().unwrap(), 1);
    assert!(r.game.pending_attack.is_none(), "attack state is fully cleared");
    assert_eq!(r.security_count(0), 1, "security was not checked");
    assert_eq!(r.trash_size(0), 3, "three sources were paid");
}
```

- [ ] **Step 3: Run the attack-cancel test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
```

Expected: FAIL because `cancel_pending_attack` does not exist or does not resume/clear attack state after pay-cost selection.

- [ ] **Step 4: Add the effect-context attack cancellation helper**

In `effect_context/mod.rs`, add:

```rust
    pub fn cancel_pending_attack(&mut self) {
        self.game.cancel_pending_attack_from_effect();
    }
```

- [ ] **Step 5: Add the combat cancellation implementation**

In `combat.rs`, add:

```rust
impl Game {
    pub fn cancel_pending_attack_from_effect(&mut self) {
        if let Some(pending) = self.pending_attack.as_mut() {
            pending.cancelled = true;
        }
        self.advance_pending_attack();
        if self.pending_attack.as_ref().is_some_and(|pending| pending.cancelled) {
            self.pending_attack = None;
        }
    }
}
```

If `advance_pending_attack` already clears cancelled attacks, keep the final `if` as a defensive no-op guard. Do not call battle resolution or security resolution in this helper.

- [ ] **Step 6: Lower DSL attack-cancel step**

In `lower_replacement.rs` or the DSL step lowering module that owns combat steps, map:

```yaml
- end_attack: true
```

to:

```rust
ctx.cancel_pending_attack();
```

- [ ] **Step 7: Run combat and replacement regressions**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- attack --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- source --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/combat.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/replacements/main.rs code/digimon-engine/tests/replacements/attack_cancel.rs
git commit -m "feat: allow effects to cancel pending attacks"
```

---

### Task 7: Documentation and Roadmap Closure

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

- [ ] **Step 1: Update engine gap documentation**

In `docs/RUST_ENGINE_GAPS.md`, mark the Group 3 cost/replacement items as covered by the new tests:

```markdown
### Cost and Replacement Framework

Status: implemented.

Regression coverage:
- `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`
- `code/digimon-engine/tests/replacements/context_predicates.rs`
- `code/digimon-engine/tests/replacements/partition.rs`
- `code/digimon-engine/tests/option_flow/replacement_integration.rs::bt17_097_delay_prevents_deletion_and_digivolves_from_hand`
- `code/digimon-engine/tests/replacements/attack_cancel.rs`

The engine supports triggered pay costs that park pending selections before
process execution, optional pay-cost decline, replacement cause/controller
predicates, Partition source selection, Delay-as-replacement prevention, and
effect-driven pending attack cancellation.
```

- [ ] **Step 2: Update archetype QA gaps**

In `qa/archetype-qa/engine-gaps.md`, replace the open Group 3 entries with:

```markdown
### Cost and Replacement Framework

Resolved by Group 3:
- Triggered effect costs may install pending selections and resume process only after cost payment.
- Optional cost decline skips process without hidden auto-selection.
- Replacement predicates can inspect cause, source controller, and subject controller.
- Partition source requirements are enforced before prevention.
- Delay options can pay themselves as replacement costs and prevent deletion.
- Effects can end a pending attack after a printed cost resolves.
```

- [ ] **Step 3: Check off the parent roadmap Group 3 child-plan task**

In `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`, under `Task 4: Create Child Plan for Cost and Replacement Framework`, mark the child-plan steps complete:

```markdown
- [x] Create `docs/superpowers/plans/2026-04-29-gap-group-3-cost-replacement.md`.
- [x] Define slices for:
  - `.pay_cost()` for non-BeforePayCost triggered effects.
  - Optional cost decline path through pending selection.
  - Replacement context cause/controller predicate.
  - Partition source enforcement and selection.
  - Delay-as-replacement prevention.
  - Attack cancellation return path.
- [x] Require regression fixtures:
  - `EX10-003` Tumblemon for attack cancellation.
  - `BT16-025` Paildramon for Partition source enforcement.
  - `BT17-097` Return to the Primogenitor for Delay-as-replacement.
  - `EX9-032` / `EX7-027` / `BT22-036` for replacement cause gate.
```

- [ ] **Step 4: Run the full Group 3 verification set**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context_predicates --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md
git commit -m "docs: close cost replacement gaps"
```

---

## Final Verification

Run the broad gates that protect the touched subsystems:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement
```

Expected: PASS. Existing compiler warnings about unrelated DNA-origin fields are acceptable only if they were present before this plan's implementation.

---

## Self-Review Checklist

- Spec coverage:
  - `.pay_cost()` for non-BeforePayCost triggered effects: Task 1.
  - Optional cost decline path through pending selection: Task 2.
  - Replacement context cause/controller predicate: Task 3.
  - Partition source enforcement and selection: Task 4.
  - Delay-as-replacement prevention: Task 5.
  - Attack cancellation return path: Task 6.
  - Required regression fixture names: Tasks 3 through 6.

- Placeholder scan:
  - No open-ended placeholder text remains in implementation steps.
  - Every new function referenced by a test is introduced in a subsequent step.
  - Every command includes an expected result.

- Type consistency:
  - `pending_pay_cost_effect` is the single parked continuation field.
  - `decline_pending_pay_cost` exists on both `Game` and `EffectContext`.
  - Replacement context accessors are exposed through `EffectReadContext`.
  - Partition helpers operate on `SourceSelectionRef`.
  - Attack cancellation is exposed as `EffectContext::cancel_pending_attack()`.
