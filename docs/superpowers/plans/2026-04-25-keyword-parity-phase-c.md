# Keyword Parity Phase C — Nested-selection-in-replacement substrate

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the engine substrate that lets a `WhenWouldBe*` replacement-process closure install a nested player-selection (e.g., "pick a Tamer") and resume cleanly after the player picks, with the resulting outcome routed back into the replacement dispatcher. Engine-only — DSL Phase 3 will consume the substrate later.

**Architecture:** Add `Game.parked_replacement: Option<ParkedReplacement>` slot. Process closures call existing `ctx.select_*` helpers from inside the body; the dispatcher detects `pending_selection.is_some()` after the process returns and snapshots the replacement context state. The user's `select_*` callback IS the continuation — Phase C adds NO new continuation storage; it adds four `EffectContext` outcome-setters (`cancel_leave`, `handle_replacement`, `redirect_replacement`, `substitute_replacement`) that write into the parked slot. After the callback completes, a post-callback hook in `resolve_generic_selection` drains the slot and routes through the existing `commit_deferred_outcome`.

**Tech Stack:** Rust 2021, `digimon-engine` crate, `cargo test --manifest-path digimon-engine/Cargo.toml`, `DebugRunner` test harness.

---

## Background — read before starting

- **Spec:** `docs/superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md`. The spec is short — read it end-to-end. Key sections:
  - §3 — DSL-aligned closure-continuation approach
  - §4 — Components (struct, outcome-setters, dispatcher hooks)
  - §5 — Data flow (Save, Fragment, Decoy, edge cases)
  - §10 — Open questions (all locked-in defaults; do NOT re-litigate)
- **Phase B baseline:** Tasks 2–7 added `progress_excludes` gates at every `EffectContext` mutation site; Task 8 added `Game.current_deletion_cause` + `commit_permanent_deletion` plumbing; Task 9 exposed `ctx.deletion_cause()` etc. Phase C builds on this — same `Game` struct, same dispatcher framework, same test-card patterns.
- **Pre-existing infrastructure to consume (do not re-derive):**
  - `replacement.rs::run_candidate_inner` (line 580) — runs the process closure inside a fresh `EffectContext` + `ReplacementContext`.
  - `replacement.rs::make_accept_callback` (line 724) — fires after the player accepts the optional-replacement dialog; runs `run_candidate_inner`, stashes outcome, calls `commit_deferred_outcome`.
  - `replacement.rs::commit_deferred_outcome` (line 794) — already handles every (Trash/Hand/Deck × Cancelled/Redirected/Substituted/None) arm. Phase C does NOT modify it.
  - `replacement.rs::run_commit_with_flag` (line 691) — panic-safe RAII helper. Mirror its shape for the parked-commit guard.
  - `effect_queue.rs::resolve_generic_selection` (line 659) — selection-callback dispatch path. The Phase C post-callback hook lives between line 690 (callback invocation) and line 696 (drainer resume).
  - `effect_context::EffectContext::select_*` helpers — already install `PendingSelection` correctly. Phase C touches NONE of these.
  - `effect_context::CountCappedZone::Material(handle)` — multi-pick over a permanent's stack. Used by Fragment.
- **Pre-existing test pattern:** `tests/replacements/native_keywords.rs` is the canonical example for hand-rolled `CardEffect` + `Effect::when_would_be_deleted(card).optional().replacement_process(|rctx| { ... })`. Phase C's test cards mirror that shape.
- **Phase 2d coexistence:** the in-flight DSL Phase 2d adds `Game.dsl_outer_tail` for a different concern (DSL step-list tail propagation). The `parked_replacement` slot is independent. Cross-reference both fields' docs (Task 1).

---

## File Structure

**Modified files:**
- `digimon-engine/src/game.rs` — new `parked_replacement` field + initializer.
- `digimon-engine/src/replacement.rs` — `ParkedReplacement` struct, post-process hook in `run_candidate_inner`, post-callback drain helper, `make_accept_callback` modification.
- `digimon-engine/src/effect_queue.rs` — call site for the post-callback drain in `resolve_generic_selection`.
- `digimon-engine/src/effect_context/mod.rs` — four outcome-setter methods on `EffectContext`.
- `digimon-engine/src/combat.rs` — delete the Task 3 limitation doc block (lines 2213–2229).
- `docs/RUST_ENGINE_API.md` — document the four new outcome-setters.
- `docs/RUST_ENGINE_GAPS.md` — mark `WhenWouldBeDeleted framework extensions` resolved.
- `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` — Phase C status.

**Created test files:**
- `digimon-engine/tests/replacements/nested_select_save.rs`
- `digimon-engine/tests/replacements/nested_select_fragment.rs`
- `digimon-engine/tests/replacements/nested_select_decoy.rs`
- `digimon-engine/tests/replacements/nested_select_regression.rs`
- `digimon-engine/tests/replacements/nested_select_substrate.rs`
- All wired into `digimon-engine/tests/replacements/main.rs` as new modules.

---

## Task 1: Add `ParkedReplacement` struct and `Game.parked_replacement` slot

**Files:**
- Modify: `digimon-engine/src/replacement.rs` (struct definition next to `ReplacementContext`)
- Modify: `digimon-engine/src/game.rs` (field on `Game` struct + initializer)

This is plumbing only. No behavioral test in this task — Tasks 6 and 7 add tests that exercise the slot.

- [ ] **Step 1: Add the `ParkedReplacement` struct**

