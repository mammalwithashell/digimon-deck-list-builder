# Card Scripting DSL — Standalone Declaratives (Replacement / Partition / Delay)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Lower three remaining declarative clause shapes that are each self-contained (no binding environment, no selection steps, no play-cost surgery):
1. `CompiledDeclarativeClause::Replacement` — "Would*" replacement effects (Evade / redirect / cancel hooks).
2. `CompiledDeclarativeClause::Partition` — register source-exclusion rules (AD1-025 Omnimon's Partition source list).
3. `CompiledDeclarativeClause::Delay` — install a Delay Option body (end-of-next-turn firing).

Each clause type maps to an already-existing engine `Effect` constructor; the work is pure lowering + predicate/step plumbing.

**Architecture:** Three small lowering modules under `digimon-engine/src/dsl_cards/` — `lower_replacement.rs`, `lower_partition.rs`, `lower_delay.rs`. Each adds a match arm to `DslCardEffect::effects()` dispatch. Replacement and Delay reuse the `run_step` dispatcher shipped in Phase 2a; Partition is a one-shot modifier install (no process steps).

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl`.

---

## Task 1: Replacement clause lowering

**Compiled shape (`digimon-dsl/src/compiled.rs`):**

```rust
CompiledDeclarativeClause::Replacement {
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    trigger: String,             // "when_would_be_deleted", "when_would_draw", ...
    process: Vec<CompiledStep>,  // process body — uses ReplacementContext helpers
    summary: Option<String>,
    summary_key: Option<String>,
}
```

**Engine surface:**
- `Effect::when_would_be_deleted(card)`, `when_would_leave_battle_area`, `when_would_be_returned_to_hand`, `when_would_be_returned_to_deck`, `when_would_be_trashed`, `when_would_be_de_digivolved`, `when_would_lose_security`, `when_would_draw`, `when_would_place_in_security` — all in `digimon-engine/src/effect.rs:280-306`.
- `EffectBuilder::replacement_process(Fn(&mut ReplacementContext))` — `effect.rs:509`.
- `crate::replacement::ReplacementContext` exposes `.cancel()`, `.redirect(target)`, `.substitute(...)`, `.handled()`.

**Phase 1 scope:** Lower the Effect with an *empty* replacement_process closure that just records the trigger match. Full replacement process bodies require `run_step` lifted into `ReplacementContext` (separate plan) — Phase 1 here just gets the shape plumbed so tests can assert "this card has a Would* effect registered."

### Files
- Create: `digimon-engine/src/dsl_cards/lower_replacement.rs`
- Create: `digimon-engine/src/dsl_cards/trigger_map.rs` — maps trigger string → `EffectTiming::WhenWould*`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (dispatch arm)
- Test: `digimon-engine/tests/dsl/replacement.rs`

### Task 1.1: trigger_map

```rust
//! Map replacement trigger strings → EffectTiming.
use crate::enums::EffectTiming;

pub fn lookup_replacement_trigger(s: &str) -> Option<EffectTiming> {
    Some(match s {
        "when_would_be_deleted" => EffectTiming::WhenWouldBeDeleted,
        "when_would_leave_battle_area" => EffectTiming::WhenWouldLeaveBattleArea,
        "when_would_be_returned_to_hand" => EffectTiming::WhenWouldBeReturnedToHand,
        "when_would_be_returned_to_deck" => EffectTiming::WhenWouldBeReturnedToDeck,
        "when_would_be_trashed" => EffectTiming::WhenWouldBeTrashed,
        "when_would_be_de_digivolved" => EffectTiming::WhenWouldBeDeDigivolved,
        "when_would_lose_security" => EffectTiming::WhenWouldLoseSecurity,
        "when_would_draw" => EffectTiming::WhenWouldDraw,
        "when_would_place_in_security" => EffectTiming::WhenWouldPlaceInSecurity,
        _ => return None,
    })
}
```

### Task 1.2: lower_replacement — emit Effect with empty replacement_process

```rust
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<CompiledPredicate>, // Phase 2 gating
    trigger: &str,
    _process: &[CompiledStep],
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let mut builder = new_when_would_builder(card, timing)?;
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder = builder.replacement_process(|_rctx| {
        // Phase 1: no-op body. Phase 2 wires run_step into ReplacementContext.
    });
    Some(builder.build())
}

