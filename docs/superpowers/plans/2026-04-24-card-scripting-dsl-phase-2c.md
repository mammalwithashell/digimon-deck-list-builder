# Card Scripting DSL — Phase 2c Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lower the next wave of process-body verbs — permanent mutations (`DeletePermanent` / `ReturnToHand` / `ReturnToDeck` / `Suspend` / `Unsuspend` / `DeDigivolve`), modifier steps (`AddDpModifier` / `AddModifier` / `GrantKeyword`), and the basic control-flow forms (`Optional`, `If`) — unblocking triggered-removal, stat-pump, and conditional-branch cards that make up the single largest remaining DSL-coverable slice after 2b.

**Architecture:** All new verbs are **synchronous** (no parking) — they run inside the existing `run_step` synchronous family dispatch added in 2a, reusing the `ResolvedBinding::Permanent` path set up by 2b's `binding_ref` resolver. A new `expiry_map.rs` translates DSL expiry strings to `enums::Expiry`. Control flow reuses `run_steps` recursively so the continuation dispatcher still parks correctly if a selection step lives inside an `if` / `optional` body. `If` evaluates its predicate via the existing `dsl_cards::predicate::eval_predicate` against `ctx.as_read()` with `PredicateSubject::None`.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

**Scope (strict):**
- **Permanent mutations:** `DeletePermanent`, `ReturnToHand`, `ReturnToDeck`, `Suspend`, `Unsuspend`, `DeDigivolve`
- **Modifier steps:** `AddDpModifier`, `AddModifier` (binding-target form only), `GrantKeyword`
- **Expiry map:** new `expiry_map.rs` for DSL string → `Expiry`
- **Control flow:** `Optional`, `If`
- **End-to-end fixture:** synthetic DSL card with `select_opponent_permanent` → `if (your_turn) then delete_permanent else add_dp_modifier`, played through `DebugRunner`

**Non-goals (Phase 2d+):**
- `ForEach`, `PerSelected`, `ScheduleDelayed` (need multi-result bindings + scheduled-delayed queue wiring)
- Remaining selection kinds: `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectCountCappedMulti`, `SelectOrderedPermutation`, `SelectEffectChoice`, `AsSelectingPlayer`
- `AddModifier` with `ModifierTarget::Filter` — multi-target filter evaluation (single-target binding form only)
- Play/digivolve steps (`PlayFromHand`, `PlayFromTrash`, `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`, `PlayToken`, `Hatch`)
- `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`, `TrashTopSecurity`
- Wiring the formula evaluator into `add_dp_modifier` / `add_modifier` `value` fields (literals only in 2c)

---

## File structure

- Modify: `digimon-engine/src/dsl_cards/step/mod.rs` (register new handlers + control-flow branch)
- Create: `digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Create: `digimon-engine/src/dsl_cards/step/modifiers.rs`
- Create: `digimon-engine/src/dsl_cards/step/control_flow.rs`
- Create: `digimon-engine/src/dsl_cards/expiry_map.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (add `pub mod expiry_map;`)
- Create: `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`
- Create: `digimon-engine/tests/dsl/phase2c_modifiers.rs`
- Create: `digimon-engine/tests/dsl/phase2c_control_flow.rs`
- Create: `digimon-engine/tests/dsl/phase2c_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs` (register new test modules)

---

## Task 1: Expiry string → `Expiry` helper

Lift DSL expiry strings (`"Permanent"`, `"EndOfTurn"`, etc.) to the engine enum. Needed by Tasks 5–7.

**Files:**
- Create: `digimon-engine/src/dsl_cards/expiry_map.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/src/dsl_cards/expiry_map.rs` (skeleton) and put the test at the top:

```rust
//! Translate DSL expiry strings into engine `Expiry` enum values.

use crate::enums::Expiry;

pub fn lookup_expiry(s: &str) -> Option<Expiry> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::Expiry;

    #[test]
    fn all_variants_round_trip() {
        assert_eq!(lookup_expiry("Permanent"), Some(Expiry::Permanent));
        assert_eq!(lookup_expiry("EndOfTurn"), Some(Expiry::EndOfTurn));
        assert_eq!(lookup_expiry("EndOfOpponentsTurn"), Some(Expiry::EndOfOpponentsTurn));
        assert_eq!(lookup_expiry("EndOfAttack"), Some(Expiry::EndOfAttack));
        assert_eq!(lookup_expiry("EndOfBattle"), Some(Expiry::EndOfBattle));
        assert_eq!(lookup_expiry("UntilLeaveField"), Some(Expiry::UntilLeaveField));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(lookup_expiry("bogus"), None);
        assert_eq!(lookup_expiry(""), None);
    }
}
```