In `digimon-engine/src/replacement.rs`, add this struct just below the existing `ReplacementContext` definition (around line 95, after `ReplacementContext`'s `impl` block):

```rust
/// Captures the replacement-context state needed to resume a process closure
/// after a nested player selection. Set by the dispatcher's post-process hook
/// in `run_candidate_inner` when the process closure parks a `PendingSelection`;
/// drained by `try_drain_parked_replacement_with_guard` in
/// `effect_queue::resolve_generic_selection` after the user's callback runs.
///
/// **Single-outstanding invariant:** at most one parked replacement at a time.
/// The post-process hook `debug_assert!`s on entry. If a real card surfaces a
/// nested-park (callback's body itself fires another deletion that parks),
/// escalate to a follow-up plan that converts the slot to a `Vec`-stack.
///
/// **Coexistence with `Game.dsl_outer_tail`** (Phase 2d): independent slots
/// for independent concerns. Both can be `Some(_)` simultaneously; cross-
/// references are at each field's doc comment.
///
/// Phase C §4.1.
#[derive(Debug)]
pub(crate) struct ParkedReplacement {
    pub subject: ReplacementSubject,
    pub cause: ReplacementCause,
    pub original_destination: Option<Zone>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
    /// Outcome the in-flight callback writes via `EffectContext::cancel_leave()`
    /// etc. Read by the dispatcher post-callback hook after the user closure
    /// returns. Defaults to `ReplacementOutcome::None` — original event proceeds
    /// when no outcome-setter is called.
    pub outcome: ReplacementOutcome,
}
```

The imports `Zone` and `PlayerId` are already in scope at the top of `replacement.rs` (verify with `grep -n "use crate::enums::" digimon-engine/src/replacement.rs`). `CardHandle` and `PermanentHandle` are also already imported.

- [ ] **Step 2: Add the field to `Game`**

In `digimon-engine/src/game.rs`, add the new field directly below the existing `current_deletion_cause` field (search `grep -n "current_deletion_cause" digimon-engine/src/game.rs` to find it). Insert this block:

```rust
    /// Parked replacement state when a `WhenWouldBe*` replacement-process
    /// closure installs a nested player selection. Set by the dispatcher's
    /// post-process hook in `replacement::run_candidate_inner`; drained by
    /// `effect_queue::resolve_generic_selection` after the user's callback
    /// runs. `None` outside a parked-replacement scope.
    ///
    /// **Single-outstanding invariant:** at most one slot occupied at a time;
    /// the dispatcher `debug_assert!`s on duplicate install.
    ///
    /// **Coexistence with `dsl_outer_tail`** (Phase 2d): independent slots for
    /// independent concerns. Phase C §4.1.
    #[doc(hidden)]
    pub(crate) parked_replacement: Option<crate::replacement::ParkedReplacement>,
```

- [ ] **Step 3: Initialize the field in `Game::new`**

Find the `Game::new` constructor's struct literal (search `grep -n "current_deletion_cause: None" digimon-engine/src/game.rs`). Add the new field's initializer directly below:

```rust
            current_deletion_cause: None,
            parked_replacement: None,
```

- [ ] **Step 4: Verify clean build**

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/replacement.rs digimon-engine/src/game.rs
git commit -m "engine: add ParkedReplacement struct + Game.parked_replacement slot"
```

---

## Task 2: Add `EffectContext::cancel_leave` outcome-setter

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` (around line 290 — after the security-check sugar block, before `as_read`)

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/replacements/main.rs`:

```rust
mod nested_select_substrate;
```

Create `digimon-engine/tests/replacements/nested_select_substrate.rs`:

```rust
//! Phase C — substrate-level tests for the parked-replacement slot and the
//! `EffectContext::cancel_leave` / `handle_replacement` / `redirect_replacement`
//! / `substitute_replacement` outcome-setters.
//!
//! These tests do NOT exercise end-to-end replacement flows — they manually
//! install `Game.parked_replacement` and verify that the outcome-setters
//! mutate the slot correctly. End-to-end coverage lives in the per-keyword
//! test files (`nested_select_save`, `nested_select_fragment`, etc.).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Zone};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{ParkedReplacement, ReplacementCause, ReplacementOutcome, ReplacementSubject};

fn fighter(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn install_parked(game: &mut digimon_engine::game::Game, target: PermanentHandle) {
    game.parked_replacement = Some(ParkedReplacement {
        subject: ReplacementSubject::Permanent(target),
        cause: ReplacementCause::OpponentEffect,
        original_destination: Some(Zone::Trash),
        source_card: CardHandle(0),
        source_permanent: None,
        controller: 0,
        outcome: ReplacementOutcome::None,
    });
}

#[test]
fn cancel_leave_writes_cancelled_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
    }

    let parked = r.game.parked_replacement.as_ref().expect("slot still set");
    assert_eq!(
        parked.outcome,
        ReplacementOutcome::Cancelled,
        "cancel_leave should write Cancelled outcome to parked slot"
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements cancel_leave_writes_cancelled_outcome_to_parked_slot 2>&1 | tail -10`
Expected: FAIL — compile error "no method named `cancel_leave`".

- [ ] **Step 3: Add the method to `EffectContext`**

In `digimon-engine/src/effect_context/mod.rs`, find the section header `// ─── OnDeletion cause accessors (Phase B §B5) ───` (or the equivalent header where Phase B's accessors landed). Below that block, add:

```rust
    // ─── Replacement-process outcome-setters (Phase C §4.2) ──────────────

    /// Cancel the parked leave-the-field event. The carrier stays on the
    /// field; the original deletion / return / etc. is suppressed.
    ///
    /// Writes `ReplacementOutcome::Cancelled` to `Game.parked_replacement.outcome`.
    /// Calling this outside a parked-replacement scope is a `debug_assert!`
    /// panic in dev builds; release builds silently no-op.
    ///
    /// Typical use: inside a `select_*` callback that runs as the body of a
    /// `WhenWouldBeDeleted` replacement-process closure (e.g., Save:
    /// "you may pick a Tamer to slide under instead of being deleted").
    pub fn cancel_leave(&mut self) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "cancel_leave called outside a replacement-process callback; \
             the outcome would be silently dropped"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
        }
    }
```

The `// ─── ... ───` comment style matches the existing section headers in `effect_context/mod.rs` (e.g., `// ─── Memory mutations ───`).

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements cancel_leave_writes_cancelled_outcome_to_parked_slot 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/replacements/nested_select_substrate.rs digimon-engine/tests/replacements/main.rs
git commit -m "engine: add EffectContext::cancel_leave outcome-setter"
```

---

## Task 3: Add `EffectContext::handle_replacement` outcome-setter

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` (next to `cancel_leave`)
- Modify: `digimon-engine/tests/replacements/nested_select_substrate.rs`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn handle_replacement_writes_custom_handled_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.handle_replacement();
    }

    let parked = r.game.parked_replacement.as_ref().expect("slot still set");
    assert_eq!(
        parked.outcome,
        ReplacementOutcome::CustomHandled,
        "handle_replacement should write CustomHandled to parked slot"
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements handle_replacement_writes_custom_handled_to_parked_slot 2>&1 | tail -10`
Expected: FAIL — "no method named `handle_replacement`".

- [ ] **Step 3: Add the method**

Append to the `// ─── Replacement-process outcome-setters` section in `digimon-engine/src/effect_context/mod.rs`:

```rust
    /// Mark the parked replacement as custom-handled — the process body has
    /// already mutated state and the original event should be skipped.
    /// Distinct from `cancel_leave` only at the doc level; both result in
    /// `commit_deferred_outcome` taking the no-op arm.
    ///
    /// Writes `ReplacementOutcome::CustomHandled` to the parked slot.
    /// Calling this outside a parked-replacement scope is a `debug_assert!`
    /// panic in dev builds; release builds silently no-op.
    pub fn handle_replacement(&mut self) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "handle_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::CustomHandled;
        }
    }
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements handle_replacement_writes_custom_handled_to_parked_slot 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/replacements/nested_select_substrate.rs
git commit -m "engine: add EffectContext::handle_replacement outcome-setter"
```

---

## Task 4: Add `EffectContext::redirect_replacement` outcome-setter

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/replacements/nested_select_substrate.rs`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn redirect_replacement_writes_redirected_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.redirect_replacement(Zone::Hand);
    }

    let parked = r.game.parked_replacement.as_ref().expect("slot still set");
    assert_eq!(
        parked.outcome,
        ReplacementOutcome::Redirected(Zone::Hand),
        "redirect_replacement(Hand) should write Redirected(Hand) to parked slot"
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements redirect_replacement_writes_redirected_outcome_to_parked_slot 2>&1 | tail -10`
Expected: FAIL — "no method named `redirect_replacement`".

- [ ] **Step 3: Add the method**

Append to the outcome-setter section in `digimon-engine/src/effect_context/mod.rs`:

```rust
    /// Redirect the parked event to a different zone (e.g., Trash → Deck for
    /// Evade, Trash → Hand for return-to-hand replacement).
    ///
    /// Writes `ReplacementOutcome::Redirected(zone)` to the parked slot.
    /// Honored by `commit_deferred_outcome`'s existing redirect arms.
    /// Calling outside a parked-replacement scope is a `debug_assert!` panic
    /// in dev builds; release builds silently no-op.
    pub fn redirect_replacement(&mut self, zone: crate::enums::Zone) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "redirect_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Redirected(zone);
        }
    }
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements redirect_replacement_writes_redirected_outcome_to_parked_slot 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/replacements/nested_select_substrate.rs
git commit -m "engine: add EffectContext::redirect_replacement outcome-setter"
```

---

## Task 5: Add `EffectContext::substitute_replacement` outcome-setter

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/replacements/nested_select_substrate.rs`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn substitute_replacement_writes_substituted_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("X"))
        .add_card(fighter("Y"))
        .start();
    let target = r.place_on_field(0, "X", None);
    let other = r.place_on_field(0, "Y", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.substitute_replacement(ReplacementSubject::Permanent(other));
    }

    let parked = r.game.parked_replacement.as_ref().expect("slot still set");
    assert_eq!(
        parked.outcome,
        ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)),
        "substitute_replacement should write Substituted(other) to parked slot"
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements substitute_replacement_writes_substituted_outcome_to_parked_slot 2>&1 | tail -10`
Expected: FAIL — "no method named `substitute_replacement`".

- [ ] **Step 3: Add the method**

Append to the outcome-setter section in `digimon-engine/src/effect_context/mod.rs`:

```rust
    /// Substitute a different subject for the parked event. `commit_deferred_outcome`
    /// recursively dispatches the original event against the substituted subject
    /// (e.g., Decoy: replace deletion-target with self).
    ///
    /// Writes `ReplacementOutcome::Substituted(subject)` to the parked slot.
    /// Calling outside a parked-replacement scope is a `debug_assert!` panic
    /// in dev builds; release builds silently no-op.
    pub fn substitute_replacement(&mut self, subject: crate::replacement::ReplacementSubject) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "substitute_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Substituted(subject);
        }
    }
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements substitute_replacement_writes_substituted_outcome_to_parked_slot 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/replacements/nested_select_substrate.rs
git commit -m "engine: add EffectContext::substitute_replacement outcome-setter"
```

---

## Task 6: Post-process dispatcher hook + accept-callback skip-when-parked

**Files:**
- Modify: `digimon-engine/src/replacement.rs:580-614` (`run_candidate_inner`)
- Modify: `digimon-engine/src/replacement.rs:724-755` (`make_accept_callback`)
- Modify: `digimon-engine/tests/replacements/nested_select_substrate.rs`

This task wires the install-side of the parked-replacement flow. Tests in this task verify the slot is populated correctly when a process closure parks a selection. End-to-end "user picks → outcome commits" verification lands in Task 7.

- [ ] **Step 1: Append the failing test**

```rust
use digimon_engine::effect::{CardEffect, Effect};

#[test]
fn post_process_hook_installs_parked_replacement_when_select_called() {
    // Hand-rolled card with a WhenWouldBeDeleted replacement whose process
    // closure installs a select_own_permanent — the post-process hook should
    // see pending_selection.is_some() and install Game.parked_replacement.

    struct ParkingCard {
        installed: Arc<Mutex<bool>>,
    }
    impl CardEffect for ParkingCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let installed = Arc::clone(&self.installed);
            vec![Effect::when_would_be_deleted(card)
                .name("PARK-TEST")
                .optional()
                .replacement_process(move |rctx| {
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        move |_ctx, _picked| {
                            // Body never runs in this test — we resolve the
                            // outer accept then inspect parked_replacement
                            // before resolving the inner select.
                        },
                    );
                    *installed.lock().unwrap() = true;
                })
                .build()]
        }
    }

    let installed = Arc::new(Mutex::new(false));
    let mut r = DebugRunner::builder()
        .add_card(fighter("PARK-TEST"))
        .add_card(fighter("OTHER"))
        .start();
    r.register_effect(
        "PARK-TEST",
        Arc::new(ParkingCard { installed: Arc::clone(&installed) }),
    );
    let parker = r.place_on_field(0, "PARK-TEST", None);
    let _other = r.place_on_field(0, "OTHER", None);

    // Trigger the would-be-deleted dispatch.
    r.game.delete_permanent_with_effects(parker);

    // Outer optional-accept dialog is installed.
    let pending = r.game.pending_selection.as_ref().expect("optional accept installed");
    assert_eq!(pending.kind, digimon_engine::selection::SelectionKind::Replacement);

    // Resolve the outer accept (REPLACEMENT_ACCEPT action).
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept ok");

    // Process closure ran (installed flag set).
    assert!(*installed.lock().unwrap(), "process closure should have run");
    // Inner select_own_permanent installed a fresh PendingSelection.
    assert!(r.game.pending_selection.is_some(), "inner select installed");
    // POST-PROCESS HOOK: parked_replacement populated.
    assert!(
        r.game.parked_replacement.is_some(),
        "post-process hook should install Game.parked_replacement"
    );
    let parked = r.game.parked_replacement.as_ref().unwrap();
    assert_eq!(parked.subject, ReplacementSubject::Permanent(parker));
    assert_eq!(parked.outcome, ReplacementOutcome::None);
}
```

**API references** (already verified — use exactly these names):
- `CardEffect` trait lives in `digimon_engine::effect` (NOT `digimon_engine::cards`).
- Trait method: `fn effects(&self, card: CardHandle) -> Vec<Effect>`.
- Registration: `r.register_effect(&str, Arc<dyn CardEffect>)` — method on `DebugRunner` at `digimon-engine/src/debug_runner.rs:182`. There is NO free `register_card_effect` function; it's runner-scoped.
- Add `use std::sync::{Arc, Mutex};` at the top of the test file if not already present.

The Phase B Task 9 in-observer test in `digimon-engine/tests/combat/deletion_cause_observer.rs` is a working example of the same registration pattern (search for `r.register_effect` there).

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements post_process_hook_installs_parked_replacement_when_select_called 2>&1 | tail -20`
Expected: FAIL — `parked_replacement` stays `None` because the hook doesn't exist.

- [ ] **Step 3: Add the post-process hook in `run_candidate_inner`**

In `digimon-engine/src/replacement.rs`, modify the body of `run_candidate_inner` (around line 580). The current body ends with:

```rust
    process(&mut rep_ctx);
    rep_ctx.outcome
}
```

Replace those last two lines with:

```rust
    process(&mut rep_ctx);

    // Phase C §4.3 — POST-PROCESS HOOK: detect nested-select park.
    // If the process closure installed a PendingSelection (via ctx.select_*),
    // snapshot the replacement context state into Game.parked_replacement
    // so the user's callback can later set the outcome via
    // EffectContext::cancel_leave / etc., and the post-callback hook in
    // resolve_generic_selection can drain the slot via commit_deferred_outcome.
    //
    // Single-outstanding invariant: at most one parked replacement at a time.
    if game.pending_selection.is_some() {
        debug_assert!(
            game.parked_replacement.is_none(),
            "nested replacement park; outer outcome would be lost. Phase C \
             scope assumes a callback that itself fires a deletion will not \
             also install a select_* selection. If a real card requires \
             nested-park, extend ParkedReplacement into a Vec-stack."
        );
        game.parked_replacement = Some(ParkedReplacement {
            subject,
            cause,
            original_destination,
            source_card,
            source_permanent,
            controller,
            outcome: ReplacementOutcome::None,
        });
        // Caller (e.g. make_accept_callback) checks pending_selection.is_some()
        // and yields without committing — the parked outcome will be drained
        // after the user's select_* callback fires.
        return ReplacementOutcome::None;
    }

    rep_ctx.outcome
}
```

The closure can no longer borrow `game` mutably while `rep_ctx` is alive, so we need to drop `rep_ctx` first. The existing structure `let mut ctx = EffectContext::new(game, ...); let mut rep_ctx = ReplacementContext { effect: &mut ctx, ... };` keeps `rep_ctx` borrowing `game` through `ctx`. After `process(&mut rep_ctx)` returns, we need to read `rep_ctx.outcome` before checking `game.pending_selection`. Fix by extracting `outcome` first:

```rust
    process(&mut rep_ctx);
    let outcome = rep_ctx.outcome;
    drop(rep_ctx);
    drop(ctx);

    // Phase C §4.3 — POST-PROCESS HOOK: ...
    if game.pending_selection.is_some() {
        debug_assert!(
            game.parked_replacement.is_none(),
            "nested replacement park; outer outcome would be lost. ..."
        );
        game.parked_replacement = Some(ParkedReplacement {
            subject,
            cause,
            original_destination,
            source_card,
            source_permanent,
            controller,
            outcome: ReplacementOutcome::None,
        });
        return ReplacementOutcome::None;
    }

    outcome
}
```

The `drop(rep_ctx); drop(ctx);` lines explicitly release the borrows on `game` so the subsequent `game.pending_selection.is_some()` and `game.parked_replacement = Some(...)` writes compile.

- [ ] **Step 4: Modify `make_accept_callback` to skip commit when parked**

In `digimon-engine/src/replacement.rs`, find `make_accept_callback` (line 724). The current body of the closure does:

```rust
Box::new(move |game: &mut crate::game::Game, _action_id: u16| {
    let outcome = run_candidate_inner(...);
    game.replacement_pending_outcome = Some(outcome);
    run_commit_with_flag(game, |game| {
        commit_deferred_outcome(game, subject, cause, original_destination, outcome);
    });
})
```

Replace with:

```rust
Box::new(move |game: &mut crate::game::Game, _action_id: u16| {
    let outcome = run_candidate_inner(
        game,
        &card_id,
        source_card,
        source_permanent,
        controller,
        effect_slot,
        subject,
        cause,
        original_destination,
    );
    game.replacement_pending_outcome = Some(outcome);

    // Phase C §4.3: if the process parked a selection (parked_replacement
    // is now Some(_)), skip immediate commit — the post-callback hook in
    // resolve_generic_selection will drain the slot after the user's
    // select_* callback runs.
    if game.parked_replacement.is_some() {
        return;
    }

    run_commit_with_flag(game, |game| {
        commit_deferred_outcome(game, subject, cause, original_destination, outcome);
    });
})
```

- [ ] **Step 5: Run the test to confirm it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements post_process_hook_installs_parked_replacement_when_select_called 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 6: Run the full replacements test suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements 2>&1 | tail -10`
Expected: all green. Existing tests must still pass — synchronous processes (Barrier, Evade, Decode auto-installs) don't install pending_selections inside their bodies, so the new hook short-circuits.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/replacement.rs digimon-engine/tests/replacements/nested_select_substrate.rs
git commit -m "engine: post-process dispatcher hook installs parked replacement"
```

---

## Task 7: Post-callback drain hook + commit_deferred_outcome integration

**Files:**
- Modify: `digimon-engine/src/replacement.rs` (new public-to-crate helper `try_drain_parked_replacement_with_guard`)
- Modify: `digimon-engine/src/effect_queue.rs:659-715` (`resolve_generic_selection` — add the drain call)
- Modify: `digimon-engine/tests/replacements/nested_select_substrate.rs`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn post_callback_hook_drains_parked_and_commits_outcome() {
    // After the inner select_* callback writes outcome via cancel_leave(),
    // the post-callback hook in resolve_generic_selection should:
    //   1. Take Game.parked_replacement
    //   2. Run commit_deferred_outcome with the parked outcome
    //   3. Leave Game.parked_replacement = None

    struct CancelOnPickCard;
    impl CardEffect for CancelOnPickCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("CANCEL-ON-PICK")
                .optional()
                .replacement_process(|rctx| {
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        |ctx, _picked| {
                            ctx.cancel_leave();
                        },
                    );
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("CANCEL-ON-PICK"))
        .add_card(fighter("OTHER"))
        .start();
    r.register_effect("CANCEL-ON-PICK", Arc::new(CancelOnPickCard));
    let parker = r.place_on_field(0, "CANCEL-ON-PICK", None);
    let _other = r.place_on_field(0, "OTHER", None);

    r.game.delete_permanent_with_effects(parker);

    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept ok");
    assert!(r.game.parked_replacement.is_some(), "parked installed");

    // The inner OwnField selection's first valid action ID picks the parker.
    let pending = r.game.pending_selection.as_ref().expect("inner select");
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game.resolve_selection(player, action).expect("inner pick ok");

    // POST-CALLBACK HOOK should have drained parked_replacement and committed.
    assert!(
        r.game.parked_replacement.is_none(),
        "post-callback hook should clear parked_replacement after commit"
    );
    // Deletion was cancelled — parker stayed on the field.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "parker survived: deletion was cancelled by cancel_leave()"
    );
    assert!(r.game.pending_selection.is_none(), "no leftover selection");
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements post_callback_hook_drains_parked_and_commits_outcome 2>&1 | tail -15`
Expected: FAIL — without the drain hook, `parked_replacement` stays `Some(_)` and the deletion isn't actually cancelled.