fn new_when_would_builder(card: CardHandle, timing: EffectTiming) -> Option<EffectBuilder> {
    Some(match timing {
        EffectTiming::WhenWouldBeDeleted => Effect::when_would_be_deleted(card),
        EffectTiming::WhenWouldLeaveBattleArea => Effect::when_would_leave_battle_area(card),
        EffectTiming::WhenWouldBeReturnedToHand => Effect::when_would_be_returned_to_hand(card),
        EffectTiming::WhenWouldBeReturnedToDeck => Effect::when_would_be_returned_to_deck(card),
        EffectTiming::WhenWouldBeTrashed => Effect::when_would_be_trashed(card),
        EffectTiming::WhenWouldBeDeDigivolved => Effect::when_would_be_de_digivolved(card),
        EffectTiming::WhenWouldLoseSecurity => Effect::when_would_lose_security(card),
        EffectTiming::WhenWouldDraw => Effect::when_would_draw(card),
        EffectTiming::WhenWouldPlaceInSecurity => Effect::when_would_place_in_security(card),
        _ => return None,
    })
}
```

### Task 1.3: Tests

Assert that a Replacement clause with `trigger: "when_would_be_deleted"` emits exactly one Effect whose `.timing == WhenWouldBeDeleted` and `.replacement_process.is_some()`.

Assert unknown trigger strings → empty emission.

### Task 1.4: Dispatch arm in `DslCardEffect::effects()`

```rust
CompiledDeclarativeClause::Replacement { scope, active_when, trigger, process, .. } => {
    if let Some(e) = lower_replacement::lower(card, *scope, active_when.clone(), trigger, process) {
        out.push(e);
    }
}
```

Commit: `dsl: lower Replacement declarative clause (Would* effects with empty body)`

---

## Task 2: Partition clause lowering

**Compiled shape:**

```rust
CompiledDeclarativeClause::Partition {
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    sources: Vec<CompiledPredicate>,     // e.g. "WarGreymon", "MetalGarurumon"
    exclude_cause: Vec<String>,          // "own_effect", "battle"
    summary: Option<String>,
    summary_key: Option<String>,
}
```

**Engine surface:** `Keyword::Partition` already exists. Partition semantics: the card is treated as an exclusion point for the listed sources against the listed causes. Phase 1 scope: grant the `Keyword::Partition` keyword + install a declarative marker — full source-list enforcement lives in engine replacement dispatch and is orthogonal to this lowering.

### Files
- Create: `digimon-engine/src/dsl_cards/lower_partition.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (dispatch arm)
- Test: `digimon-engine/tests/dsl/partition.rs`

### Implementation sketch

```rust
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<CompiledPredicate>,
    _sources: Vec<CompiledPredicate>,
    _exclude_cause: Vec<String>,
) -> Effect {
    let mut builder = Effect::declarative(card)
        .name("Partition")
        .process(move |ctx| {
            let Some(handle) = ctx.source_permanent else { return; };
            ctx.grant_keyword(handle, Keyword::Partition, Expiry::Permanent);
        });
    if matches!(scope, CompiledScope::Inherited) { builder = builder.inherited(); }
    builder.build()
}
```

### Tests

Assert AD1-025 fixture's Partition clause emits a declarative Effect with a process closure. `process.is_some()` is the shape check; full behavior testing requires engine replacement dispatch + source-list enforcement.

### Dispatch

```rust
CompiledDeclarativeClause::Partition { scope, active_when, sources, exclude_cause, .. } => {
    out.push(lower_partition::lower(card, *scope, active_when.clone(), sources.clone(), exclude_cause.clone()));
}
```