Wire the module:

```rust
// digimon-engine/src/dsl_cards/mod.rs — add alongside sibling pub mods.
pub mod expiry_map;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::expiry_map`
Expected: FAIL (both tests — stub returns `None`).

- [ ] **Step 3: Implement the mapping**

Replace the stub body:

```rust
pub fn lookup_expiry(s: &str) -> Option<Expiry> {
    Some(match s {
        "Permanent" => Expiry::Permanent,
        "EndOfTurn" => Expiry::EndOfTurn,
        "EndOfOpponentsTurn" => Expiry::EndOfOpponentsTurn,
        "EndOfAttack" => Expiry::EndOfAttack,
        "EndOfBattle" => Expiry::EndOfBattle,
        "UntilLeaveField" => Expiry::UntilLeaveField,
        _ => return None,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::expiry_map`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/expiry_map.rs digimon-engine/src/dsl_cards/mod.rs
git commit -m "dsl phase 2c: expiry_map — DSL string → Expiry enum"
```

---

## Task 2: Permanent mutation steps — `DeletePermanent` + `ReturnToHand`

First two verbs in a new `step/permanent_mutations.rs` module. Both consume a `CompiledBindingRef` resolving to a `ResolvedBinding::Permanent`.

**Files:**
- Create: `digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs`
- Create: `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test**

Create `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`:

```rust
//! Phase 2c — permanent mutation step dispatch.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::enums::PlayerId;
use digimon_engine::permanent::PermanentHandle;

fn bound_permanent_bindings(name: &str, h: PermanentHandle) -> Bindings {
    let mut b = Bindings::new();
    b.insert_permanent(name, h);
    b
}

#[test]
fn delete_permanent_via_named_binding_removes_from_battle_area() {
    let card = make_test_card("T-DEL", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::DeletePermanent {
        target: CompiledBindingRef::Named("tgt".into()),
    };
    let mut bindings = bound_permanent_bindings("tgt", handle);

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    assert_eq!(runner.game.player(PlayerId(1)).battle_area.len(), 0);
}

#[test]
fn return_to_hand_moves_permanent_to_owner_hand() {
    let card = make_test_card("T-RTH", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    let hand_before = runner.game.player(PlayerId(1)).hand.len();
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::ReturnToHand {
        target: CompiledBindingRef::Named("tgt".into()),
    };
    let mut bindings = bound_permanent_bindings("tgt", handle);

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    assert_eq!(runner.game.player(PlayerId(1)).battle_area.len(), 0);
    // Played a card (hand -1), then returned to hand (hand +1) — net 0.
    assert_eq!(runner.game.player(PlayerId(1)).hand.len(), hand_before);
}
```

Register the module:

```rust
// digimon-engine/tests/dsl/main.rs — append to module list
mod phase2c_permanent_mutations;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_permanent_mutations`
Expected: FAIL — `run_step` doesn't handle `DeletePermanent` / `ReturnToHand` yet (silently skipped).

- [ ] **Step 3: Implement the handler module**

Create `digimon-engine/src/dsl_cards/step/permanent_mutations.rs`:

```rust
//! Synchronous permanent-mutation step lowering (Phase 2c).
//!
//! Verbs: DeletePermanent, ReturnToHand, ReturnToDeck, Suspend, Unsuspend,
//! DeDigivolve. All are binding-consuming: the `target` field resolves to
//! `ResolvedBinding::Permanent`; any other variant is silently skipped (same
//! convention as the 2b zone-move handlers).

use digimon_dsl::compiled::{CompiledStackPosition, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;
use crate::enums::StackPosition;

fn map_stack_position(p: CompiledStackPosition) -> StackPosition {
    match p {
        CompiledStackPosition::Top => StackPosition::Top,
        CompiledStackPosition::Bottom => StackPosition::Bottom,
        CompiledStackPosition::Random => StackPosition::Random,
    }
}

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::DeletePermanent { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.delete_permanent(h);
            }
            true
        }
        CompiledStep::ReturnToHand { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let _ = ctx.return_to_hand(h);
            }
            true
        }
        _ => false,
    }
}
```

Wire dispatch:

```rust
// digimon-engine/src/dsl_cards/step/mod.rs — add pub mod + try_run entry.

pub mod draw;
pub mod memory;
pub mod permanent_mutations; // ← NEW
pub mod selections;
pub mod zone_moves;

// ... inside run_step():
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
    if memory::try_run(step, ctx) {
        return;
    }
    if draw::try_run(step, ctx) {
        return;
    }
    if zone_moves::try_run(step, ctx, bindings) {
        return;
    }
    if permanent_mutations::try_run(step, ctx, bindings) {
        return;
    }
    // Phase 2c+: other families.
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_permanent_mutations`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/permanent_mutations.rs \
        digimon-engine/src/dsl_cards/step/mod.rs \
        digimon-engine/tests/dsl/phase2c_permanent_mutations.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2c: DeletePermanent + ReturnToHand steps"
```

---

## Task 3: `Suspend` + `Unsuspend` + `ReturnToDeck`

Three more permanent-mutation verbs in the same handler.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`

- [ ] **Step 1: Write failing tests**

Append to `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`:

```rust
#[test]
fn suspend_then_unsuspend_round_trip() {
    let card = make_test_card("T-SUS", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let suspend = CompiledStep::Suspend { target: CompiledBindingRef::Named("tgt".into()) };
    let unsuspend = CompiledStep::Unsuspend { target: CompiledBindingRef::Named("tgt".into()) };
    let mut bindings = bound_permanent_bindings("tgt", handle);

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&suspend, ctx, &mut bindings);
        });
    assert!(runner.game.permanent(handle).unwrap().suspended);

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&unsuspend, ctx, &mut bindings);
        });
    assert!(!runner.game.permanent(handle).unwrap().suspended);
}

#[test]
fn return_to_deck_top_removes_permanent() {
    use digimon_dsl::compiled::CompiledStackPosition;

    let card = make_test_card("T-RTD", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::ReturnToDeck {
        target: CompiledBindingRef::Named("tgt".into()),
        position: CompiledStackPosition::Top,
        include_sources: false,
    };
    let mut bindings = bound_permanent_bindings("tgt", handle);

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    assert_eq!(runner.game.player(PlayerId(1)).battle_area.len(), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_permanent_mutations`
Expected: FAIL — new tests panic on no state change.

- [ ] **Step 3: Extend the handler**

Add these arms to the `match` in `try_run`:

```rust
CompiledStep::Suspend { target } => {
    if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
        ctx.suspend(h);
    }
    true
}
CompiledStep::Unsuspend { target } => {
    if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
        ctx.unsuspend(h);
    }
    true
}
CompiledStep::ReturnToDeck { target, position, include_sources: _ } => {
    // Phase 2c: `include_sources=true` is modelled in CompiledStep but engine
    // API only supports the top-card-only form. When full-stack return lands
    // (Phase 2d+), extend this arm.
    if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
        ctx.return_to_deck(h, map_stack_position(*position));
    }
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_permanent_mutations`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/permanent_mutations.rs \
        digimon-engine/tests/dsl/phase2c_permanent_mutations.rs
git commit -m "dsl phase 2c: Suspend + Unsuspend + ReturnToDeck steps"
```

---

## Task 4: `DeDigivolve` step

Pops sources off the target's digivolution stack. `amount` / `stop_at_level` are both optional; `EffectContext::de_digivolve` exposes the full signature.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`

- [ ] **Step 1: Find the `de_digivolve` signature**

Run: `grep -n "pub fn de_digivolve" digimon-engine/src/effect_context/mod.rs`

This will show the exact signature — confirm arg order is `(target, amount, stop_at_level)` or similar. Use the engine's signature verbatim in the arm below; if the signature differs, adjust the call site.

- [ ] **Step 2: Write failing test**

Append to `digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`:

```rust
#[test]
fn de_digivolve_amount_one_pops_one_source() {
    // Build a stack of 2 cards: digivolve test_card_2 onto test_card_1.
    let base = make_test_card("T-BASE", 1, "yellow");
    let evo = make_test_card("T-EVO", 2, "yellow");
    let mut runner = DebugRunner::new(
        vec![base.clone(), evo.clone(), base.clone(), base.clone(), base.clone()],
        vec![base.clone(); 5],
    );
    runner.play_from_hand(PlayerId(1), 0).unwrap();
    runner.digivolve_from_hand(PlayerId(1), 0, 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let stack_before = runner.game.permanent(handle).unwrap().sources.len();
    assert_eq!(stack_before, 2);

    let step = CompiledStep::DeDigivolve {
        target: CompiledBindingRef::Named("tgt".into()),
        amount: Some(1),
        stop_at_level: None,
    };
    let mut bindings = bound_permanent_bindings("tgt", handle);

    runner
        .game
        .with_effect_context(PlayerId(1), base.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    let stack_after = runner.game.permanent(handle).unwrap().sources.len();
    assert_eq!(stack_after, 1);
}
```