- [ ] **Step 3: Add the drain helper to `replacement.rs`**

In `digimon-engine/src/replacement.rs`, add this function at module scope (place it near `run_commit_with_flag`, around line 691):

```rust
/// Phase C §4.4 — POST-CALLBACK DRAIN: take `Game.parked_replacement` (if any)
/// and route its outcome through `commit_deferred_outcome`. Called from
/// `effect_queue::resolve_generic_selection` after the user's selection
/// callback returns. Panic-safe: the outer flag guard mirrors
/// `run_commit_with_flag` so a panic in the commit body doesn't leak the
/// `in_replacement_commit` flag.
///
/// No-op when `parked_replacement.is_none()` — the vast majority of
/// selection resolutions don't involve a parked replacement.
pub(crate) fn try_drain_parked_replacement_with_guard(game: &mut crate::game::Game) {
    let Some(parked) = game.parked_replacement.take() else {
        return;
    };

    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

    let prior = game.in_replacement_commit;
    game.in_replacement_commit = true;

    let result = catch_unwind(AssertUnwindSafe(|| {
        commit_deferred_outcome(
            game,
            parked.subject,
            parked.cause,
            parked.original_destination,
            parked.outcome,
        );
    }));

    game.in_replacement_commit = prior;

    if let Err(payload) = result {
        resume_unwind(payload);
    }
}
```