Commit: `dsl: lower Partition declarative clause (Keyword::Partition grant)`

---

## Task 3: Delay clause lowering

**Compiled shape:**

```rust
CompiledDeclarativeClause::Delay {
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    trigger: CompiledTiming,    // e.g. EndOfYourTurn
    process: Vec<CompiledStep>,
    summary: Option<String>,
    summary_key: Option<String>,
}
```

**Engine surface:**
- `EffectBuilder::delay(DelayTrigger)` — `effect.rs:529`, sets timing to `EffectTiming::DelayEffect`.
- `DelayTrigger::{EndOfYourNextTurn, EndOfThisTurn}` — `enums.rs:258`.

**Phase 1 scope:** Map `CompiledTiming::EndOfYourTurn` → `DelayTrigger::EndOfThisTurn`; default other timings to `EndOfYourNextTurn`. Body runs through `run_step` (Phase 2a dispatcher).

### Files
- Create: `digimon-engine/src/dsl_cards/lower_delay.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (dispatch arm)
- Test: `digimon-engine/tests/dsl/delay.rs`

### Implementation sketch

```rust
use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use crate::enums::{DelayTrigger, EffectTiming};

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: Vec<CompiledStep>,
) -> Effect {
    let delay_trigger = match trigger {
        CompiledTiming::EndOfYourTurn => DelayTrigger::EndOfThisTurn,
        _ => DelayTrigger::EndOfYourNextTurn,
    };
    let process_arc = std::sync::Arc::new(process_steps);
    let mut builder = Effect::option_main_builder_stub(card)  // see note
        .delay(delay_trigger)
        .process(move |ctx| {
            let mut bindings = crate::dsl_cards::bindings::Bindings::new();
            for s in process_arc.iter() {
                crate::dsl_cards::step::run_step(s, ctx, &mut bindings);
            }
        });
    if matches!(scope, CompiledScope::Inherited) { builder = builder.inherited(); }
    builder.build()
}
```

**Note:** There's no `option_main_builder_stub` — use `EffectBuilder::new(card, EffectTiming::DelayEffect)` then chain `.delay(trigger)` (delay() already sets timing). Check `effect.rs:529` for the real builder entry point; adapt the sketch.

### Tests

Assert a Delay clause with `trigger: end_of_your_turn` emits an Effect with `timing == DelayEffect` and `delay_trigger == Some(EndOfThisTurn)`. Assert process closure runs when invoked.

### Dispatch

```rust
CompiledDeclarativeClause::Delay { scope, active_when, trigger, process, .. } => {
    out.push(lower_delay::lower(card, *scope, active_when.clone(), *trigger, process.clone()));
}
```

Commit: `dsl: lower Delay declarative clause (DelayEffect with DelayTrigger)`

---

## Task 4: Smoke exit test

After all three lowerings land, write an exit test that constructs one synthetic `CompiledCard` per clause type (Replacement / Partition / Delay) and asserts each produces at least one Effect with the expected timing/shape. This catches regressions in future dispatch-arm edits.

Commit: `dsl: standalone-declaratives smoke exit test`

---

## Self-Review

**Spec coverage:**
- Replacement — Task 1 (Would* timings surface)
- Partition — Task 2 (Keyword::Partition grant)
- Delay — Task 3 (DelayEffect with DelayTrigger mapping)
- Exit test — Task 4

**Deferrals:**
- Replacement full process body needs `run_step` lifted into `ReplacementContext` (separate plan)
- Partition source-list enforcement is engine-side (replacement dispatch hook)
- Delay step body uses existing `run_step` — any steps beyond Phase 2a scope silently no-op

**Conflict with other worktrees:** each lowering touches a distinct match arm in `DslCardEffect::effects()`. Merge is a one-line addition per lowering.

**Type consistency:** `lookup_replacement_trigger(&str) -> Option<EffectTiming>`, `lower_*::lower(CardHandle, CompiledScope, ...) -> Effect` or `Option<Effect>`.