(If `DebugRunner::digivolve_from_hand` has a different name in this codebase, look it up with `grep -n "pub fn digivolve" digimon-engine/src/debug_runner.rs` and substitute. The test asserts the number of sources decrements by 1, nothing more.)

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_permanent_mutations de_digivolve_amount_one`
Expected: FAIL — stack size unchanged.

- [ ] **Step 4: Extend the handler**

Append this arm to `try_run` in `permanent_mutations.rs`:

```rust
CompiledStep::DeDigivolve { target, amount, stop_at_level } => {
    if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
        ctx.de_digivolve(h, *amount, *stop_at_level);
    }
    true
}
```

If the `EffectContext::de_digivolve` signature you observed in Step 1 differs (e.g. takes non-`Option` args with sentinel values), mirror that here — the intent is a faithful pass-through.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_permanent_mutations`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/permanent_mutations.rs \
        digimon-engine/tests/dsl/phase2c_permanent_mutations.rs
git commit -m "dsl phase 2c: DeDigivolve step"
```

---

## Task 5: `AddDpModifier` step

First modifier step. Binding-target form only.

**Files:**
- Create: `digimon-engine/src/dsl_cards/step/modifiers.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs`
- Create: `digimon-engine/tests/dsl/phase2c_modifiers.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test**

Create `digimon-engine/tests/dsl/phase2c_modifiers.rs`:

```rust
//! Phase 2c — modifier step dispatch.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::enums::PlayerId;
use digimon_engine::permanent::PermanentHandle;

fn bound(name: &str, h: PermanentHandle) -> Bindings {
    let mut b = Bindings::new();
    b.insert_permanent(name, h);
    b
}

#[test]
fn add_dp_modifier_end_of_turn_raises_effective_dp() {
    let card = make_test_card("T-DP", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let base_dp = runner.game.permanent(handle).unwrap().effective_dp(&runner.game);

    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: 3000,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = bound("tgt", handle);
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    let new_dp = runner.game.permanent(handle).unwrap().effective_dp(&runner.game);
    assert_eq!(new_dp, base_dp + 3000);
}

#[test]
fn add_dp_modifier_with_bad_expiry_is_noop() {
    let card = make_test_card("T-DP2", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let base_dp = runner.game.permanent(handle).unwrap().effective_dp(&runner.game);

    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: 9999,
        expiry: "NotARealExpiry".into(),
    };
    let mut bindings = bound("tgt", handle);
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    assert_eq!(
        runner.game.permanent(handle).unwrap().effective_dp(&runner.game),
        base_dp,
        "unknown expiry must silently no-op, matching the 2b strictness convention"
    );
}
```

Register:

```rust
// digimon-engine/tests/dsl/main.rs
mod phase2c_modifiers;
```

(If `Permanent::effective_dp` signature differs, look it up with `grep -n "fn effective_dp" digimon-engine/src/permanent.rs` and match — intent is to read post-modifier DP.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_modifiers`
Expected: FAIL — `run_step` doesn't handle `AddDpModifier`.

- [ ] **Step 3: Implement the handler module**

Create `digimon-engine/src/dsl_cards/step/modifiers.rs`:

```rust
//! Synchronous modifier-step lowering (Phase 2c).
//!
//! Verbs: AddDpModifier, AddModifier, GrantKeyword. All are binding-target
//! (filter-target form for AddModifier is Phase 2d). Unknown expiry strings,
//! unknown modifier names, or unknown keyword names cause the step to no-op —
//! same strictness convention as 2b (invalid references don't panic).

use digimon_dsl::compiled::{CompiledModifierTarget, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::expiry_map::lookup_expiry;
use crate::effect_context::EffectContext;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::AddDpModifier { target, value, expiry } => {
            let Some(expiry) = lookup_expiry(expiry) else { return true; };
            if let Some(ResolvedBinding::Permanent(h)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                ctx.add_dp_modifier(h, *value, expiry);
            }
            true
        }
        _ => false,
    }
}
```

Wire dispatch in `digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
pub mod draw;
pub mod memory;
pub mod modifiers; // ← NEW
pub mod permanent_mutations;
pub mod selections;
pub mod zone_moves;