- [ ] **Step 4: Wire the drain hook into `resolve_generic_selection`**

In `digimon-engine/src/effect_queue.rs:659-715`, find the line:

```rust
        if is_pass {
            if let Some(on_decline) = sel.on_decline {
                on_decline(self);
            }
        } else {
            (sel.callback)(self, action_id);
        }
```

Add the drain hook directly below (before the existing `if self.pending_selection.is_none() { self.drain_effect_queue(); }` block):

```rust
        if is_pass {
            if let Some(on_decline) = sel.on_decline {
                on_decline(self);
            }
        } else {
            (sel.callback)(self, action_id);
        }

        // Phase C §4.4: drain parked-replacement slot (if any). If the
        // resolved selection was a callback inside a replacement-process,
        // its body wrote the outcome via EffectContext::cancel_leave() etc.;
        // commit it now via commit_deferred_outcome.
        crate::replacement::try_drain_parked_replacement_with_guard(self);

        // If the callback parked a fresh selection, leave the drainer alone.
        // ... (existing comment + drainer block continues unchanged)
```

- [ ] **Step 5: Run the test to confirm it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements post_callback_hook_drains_parked_and_commits_outcome 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 6: Run the full replacements test suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements 2>&1 | tail -10`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/replacement.rs digimon-engine/src/effect_queue.rs digimon-engine/tests/replacements/nested_select_substrate.rs
git commit -m "engine: post-callback drain hook commits parked replacement outcome"
```

---

## Task 8: Substrate-level edge-case tests

**Files:**
- Modify: `digimon-engine/tests/replacements/nested_select_substrate.rs`

Lock in the substrate's edge-case contracts (default-None, last-write-wins, single-outstanding invariant, panic recovery).

- [ ] **Step 1: Append the four edge-case tests**

```rust
#[test]
fn default_none_when_callback_skips_outcome() {
    // A replacement process that installs a select_* but the user's callback
    // never calls any outcome-setter — parked.outcome stays None, so the
    // original event proceeds normally.

    struct NoOutcomeCard;
    impl CardEffect for NoOutcomeCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("NO-OUTCOME")
                .optional()
                .replacement_process(|rctx| {
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        |_ctx, _picked| {
                            // No outcome-setter call — outcome stays None.
                        },
                    );
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("NO-OUTCOME"))
        .add_card(fighter("X"))
        .start();
    r.register_effect("NO-OUTCOME", Arc::new(NoOutcomeCard));
    let parker = r.place_on_field(0, "NO-OUTCOME", None);
    let _x = r.place_on_field(0, "X", None);

    r.game.delete_permanent_with_effects(parker);
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept");
    let pending = r.game.pending_selection.as_ref().unwrap();
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game.resolve_selection(player, action).expect("pick");

    // outcome was None → original deletion proceeds → parker is gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "parker should have been deleted (outcome=None defaults to original event)"
    );
}

#[test]
fn last_write_wins_on_multiple_outcome_setters() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
        ctx.redirect_replacement(Zone::Hand);
    }

    let parked = r.game.parked_replacement.as_ref().unwrap();
    assert_eq!(
        parked.outcome,
        ReplacementOutcome::Redirected(Zone::Hand),
        "last write should win"
    );
}

#[test]
#[should_panic(expected = "nested replacement park")]
fn single_outstanding_park_panics_on_double_install() {
    // Manually install parked_replacement, then trigger a second install via
    // the dispatcher post-process hook — should panic in dev builds.

    struct DoubleParkCard;
    impl CardEffect for DoubleParkCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("DOUBLE-PARK")
                .optional()
                .replacement_process(|rctx| {
                    rctx.effect.select_own_permanent(
                        "x", false, |_g, _h| true, |_ctx, _p| {},
                    );
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("DOUBLE-PARK"))
        .add_card(fighter("X"))
        .start();
    r.register_effect("DOUBLE-PARK", Arc::new(DoubleParkCard));
    let parker = r.place_on_field(0, "DOUBLE-PARK", None);
    let other = r.place_on_field(0, "X", None);

    // Pre-install parked_replacement so the dispatcher hook sees an existing slot.
    install_parked(&mut r.game, other);

    r.game.delete_permanent_with_effects(parker);
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    // The accept-callback runs run_candidate_inner which hits the post-process
    // hook with parked_replacement already Some(_) → debug_assert! fires.
    let _ = r.game.resolve_selection(0, REPLACEMENT_ACCEPT);
}

#[test]
fn cancel_leave_outside_parked_scope_panics_in_dev() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let _ = r.place_on_field(0, "X", None);

    // No parked_replacement installed — ctx.cancel_leave() should panic in dev.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
    }));
    assert!(
        result.is_err(),
        "cancel_leave outside parked scope should debug_assert!"
    );
}
```

- [ ] **Step 2: Run the new tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements 2>&1 | tail -10`
Expected: all 9 substrate tests pass (the 4 outcome-setter tests from Tasks 2-5 plus the 4 edge-case tests + the parked-install + drain tests from Tasks 6-7 = 9 total, plus existing replacement tests still green).

If `single_outstanding_park_panics_on_double_install` fails because the panic is caught somewhere internal (e.g., inside `resolve_selection`'s error handling), the test framework's `#[should_panic]` only catches panics that propagate to the test boundary. If the panic is swallowed, the test will fail with "did not panic" — in that case, change the test to manually inspect that the second `parked_replacement = Some(_)` write triggers the assertion via a direct call, or skip the test with a `#[ignore]` and a TODO if the panic-propagation path needs a separate plan.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/replacements/nested_select_substrate.rs
git commit -m "test: substrate edge-cases — default-None / last-write-wins / single-outstanding / panic-on-misuse"
```

---

## Task 9: End-to-end test card — Save (single-pick + Cancelled)

**Files:**
- Create: `digimon-engine/tests/replacements/nested_select_save.rs`
- Modify: `digimon-engine/tests/replacements/main.rs` (add `mod nested_select_save;`)

This is the first end-to-end test card that exercises the full Save-like flow: optional outer accept → inner own-Tamer pick → cancel_leave → carrier survives. Plus the outer-decline and empty-filter edge cases.

- [ ] **Step 1: Wire the new module**

Append to `digimon-engine/tests/replacements/main.rs`:

```rust
mod nested_select_save;
```

- [ ] **Step 2: Write the file with all three tests**

Create `digimon-engine/tests/replacements/nested_select_save.rs`:

```rust
//! Phase C end-to-end test card — Save-like single-pick replacement.
//!
//! A Digimon with `<Save>` on `WhenWouldBeDeleted` may pick one of the
//! controller's own Tamers; if they do, the Digimon "slides under" the
//! Tamer (Phase D primitive — for this Phase C test we substitute an
//! inline source-push) and the deletion is cancelled. Three cases:
//!   1. Accept + pick → carrier survives, gains a stack source.
//!   2. Decline outer accept → carrier dies, no stack mutation.
//!   3. No Tamers on field → no candidate offered, carrier dies normally.

use std::sync::Arc;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementSubject;

fn save_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: "SAVE_LIKE".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

fn tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// SAVE_LIKE effect — selects a Tamer, pushes self's top card under it as a
/// new bottom source (Phase D primitive substitute), and cancels deletion.
/// Each test installs this fresh on its own DebugRunner via `r.register_effect`.
struct SaveLike;
impl CardEffect for SaveLike {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Save>")
            .optional()
            .replacement_process(|rctx| {
                let me = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                rctx.effect.select_own_permanent(
                    "pick a Tamer to slide under",
                    false,
                    |g, h| {
                        let p = &g.players[h.player as usize];
                        if let Some(perm) = p.battle_area.get(h.index as usize) {
                            perm.is_tamer(&g.card_data)
                        } else {
                            false
                        }
                    },
                    move |ctx, tamer| {
                        // Phase D primitive substitute: manually push self's
                        // top card under the tamer as a new bottom source.
                        // For a real Save card this would be
                        // ctx.move_self_under(tamer).
                        let me_player = me.player;
                        let me_idx = me.index as usize;
                        let tamer_player = tamer.player;
                        let tamer_idx = tamer.index as usize;
                        if let Some(top) = ctx.game.players[me_player as usize]
                            .battle_area[me_idx]
                            .card_sources
                            .last()
                            .cloned()
                        {
                            ctx.game.players[tamer_player as usize]
                                .battle_area[tamer_idx]
                                .card_sources
                                .insert(0, top);
                        }
                        ctx.cancel_leave();
                    },
                );
            })
            .build()]
    }
}

#[test]
fn save_picks_tamer_and_cancels_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer("TAMER"))
        .start();
    r.register_effect("SAVE-D", Arc::new(SaveLike));
    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    let stack_before = r.game.players[0].battle_area[saved.index as usize]
        .card_sources
        .len();

    r.game.delete_permanent_with_effects(saved);

    // Outer accept dialog is up.
    assert!(r.game.pending_selection.is_some());
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept");

    // Inner Tamer pick is up; pick the only Tamer.
    let pending = r.game.pending_selection.as_ref().expect("inner select");
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game.resolve_selection(player, action).expect("pick");

    // Saved digimon survived (cancel_leave fired).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "Saved digimon survived because cancel_leave was called"
    );
    // Tamer gained a stack source from our move-under primitive.
    let tamer_perm = &r.game.players[0].battle_area[1];
    assert!(
        tamer_perm.card_sources.len() > 1,
        "Tamer should have gained a stack source from move-self-under"
    );
    let _ = stack_before;
}

#[test]
fn save_outer_decline_proceeds_with_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer("TAMER"))
        .start();
    r.register_effect("SAVE-D", Arc::new(SaveLike));
    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    r.game.delete_permanent_with_effects(saved);

    assert!(r.game.pending_selection.is_some());
    use digimon_engine::action::space::PASS;
    r.game.resolve_selection(0, PASS).expect("decline");

    // Decline → original deletion proceeds → Saved digimon is gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Decline → original deletion → only the Tamer remains"
    );
    assert!(r.game.parked_replacement.is_none(), "no parked slot on decline path");
}