// ... inside run_step(), after permanent_mutations:
    if modifiers::try_run(step, ctx, bindings) {
        return;
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_modifiers`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/modifiers.rs \
        digimon-engine/src/dsl_cards/step/mod.rs \
        digimon-engine/tests/dsl/phase2c_modifiers.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2c: AddDpModifier step"
```

---

## Task 6: `AddModifier` step (binding target)

Generic typed-modifier install. Binding-target form only — `ModifierTarget::Filter` is Phase 2d.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/modifiers.rs`
- Modify: `digimon-engine/tests/dsl/phase2c_modifiers.rs`

- [ ] **Step 1: Write failing test**

Append to `digimon-engine/tests/dsl/phase2c_modifiers.rs`:

```rust
#[test]
fn add_modifier_cannot_attack_blocks_attack_flag() {
    use digimon_dsl::compiled::CompiledModifierTarget;

    let card = make_test_card("T-MOD", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::AddModifier {
        target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("tgt".into())),
        modifier: "CannotAttack".into(),
        value: 0,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = bound("tgt", handle);
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    let has_cannot_attack = runner
        .game
        .modifiers
        .has_modifier(handle, digimon_engine::enums::ModifierType::CannotAttack);
    assert!(has_cannot_attack, "CannotAttack modifier must be registered");
}

#[test]
fn add_modifier_unknown_modifier_string_is_noop() {
    use digimon_dsl::compiled::CompiledModifierTarget;

    let card = make_test_card("T-MOD2", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::AddModifier {
        target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("tgt".into())),
        modifier: "NotAModifier".into(),
        value: 0,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = bound("tgt", handle);
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });
    // No panic, no registered modifier — the step must silently no-op.
}
```

(If `ModifierRegistry::has_modifier` isn't the right accessor, use `grep -n "pub fn" digimon-engine/src/modifiers.rs | grep -i has` and substitute the closest check.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_modifiers`
Expected: FAIL — new test panics on missing modifier.

- [ ] **Step 3: Extend the handler**

Add this arm to `try_run` in `modifiers.rs`:

```rust
CompiledStep::AddModifier { target, modifier, value, expiry } => {
    let Some(expiry) = lookup_expiry(expiry) else { return true; };
    let Some(modifier_ty) = crate::dsl_cards::modifier_map::lookup_modifier_type(modifier) else {
        return true;
    };
    // Phase 2c: binding-target only. Filter-target multi-targeting ships in 2d.
    let target_binding = match target {
        CompiledModifierTarget::Binding(b) => b,
        CompiledModifierTarget::Filter(_) => return true,
    };
    if let Some(ResolvedBinding::Permanent(h)) =
        resolve_binding_ref(target_binding, ctx, bindings)
    {
        ctx.add_modifier(h, modifier_ty, *value, expiry);
    }
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_modifiers`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/modifiers.rs \
        digimon-engine/tests/dsl/phase2c_modifiers.rs
git commit -m "dsl phase 2c: AddModifier step (binding target only)"
```

---

## Task 7: `GrantKeyword` step

Uses `modifier_map::lookup_keyword` + `ctx.grant_keyword`.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/modifiers.rs`
- Modify: `digimon-engine/tests/dsl/phase2c_modifiers.rs`

- [ ] **Step 1: Write failing test**

Append to `digimon-engine/tests/dsl/phase2c_modifiers.rs`:

```rust
#[test]
fn grant_keyword_blocker_is_queryable() {
    let card = make_test_card("T-GK", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::GrantKeyword {
        target: CompiledBindingRef::Named("tgt".into()),
        keyword: "Blocker".into(),
        expiry: "EndOfTurn".into(),
        value: None,
    };
    let mut bindings = bound("tgt", handle);
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });

    let has_blocker = runner
        .game
        .has_keyword(handle, digimon_engine::enums::Keyword::Blocker);
    assert!(has_blocker, "Blocker keyword must be granted");
}

#[test]
fn grant_keyword_unknown_name_is_noop() {
    let card = make_test_card("T-GK2", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    runner.play_from_hand(PlayerId(1), 0).unwrap();

    let handle = PermanentHandle { player: PlayerId(1), index: 0 };
    let step = CompiledStep::GrantKeyword {
        target: CompiledBindingRef::Named("tgt".into()),
        keyword: "NotAKeyword".into(),
        expiry: "EndOfTurn".into(),
        value: None,
    };
    let mut bindings = bound("tgt", handle);
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_step(&step, ctx, &mut bindings);
        });
    // Pass == no panic. The handler returns true (the step was recognised)
    // even though the keyword name is unknown.
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_modifiers`
Expected: FAIL on `grant_keyword_blocker_is_queryable`.

- [ ] **Step 3: Extend the handler**

Add this arm to `try_run`:

```rust
CompiledStep::GrantKeyword { target, keyword, expiry, value } => {
    let Some(expiry) = lookup_expiry(expiry) else { return true; };
    let Some(kw) = crate::dsl_cards::modifier_map::lookup_keyword(keyword, *value) else {
        return true;
    };
    if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
        ctx.grant_keyword(h, kw, expiry);
    }
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_modifiers`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/modifiers.rs \
        digimon-engine/tests/dsl/phase2c_modifiers.rs
git commit -m "dsl phase 2c: GrantKeyword step"
```

---

## Task 8: Control-flow — `Optional` wrapper

`Optional(Vec<CompiledStep>)` is the simplest control form: run the inner slice via `run_steps`. The author semantics are "player may decline the effect," but in Phase 2c the inner body always runs (the RL agent always exercises the branch). Hookable opt-out lands in 2d alongside `ScheduleDelayed`.

**Files:**
- Create: `digimon-engine/src/dsl_cards/step/control_flow.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs`
- Create: `digimon-engine/tests/dsl/phase2c_control_flow.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test**

Create `digimon-engine/tests/dsl/phase2c_control_flow.rs`:

```rust
//! Phase 2c — control-flow step lowering.

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::enums::PlayerId;

#[test]
fn optional_runs_inner_body() {
    let card = make_test_card("T-OPT", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    let memory_before = runner.game.memory;

    let steps = vec![CompiledStep::Optional(vec![CompiledStep::GainMemory(2)])];
    let mut bindings = Bindings::new();

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_steps(&steps, ctx, &mut bindings);
        });

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "Phase 2c: Optional body always runs"
    );
}
```

Register:

```rust
// digimon-engine/tests/dsl/main.rs
mod phase2c_control_flow;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_control_flow`
Expected: FAIL — `Optional` unhandled, memory unchanged.

- [ ] **Step 3: Implement the handler module**

Create `digimon-engine/src/dsl_cards/step/control_flow.rs`:

```rust
//! Control-flow step lowering (Phase 2c: Optional + If).
//!
//! These live on the `run_steps` path (not `run_step`) because their inner
//! bodies may contain selection steps that need to park the continuation.
//! The dispatcher re-enters `run_steps` for each branch.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::step::run_steps;
use crate::effect_context::EffectContext;

/// Returns `true` if the step is a control-flow verb whose body has been
/// dispatched. The caller (`run_steps`) should continue with the next step
/// at the outer level after this returns.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::Optional(body) => {
            // Phase 2c: always run the body. Opt-out UX lands in 2d.
            run_steps(body, ctx, bindings);
            true
        }
        _ => false,
    }
}
```

Wire into `run_steps` in `digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
pub mod control_flow; // ← NEW alongside siblings
pub mod draw;
pub mod memory;
pub mod modifiers;
pub mod permanent_mutations;
pub mod selections;
pub mod zone_moves;
```

Modify the `run_steps` loop so control-flow dispatches *before* selection parking (a selection inside a branch still parks normally because `run_steps` recurses):

```rust
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];

        // Control flow: run body inline (it recurses into run_steps), then
        // advance. A selection inside the body still parks the outer tail
        // as its callback via the inner run_steps call.
        if control_flow::try_run(step, ctx, bindings) {
            i += 1;
            continue;
        }

        // Selection steps install the remainder as their callback and return.
        if selections::try_install(step, &steps[i + 1..], ctx, bindings.clone()) {
            return;
        }

        // Synchronous families.
        run_step(step, ctx, bindings);
        i += 1;
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_control_flow`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/control_flow.rs \
        digimon-engine/src/dsl_cards/step/mod.rs \
        digimon-engine/tests/dsl/phase2c_control_flow.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2c: control-flow dispatcher + Optional step"
```

---

## Task 9: Control-flow — `If` step

Conditional branch. `CompiledPredicate` evaluates against `ctx.as_read()` with `PredicateSubject::None` for the game-state form. Whichever branch the predicate picks is dispatched via `run_steps`.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/control_flow.rs`
- Modify: `digimon-engine/tests/dsl/phase2c_control_flow.rs`

- [ ] **Step 1: Write failing tests**

Append to `digimon-engine/tests/dsl/phase2c_control_flow.rs`:

```rust
#[test]
fn if_true_runs_then_branch() {
    use digimon_dsl::compiled::CompiledPredicate;

    let card = make_test_card("T-IF-T", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    let memory_before = runner.game.memory;

    // Player 1's effect on player 1's turn: your_turn == true
    let cond = CompiledPredicate { your_turn: Some(true), ..CompiledPredicate::default() };
    let steps = vec![CompiledStep::If {
        condition: cond,
        then: vec![CompiledStep::GainMemory(3)],
        else_branch: vec![CompiledStep::LoseMemory(3)],
    }];
    let mut bindings = Bindings::new();

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_steps(&steps, ctx, &mut bindings);
        });

    assert_eq!(runner.game.memory, memory_before + 3);
}

#[test]
fn if_false_runs_else_branch() {
    use digimon_dsl::compiled::CompiledPredicate;

    let card = make_test_card("T-IF-F", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    let memory_before = runner.game.memory;

    // Player 1's effect on player 1's turn: opponents_turn == true is FALSE
    let cond = CompiledPredicate { opponents_turn: Some(true), ..CompiledPredicate::default() };
    let steps = vec![CompiledStep::If {
        condition: cond,
        then: vec![CompiledStep::GainMemory(100)],
        else_branch: vec![CompiledStep::GainMemory(1)],
    }];
    let mut bindings = Bindings::new();

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_steps(&steps, ctx, &mut bindings);
        });

    assert_eq!(runner.game.memory, memory_before + 1);
}

#[test]
fn if_with_empty_else_false_branch_is_noop() {
    use digimon_dsl::compiled::CompiledPredicate;

    let card = make_test_card("T-IF-E", 1, "yellow");
    let mut runner = DebugRunner::new(vec![card.clone(); 5], vec![card.clone(); 5]);
    let memory_before = runner.game.memory;

    let cond = CompiledPredicate { opponents_turn: Some(true), ..CompiledPredicate::default() };
    let steps = vec![CompiledStep::If {
        condition: cond,
        then: vec![CompiledStep::GainMemory(5)],
        else_branch: vec![], // empty else == no-op on false
    }];
    let mut bindings = Bindings::new();

    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_steps(&steps, ctx, &mut bindings);
        });

    assert_eq!(runner.game.memory, memory_before);
}
```

(If `CompiledPredicate` doesn't derive `Default` in this codebase, check `grep -n "derive.*CompiledPredicate\|impl Default for CompiledPredicate" digimon-dsl/src/compiled.rs`. If not derived, construct the predicate by setting all fields explicitly; only `your_turn` / `opponents_turn` matter for these tests.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_control_flow`
Expected: FAIL — `If` unhandled, memory unchanged for all three cases.

- [ ] **Step 3: Extend the handler**

Add the `If` arm to `try_run` in `control_flow.rs`, above the catch-all:

```rust
CompiledStep::If { condition, then, else_branch } => {
    let cond_holds = {
        let rctx = ctx.as_read();
        eval_predicate(condition, &rctx, PredicateSubject::None)
    };
    let body = if cond_holds { then } else { else_branch };
    run_steps(body, ctx, bindings);
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_control_flow`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/control_flow.rs \
        digimon-engine/tests/dsl/phase2c_control_flow.rs
git commit -m "dsl phase 2c: If control-flow step"
```

---

## Task 10: End-to-end fixture — selection + if + permanent mutation

Synthetic DSL card exercising the full new surface through `DebugRunner`. Shape:

```
select_opponent_permanent  as: tgt
if your_turn:
  then: delete_permanent tgt
  else: add_dp_modifier tgt +3000 EndOfTurn
```

This proves the continuation dispatcher from 2b still works when the parked tail contains an `if` branch that mutates the selected permanent.

**Files:**
- Create: `digimon-engine/tests/dsl/phase2c_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test**

Create `digimon-engine/tests/dsl/phase2c_end_to_end.rs`:

```rust
//! Phase 2c end-to-end: select_opponent_permanent → if → delete_permanent.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledPlayerRef, CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::enums::PlayerId;

#[test]
fn select_then_if_your_turn_deletes_opponent_permanent() {
    let card = make_test_card("T-E2E", 1, "yellow");
    let target_card = make_test_card("T-TGT", 1, "red");
    let mut runner = DebugRunner::new(
        vec![card.clone(); 5],
        vec![target_card.clone(); 5],
    );

    // Put a permanent onto player 2's battle area.
    runner.play_from_hand(PlayerId(2), 0).unwrap();
    assert_eq!(runner.game.player(PlayerId(2)).battle_area.len(), 1);

    // Ensure it's player 1's turn so your_turn == true from P1's perspective.
    // DebugRunner starts on P1's turn by default; if not, advance here.
    assert_eq!(runner.game.turn_player(), PlayerId(1));

    let cond_your_turn = CompiledPredicate {
        your_turn: Some(true),
        ..CompiledPredicate::default()
    };
    let steps = vec![
        CompiledStep::SelectOpponentPermanent {
            filter: CompiledPredicate::default(),
            bind_as: Some("tgt".into()),
            prompt: "pick".into(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::If {
            condition: cond_your_turn,
            then: vec![CompiledStep::DeletePermanent {
                target: CompiledBindingRef::Named("tgt".into()),
            }],
            else_branch: vec![CompiledStep::AddDpModifier {
                target: CompiledBindingRef::Named("tgt".into()),
                value: 3000,
                expiry: "EndOfTurn".into(),
            }],
        },
    ];
    let mut bindings = Bindings::new();

    // Kick off the selection. The continuation parks; resolve it via the
    // DebugRunner pending-selection API.
    runner
        .game
        .with_effect_context(PlayerId(1), card.handle(), None, |ctx| {
            run_steps(&steps, ctx, &mut bindings);
        });

    // Resolve the pending selection: pick opponent's permanent at index 0.
    runner.resolve_pending_permanent_selection(PlayerId(1), 0);

    // Expected: opponent's battle area is empty (DeletePermanent ran).
    assert_eq!(
        runner.game.player(PlayerId(2)).battle_area.len(),
        0,
        "your_turn==true → then branch (DeletePermanent) must fire"
    );
}
```

Register:

```rust
// digimon-engine/tests/dsl/main.rs
mod phase2c_end_to_end;
```

(Look up the exact `DebugRunner` pending-selection resolver name with `grep -n "resolve_pending\|pub fn.*pending.*selection\|pub fn.*selection" digimon-engine/src/debug_runner.rs` and substitute. The 2b `phase2b_end_to_end.rs` file already uses the right API — match it.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_end_to_end`
Expected: Either FAIL (if the end-to-end path was broken) or PASS (if the prior tasks covered it end-to-end). If FAIL, diagnose which step is silently no-op'ing before moving on.

- [ ] **Step 3: Fix forward if needed**

If the test fails, the most likely cause is one of:
1. `ctx.as_read()` returns a context whose `player` field is wrong when called from inside a selection callback — fix by threading `ctx.player` through the callback if needed.
2. The `delete_permanent` call doesn't decrement the opponent's battle area because `target.player` was captured as the wrong player — check `ResolvedBinding::Permanent` came from `SelectOpponentPermanent` which should have inserted a `PermanentHandle` with `player = opponent_id()`. Verify by printing the bound handle in the callback.

Resolve the root cause (no workarounds; the Working Rules in CLAUDE.md forbid script-level workarounds when the engine supports the behavior).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2c_end_to_end`
Expected: PASS (1 test).

- [ ] **Step 5: Run the full DSL test suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl`
Expected: all prior phase tests (phase0/1a/1b/1c/2a/2b) still pass alongside the new phase2c_* tests.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/tests/dsl/phase2c_end_to_end.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2c: end-to-end — select + if + delete_permanent round-trip"
```

---

## Self-Review

Spec-coverage sweep against spec §3.7 (mutation verbs) — Phase 2c target subset:

- §3.7.3 Field / permanent: `DeletePermanent` (T2), `ReturnToHand` (T2), `ReturnToDeck` (T3), `Suspend` (T3), `Unsuspend` (T3), `DeDigivolve` (T4). All present. `PlaceOnSecurity` / `PlayToken` / `PlaceAsBottomSource` / `TrashTopSource` / `Hatch` are explicit non-goals (Phase 2d+).
- §3.7.6 Modifiers: `AddDpModifier` (T5), `AddModifier` (T6), `GrantKeyword` (T7). `AddModifier` filter-target is explicit non-goal.
- §3.7.8 Control flow: `Optional` (T8), `If` (T9). `ForEach` / `PerSelected` / `ScheduleDelayed` are explicit non-goals.
- §3.11 Expiry catalogue: all six variants covered by `lookup_expiry` (T1).
- End-to-end proves continuation parking from 2b still works when a selection's tail contains a control-flow branch that consumes the bound permanent (T10).

Placeholder scan: no `TBD`, `implement later`, or "add appropriate X" — every step contains concrete code or a concrete command. The lookup-the-signature instructions in T4 / T6 / T10 are bounded investigations (single `grep` + substitute), not open-ended work.

Type consistency: `ResolvedBinding::Permanent` used consistently across permanent_mutations.rs + modifiers.rs; `lookup_expiry` return shape (`Option<Expiry>`) matched at every call site; `run_steps` recursion path is the single inlet into control-flow bodies so selection parking works uniformly.

Plan complete and saved to [docs/superpowers/plans/2026-04-24-card-scripting-dsl-phase-2c.md](docs/superpowers/plans/2026-04-24-card-scripting-dsl-phase-2c.md). Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