#[test]
fn save_with_no_tamers_does_not_offer() {
    // No Tamer on field → the candidate filter for the inner select is empty,
    // BUT the outer optional-accept still fires (because Phase C does not
    // pre-filter candidates on inner-filter emptiness — that's a Phase D
    // auto-install authoring concern). On accept, the inner select_own_permanent
    // sees zero candidates and silently no-ops; the user's callback never
    // runs; outcome stays None; original deletion proceeds.
    let mut r = DebugRunner::builder().add_card(save_card("SAVE-D")).start();
    r.register_effect("SAVE-D", Arc::new(SaveLike));
    let saved = r.place_on_field(0, "SAVE-D", None);

    r.game.delete_permanent_with_effects(saved);
    assert!(r.game.pending_selection.is_some(), "outer accept still installed");

    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept");

    // Inner select_own_permanent had no Tamer candidates → no PendingSelection
    // installed; user callback never ran; outcome stayed None.
    // Either parked_replacement is None (process closure returned without
    // installing pending_selection) OR the post-callback drain already fired
    // with outcome=None (which would commit the original delete).
    // Either way, the saved digimon should be gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "Empty Tamer filter → outcome=None → original deletion proceeds"
    );
    let _ = saved;
}
```

- [ ] **Step 3: Run all three Save tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements save_ 2>&1 | tail -15`
Expected: all 3 tests pass.

- [ ] **Step 4: Run the full replacements suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements 2>&1 | tail -10`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/tests/replacements/nested_select_save.rs digimon-engine/tests/replacements/main.rs
git commit -m "test: end-to-end nested-select Save (single-pick + cancel + decline + empty-filter)"
```

---

## Task 10: End-to-end test card — Fragment(2) (multi-pick + Cancelled)

**Files:**
- Create: `digimon-engine/tests/replacements/nested_select_fragment.rs`
- Modify: `digimon-engine/tests/replacements/main.rs`

Multi-pick uses the existing `select_count_capped_multi`; same parked-replacement substrate handles it.

- [ ] **Step 1: Wire the new module**

Append to `digimon-engine/tests/replacements/main.rs`:

```rust
mod nested_select_fragment;
```

- [ ] **Step 2: Write the test file**

Create `digimon-engine/tests/replacements/nested_select_fragment.rs`:

```rust
//! Phase C end-to-end test card — Fragment(2)-like multi-pick replacement.
//!
//! A Digimon with `<Fragment(2)>` on `WhenWouldBeDeleted` may trash 2 of
//! its own digivolution sources to cancel deletion. Two cases:
//!   1. Stack ≥ 3 sources → accept + pick 2 → cancel + stack shrinks by 2.
//!   2. Stack < N+1 sources → outer optional-accept gated off via condition.

use std::sync::Arc;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::CountCappedZone;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementSubject;

const FRAGMENT_N: u8 = 2;

fn fragment_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: "FRAGMENT_LIKE".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

fn under_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// FragmentLike effect — multi-pick over self's stack sources, cancels
/// deletion if N sources are trashed. Each test installs this fresh on its
/// own DebugRunner via `r.register_effect`.
struct FragmentLike;
impl CardEffect for FragmentLike {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Fragment(2)>")
            .optional()
            // Phase D's auto-install will set this condition; for Phase C
            // end-to-end we set it inline.
            .condition(|ctx| {
                let h = match ctx.source_permanent {
                    Some(h) => h,
                    None => return false,
                };
                let p = &ctx.game.players[h.player as usize];
                let perm = match p.battle_area.get(h.index as usize) {
                    Some(p) => p,
                    None => return false,
                };
                // Need > N sources so we can trash N and still keep ≥ 1.
                perm.card_sources.len() > FRAGMENT_N as usize
            })
            .replacement_process(|rctx| {
                let me = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                let controller = rctx.effect.player;
                rctx.effect.select_count_capped_multi(
                    controller,
                    CountCappedZone::Material(me),
                    FRAGMENT_N,
                    "trash N sources",
                    false,
                    |_g, _src| true,
                    move |ctx, picks| {
                        // Trash sources by index. picks is a Vec<usize> of
                        // positions in me.card_sources at install time.
                        // Sort descending so removals don't shift indices.
                        let mut sorted = picks.clone();
                        sorted.sort_by(|a, b| b.cmp(a));
                        let me_player = me.player;
                        let me_idx = me.index as usize;
                        for src_idx in sorted {
                            let perm = &mut ctx.game.players[me_player as usize]
                                .battle_area[me_idx];
                            if src_idx < perm.card_sources.len() {
                                let popped = perm.card_sources.remove(src_idx);
                                ctx.game.players[me_player as usize]
                                    .trash
                                    .push(popped);
                            }
                        }
                        ctx.cancel_leave();
                    },
                );
            })
            .build()]
    }
}

#[test]
fn fragment_2_picks_two_sources_and_cancels() {
    // Register the FragmentLike effect against this runner.

    let mut r = DebugRunner::builder()
        .add_card(fragment_card("FRAG"))
        .add_card(under_card("UNDER1"))
        .add_card(under_card("UNDER2"))
        .start();
    r.register_effect("FRAG", Arc::new(FragmentLike));
    let frag = r.place_on_field(0, "FRAG", None);

    // Manually push 2 cards under FRAG to make stack size 3.
    for under in &["UNDER1", "UNDER2"] {
        let data_idx = r.game.card_data.iter().position(|c| c.card_id == *under).unwrap();
        let next = r.game.next_card_index();
        let bottom = CardSource::new(data_idx, 0, next);
        let perm = &mut r.game.players[0].battle_area[frag.index as usize];
        perm.card_sources.insert(0, bottom);
    }
    let stack_before = r.game.players[0].battle_area[frag.index as usize]
        .card_sources
        .len();
    assert_eq!(stack_before, 3, "preconditions");

    r.game.delete_permanent_with_effects(frag);
    assert!(r.game.pending_selection.is_some(), "outer accept up");
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept");

    // Multi-pick chain: 2 picks. Each pick uses the first valid action ID.
    for _ in 0..FRAGMENT_N {
        let pending = r.game.pending_selection.as_ref().expect("inner step");
        let action = pending.valid_action_ids[0];
        let player = pending.selecting_player;
        r.game.resolve_selection(player, action).expect("pick");
    }

    // Carrier survived; stack shrank by 2.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "FRAG survived (cancel_leave)"
    );
    let stack_after = r.game.players[0].battle_area[frag.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after,
        stack_before - FRAGMENT_N as usize,
        "stack shrank by N"
    );
}

#[test]
fn fragment_n_too_few_sources_does_not_offer() {
    // Stack size 1 (just the top card) — condition gates the outer accept off.
    // Register the FragmentLike effect against this runner.

    let mut r = DebugRunner::builder().add_card(fragment_card("FRAG")).start();
    r.register_effect("FRAG", Arc::new(FragmentLike));
    let frag = r.place_on_field(0, "FRAG", None);
    let stack_size = r.game.players[0].battle_area[frag.index as usize]
        .card_sources
        .len();
    assert_eq!(stack_size, 1, "preconditions: only the top card");

    r.game.delete_permanent_with_effects(frag);

    // Outer accept should NOT have been offered (condition false: 1 ≤ N).
    // The deletion should have proceeded directly.
    assert!(
        r.game.pending_selection.is_none(),
        "no PendingSelection — condition gated the outer accept"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "FRAG was deleted normally (no Fragment offered)"
    );
}
```

- [ ] **Step 3: Run the new tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements fragment 2>&1 | tail -15`
Expected: both tests pass.

- [ ] **Step 4: Run the full replacements suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements 2>&1 | tail -10`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/tests/replacements/nested_select_fragment.rs digimon-engine/tests/replacements/main.rs
git commit -m "test: end-to-end nested-select Fragment(2) (multi-pick + condition gate)"
```

---

## Task 11: End-to-end test card — Decoy (substitute outcome)

**Files:**
- Create: `digimon-engine/tests/replacements/nested_select_decoy.rs`
- Modify: `digimon-engine/tests/replacements/main.rs`

Tests the `Substituted(Permanent(...))` outcome arm.

- [ ] **Step 1: Wire the module**

Append to `digimon-engine/tests/replacements/main.rs`:

```rust
mod nested_select_decoy;
```

- [ ] **Step 2: Write the test file**

Create `digimon-engine/tests/replacements/nested_select_decoy.rs`:

```rust
//! Phase C end-to-end test card — Decoy-like substitute replacement.
//!
//! When an ally would be deleted, the decoy carrier may redirect deletion
//! to itself. Tests `EffectContext::substitute_replacement` end-to-end.

use std::sync::Arc;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementSubject;

fn decoy_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: "DECOY_LIKE".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

fn ally_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// DecoyLike effect — when an ally would be deleted, the decoy may redirect
/// deletion to itself via substitute_replacement. Each test installs this
/// fresh on its own DebugRunner via `r.register_effect`.
struct DecoyLike;
impl CardEffect for DecoyLike {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Decoy>")
            .optional()
            .replacement_process(|rctx| {
                        // Only fire when the subject is a DIFFERENT permanent
                        // (the decoy redirects deletion of its allies, not
                        // self-deletion).
                        let ally = match rctx.subject {
                            ReplacementSubject::Permanent(h) => h,
                            _ => return,
                        };
                        let me = match rctx.effect.source_permanent {
                            Some(h) if h != ally => h,
                            _ => return, // self-deletion: skip
                        };
                        let _ = ally;
                        rctx.effect.substitute_replacement(
                            ReplacementSubject::Permanent(me),
                        );
                        // No selection — synchronous-substitute. Confirms the
                        // outcome-setter works even WITHOUT a parked selection
                        // (sets the synchronous rep_ctx.outcome path).
                        // For an authentic "select an ally to redirect FROM" Decoy
                        // the process would call select_own_permanent first,
                        // then substitute inside the callback.
            })
            .build()]
    }
}

#[test]
fn decoy_substitutes_self_for_ally_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(decoy_card("DECOY"))
        .add_card(ally_card("ALLY"))
        .start();
    r.register_effect("DECOY", Arc::new(DecoyLike));
    let _decoy = r.place_on_field(0, "DECOY", None);
    let ally = r.place_on_field(0, "ALLY", None);

    // Delete the ally — decoy's WhenWouldBeDeleted fires (subject = ally).
    r.game.delete_permanent_with_effects(ally);

    // Outer accept dialog up.
    assert!(r.game.pending_selection.is_some());
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept");

    // Substitute outcome: ally survives, decoy is deleted in its place.
    // Re-check positions: only the ally should remain.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "exactly one permanent left after substitute"
    );
    assert_eq!(
        r.game.players[0].battle_area[0].card_id(&r.game.card_data),
        "ALLY",
        "the ally is the survivor; decoy was deleted in its place"
    );
}
```

Note: this test uses **synchronous** `substitute_replacement` (no inner select). That's intentional — Phase C's outcome-setters must work in BOTH the parked-replacement path AND the synchronous replacement path. The synchronous path writes into `rep_ctx.outcome` via the existing `ReplacementContext` mechanism, NOT into `Game.parked_replacement`. To support both, the outcome-setters need to detect which case is active. Update Tasks 2-5's implementations:

```rust
pub fn cancel_leave(&mut self) {
    if let Some(parked) = self.game.parked_replacement.as_mut() {
        parked.outcome = ReplacementOutcome::Cancelled;
    } else {
        // No parked slot — assume we're inside a synchronous replacement_process
        // closure where rep_ctx.outcome is the target. But ctx doesn't have
        // direct access to rep_ctx. The synchronous path uses rctx.cancel()
        // / rctx.handled() / etc. directly on ReplacementContext, not on
        // EffectContext. So this branch is a bug — debug_assert or fall back.
        debug_assert!(
            false,
            "cancel_leave called outside parked-replacement scope; \
             use rctx.cancel() in synchronous replacement processes"
        );
    }
}
```

Actually re-reading the spec — synchronous processes use `rctx.cancel() / handled() / redirect_to() / substitute()` (existing methods on `ReplacementContext`, see `replacement.rs:82-95`). The Phase C outcome-setters on `EffectContext` are ONLY for the parked-replacement (callback-resolved) case. The Decoy test as written above WOULD FAIL because `substitute_replacement` debug-asserts when `parked_replacement.is_none()`.

**Resolution: rewrite the Decoy test to install a select_* before the substitute** so the parked-replacement path is exercised:

Replace the Decoy `replacement_process` body with:

```rust
.replacement_process(|rctx| {
    let ally = match rctx.subject {
        ReplacementSubject::Permanent(h) => h,
        _ => return,
    };
    let me = match rctx.effect.source_permanent {
        Some(h) if h != ally => h,
        _ => return,
    };
    // Install a "confirm" prompt (single own-permanent select) so the
    // post-process hook installs parked_replacement, then substitute
    // in the callback.
    rctx.effect.select_own_permanent(
        "confirm decoy",
        false,
        move |_g, h| h == me,
        move |ctx, _picked| {
            ctx.substitute_replacement(ReplacementSubject::Permanent(me));
        },
    );
})
```

This adds a one-option "confirm" prompt that forces the parked-replacement path. The user picks the only valid option (the decoy itself), and the callback runs `substitute_replacement(decoy)`.

Update the test resolution sequence:

```rust
r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("outer accept");
// Inner confirm prompt — pick the only valid action.
let pending = r.game.pending_selection.as_ref().expect("confirm");
let action = pending.valid_action_ids[0];
let player = pending.selecting_player;
r.game.resolve_selection(player, action).expect("confirm pick");

// Now substitute should have committed.
assert_eq!(r.game.players[0].battle_area.len(), 1);
```

- [ ] **Step 3: Run the test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements decoy 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/tests/replacements/nested_select_decoy.rs digimon-engine/tests/replacements/main.rs
git commit -m "test: end-to-end nested-select Decoy (substitute outcome via parked)"
```

---

## Task 12: Regression test — Barrier/Evade/Decode unchanged

**Files:**
- Create: `digimon-engine/tests/replacements/nested_select_regression.rs`
- Modify: `digimon-engine/tests/replacements/main.rs`

Confirms the existing synchronous-process auto-installs are unaffected by Phase C's hooks. The post-process hook checks `pending_selection.is_some()` — since synchronous processes never install one, the hook short-circuits.

- [ ] **Step 1: Wire the module**

Append to `digimon-engine/tests/replacements/main.rs`:

```rust
mod nested_select_regression;
```

- [ ] **Step 2: Write the regression file**

Create `digimon-engine/tests/replacements/nested_select_regression.rs`:

```rust
//! Phase C regression — synchronous replacement_process closures (Barrier,
//! Evade, Decode auto-installs) must continue to work unchanged. The
//! post-process hook short-circuits when pending_selection.is_none().
//!
//! These tests overlap with `tests/replacements/native_keywords.rs` but
//! exist in this file as a Phase C-specific regression marker — if the
//! parked-replacement substrate ever inadvertently breaks the synchronous
//! path, this test catches it.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn with_keyword(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn barrier_synchronous_process_unchanged() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    let mut r = DebugRunner::builder()
        .add_card(with_keyword("BARRIER-D", 3000, vec![Keyword::Barrier]))
        .start();
    let b = r.place_on_field(0, "BARRIER-D", None);
    let deck_size_before = r.game.players[0].deck.len();

    r.game.delete_permanent_with_effects(b);
    assert!(r.game.pending_selection.is_some());
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept Barrier");

    // Barrier: trash top of deck, cancel deletion.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Barrier preserved digimon"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before - 1,
        "Barrier trashed top of deck"
    );
    assert!(r.game.parked_replacement.is_none(), "synchronous path leaves parked slot None");
}

#[test]
fn evade_synchronous_process_unchanged() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    let mut r = DebugRunner::builder()
        .add_card(with_keyword("EVADE-D", 3000, vec![Keyword::Evade]))
        .start();
    let e = r.place_on_field(0, "EVADE-D", None);
    let deck_size_before = r.game.players[0].deck.len();

    r.game.delete_permanent_with_effects(e);
    assert!(r.game.pending_selection.is_some());
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept Evade");

    // Evade: redirect to deck bottom.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "Evade redirected: digimon left field"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before + 1,
        "Evade put digimon at deck bottom"
    );
    assert!(r.game.parked_replacement.is_none());
}
```

(Decode test is more complex — it needs hand/deck setup. Keep this regression file focused on Barrier + Evade since those exercise both Cancelled and Redirected outcomes via the synchronous path.)

- [ ] **Step 3: Run the regression tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements regression 2>&1 | tail -10`
Expected: both tests pass.

- [ ] **Step 4: Run the full replacements suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements 2>&1 | tail -15`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/tests/replacements/nested_select_regression.rs digimon-engine/tests/replacements/main.rs
git commit -m "test: synchronous Barrier/Evade processes unchanged by Phase C hooks"
```

---

## Task 13: Remove the Task 3 limitation doc block + add Phase C reference

**Files:**
- Modify: `digimon-engine/src/combat.rs:2213-2229`

The doc block at `delete_permanent_with_cause` (lines 2213-2229 before edits — verify with grep) describes the limitation Phase C lifts. Replace with a one-line pointer to Phase C.

- [ ] **Step 1: Locate the block**

Run: `grep -n "Task 3 limitation" digimon-engine/src/combat.rs`

Expected to show the block start. Read the surrounding 20 lines.

- [ ] **Step 2: Replace the doc block**

In `digimon-engine/src/combat.rs`, find the block that begins:

```rust
    /// Task 3 limitation: if an optional replacement installs a
    /// `PendingSelection::Replacement` at EITHER dispatch stage
    /// (`WhenWouldLeaveBattleArea` or `WhenWouldBeDeleted`), this method
```

(approximately lines 2213-2229). Delete the entire `/// Task 3 limitation: ...` comment block and the `/// In practice Task 3 tests exercise mandatory replacements only;` continuation paragraph. Replace with this single line:

```rust
    /// Nested-select-in-replacement is supported via Phase C's
    /// `Game.parked_replacement` slot — see
    /// `docs/superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md`.
```

Keep the rest of the function's doc comments (the one describing the actual function shape, replacement-window semantics, etc.) intact.

- [ ] **Step 3: Verify build clean**

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/src/combat.rs
git commit -m "docs: replace Task 3 limitation comment with Phase C spec pointer"
```

---

## Task 14: Documentation updates

**Files:**
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`

- [ ] **Step 1: Add the four outcome-setters to RUST_ENGINE_API.md**

Open `docs/RUST_ENGINE_API.md`, find the `EffectContext` API section. Add a new subsection **"Replacement-process outcome-setters (Phase C)"** with one usage example each:

```markdown
### Replacement-process outcome-setters

Inside a `WhenWouldBe*` replacement-process closure, after installing a
nested player selection via `ctx.select_*`, the user's callback body
sets the replacement outcome via one of these methods:

- **`ctx.cancel_leave()`** — Cancel the original event (Save, Fragment).
  ```rust
  ctx.cancel_leave();
  ```

- **`ctx.handle_replacement()`** — Mark as custom-handled (process body
  has already mutated state; original event should be skipped).
  ```rust
  ctx.handle_replacement();
  ```

- **`ctx.redirect_replacement(zone)`** — Redirect to a different zone
  (Evade-style redirect to deck bottom).
  ```rust
  ctx.redirect_replacement(Zone::Deck);
  ```

- **`ctx.substitute_replacement(subject)`** — Substitute a different
  subject for the parked event (Decoy redirects ally-deletion to self).
  ```rust
  ctx.substitute_replacement(ReplacementSubject::Permanent(decoy_self));
  ```

All four panic in dev builds when called outside a parked-replacement scope
(`Game.parked_replacement.is_none()`). Synchronous replacement processes
that don't install a nested selection use the existing `rctx.cancel() /
handled() / redirect_to() / substitute()` methods on `ReplacementContext`.
```

- [ ] **Step 2: Mark resolved in RUST_ENGINE_GAPS.md**

Search `docs/RUST_ENGINE_GAPS.md` for `WhenWouldBeDeleted framework extensions` (or similar). Mark the row Resolved with date `2026-04-25` and the one-liner:

```markdown
RESOLVED 2026-04-25 (Phase C): nested-selection-in-replacement substrate
landed via Game.parked_replacement + EffectContext outcome-setters. Phase D
auto-installs (Save, Decoy, Fortitude, Fragment, ArmorPurge, Partition) can
now be authored. See docs/superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md.
```

If the row doesn't exist verbatim, locate the closest matching gap and update its status. Note in your report which row you updated.

- [ ] **Step 3: Update Phase C status in the parent spec**

Open `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`, find the §5 Phase C entry. Mark deliverables landed and reference the new sub-spec:

```markdown
### Phase C — Nested-selection-in-replacement substrate (§4.2)

✅ Landed 2026-04-25 on `claude/gracious-ptolemy-744e69`. Engine-only
substrate — DSL Phase 3 will consume it later. See
[Phase C sub-spec](2026-04-25-keyword-parity-phase-c-design.md).

Deliverables shipped:
- `Game.parked_replacement: Option<ParkedReplacement>` slot
- `EffectContext::cancel_leave / handle_replacement / redirect_replacement / substitute_replacement`
- Dispatcher post-process + post-callback hooks
- End-to-end test cards: Save, Fragment(2), Decoy
- Regression coverage: Barrier, Evade

Spec deviations:
- DSL Phase 3 (`lower_replacement.rs` body wiring + new step verbs) deferred —
  Phase D's hand-rolled keyword auto-installs consume the substrate directly.
- Single-outstanding-park invariant; if a real card surfaces nested-park,
  escalate to a Vec-stack.
```

- [ ] **Step 4: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md
git commit -m "docs: Phase C — outcome-setters API reference + parity tracker"
```

---

## Task 15: Final verification

No code changes; cross-surface validation. Same shape as Phase A and B's final verification.

- [ ] **Step 1: Full Rust engine test sweep**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: all green except the two pre-existing main inheritances:
- `dsl::phase0_exit::phase_0_exit_criteria` (color mismatch in YAML fixtures)
- `dsl::real_cards_json::real_adapter_all_fixtures_cross_check` (color mismatch)

If anything else fails, root-cause it. The most likely cause is an interaction between the new dispatcher hooks and an existing test scenario — flag DONE_WITH_CONCERNS.

- [ ] **Step 2: PyO3 binding rebuild**

Run: `cd digimon-engine-py && python -m maturin build --release` (or `maturin develop --release` if available)
Expected: clean. Phase C's surface changes are all on `EffectContext` (script-API, not exposed to Python) and a new internal `Game` slot. No PyO3-visible signatures changed.

- [ ] **Step 3: Python parity smoke**

Run: `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v`
Expected: 13/13 pass. Phase C is engine-internal — no observable behavior change for Python callers.

- [ ] **Step 4: Tauri tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all green.

- [ ] **Step 5: Report final status**

Use `git log --oneline -20` to summarize what landed during Phase C. No commit in this task.

---

## Self-review (run before declaring Phase C done)

- **Spec coverage:** verify each §4 component (struct, four outcome-setters, two dispatcher hooks) has a task implementing it. Verify each §5 data-flow scenario (single-pick accept, single-pick decline, multi-pick, empty-stack, substitute) has a corresponding test. Verify all §7 substrate-level tests are present.
- **No new card-side regressions:** Task 12 catches synchronous-path breakage; Task 15 step 1 catches end-to-end regressions.
- **Test cards scoped per-runner:** each test calls `r.register_effect(...)` on its own fresh `DebugRunner`, so no cross-test registry pollution. The runner's effect registry is ephemeral.
- **Outcome-setters work in BOTH parked and synchronous paths:** the spec states they're parked-only, with `debug_assert!` for misuse. The Decoy test (Task 11) validates that when a select_* IS installed, the substitute writes through correctly.
- **DSL Phase 3 deferred cleanly:** `lower_replacement.rs` stays a no-op stub; no DSL changes in this plan.
- **`Task 3 limitation` doc removed:** Task 13 deletes the block.
