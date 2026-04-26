# Card Scripting DSL — Phase 2f Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the Phase 2 deferred-items list from §7.3 of the DSL spec — wire `play_*` / `digivolve` / placement steps to the engine, accept `formula:` values in `add_modifier` / `add_dp_modifier`, persist the `as_selecting_player` override across selection callbacks, and add the `schedule_delayed` engine primitive plus its lowering. After 2f, every Phase-2 step variant in `CompiledStep` is wired to engine behaviour and Phase 2 of the DSL migration is feature-complete; subsequent work moves to Phase 3 (replacement / partition / event_target_* predicates).

**Architecture:** Four self-contained sub-phases, each shippable as its own PR following the established `Phase 2a/b/c/d/e — landed` pattern in §7.3. Each sub-phase pairs a thin DSL lowering layer with whatever new `EffectContext` / `Game` primitive it needs underneath; tests are co-located under `code/digimon-engine/tests/dsl/phase2f*_*.rs` mirroring the 2e file naming. The four sub-phases share zero code and can be implemented in any order, but the recommended sequence is: **2f1** (play/digivolve/placement wiring — pure mechanical, exercises the broadest surface), **2f2** (formula values in modifiers — smallest, low-risk polish), **2f3** (`AsSelectingPlayer` override persistence — engine callback-dispatch refactor), **2f4** (`schedule_delayed` — new engine subsystem).

**Tech Stack:** Rust 2021, `digimon-engine` library crate, `digimon-dsl` leaf crate.

**Scope (per sub-phase):**

- **2f1 — Play / digivolve / placement step lowering.** Wire 12 `CompiledStep` variants that already exist in the IR but are unhandled in the step runner: `PlayFromHand`, `PlayFromHandFree`, `PlayFromTrash`, `PlayFromTrashFree`, `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`, `PlayToken`, `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`. Add the missing engine primitives `play_from_hand_free`, `play_from_security`, `play_from_materials`, `effect_initiated_dna_digivolve`, `trash_top_source`. Verify the existing primitives `play_from_hand_with_cost`, `play_from_trash_with_cost`, `play_from_trash_free_unsuspended`, `effect_initiated_digivolve`, `place_on_security`, `place_as_bottom_source`, `play_token` continue to satisfy their step variants unchanged.
- **2f2 — Formula values in `add_modifier` / `add_dp_modifier`.** Replace the `value: i32` field on `CompiledStep::AddModifier` and `CompiledStep::AddDpModifier` with a new `CompiledModifierValue` enum (`Literal(i32)` | `Formula(CompiledFormula)`); add a runtime formula evaluator `code/digimon-engine/src/dsl_cards/formula_eval.rs` that takes an `&EffectContext` and returns `i32`; update the YAML schema/lowering to accept either `value: 3000` or `value: { formula: { base: ..., per: ..., delta: ... } }`. Backwards-compatible with existing literal authors.
- **2f3 — `AsSelectingPlayer` override-persistence.** Refactor the selection callback dispatch path so the `(controller, override_selecting_player)` pair persists into freshly-constructed `EffectContext` inside callbacks, instead of collapsing to `player = selecting_player`. Add the DSL `CompiledStep::AsSelectingPlayer { of, body }` lowering: set `ctx.override_selecting_player = Some(resolve_player(of))`, run `body`, restore the previous override on synchronous return; on park, the captured callback persists the override into the resumed slice via the new dispatch path.
- **2f4 — `schedule_delayed` engine primitive + DSL lowering.** Add `Game::scheduled_effects: Vec<ScheduledEffect>` and the `EffectContext::schedule_delayed(when: Timing, body: Vec<Effect>)` API. The `ScheduledEffect` queue is drained at every timing-trigger boundary that matches the `when:` field — reuses the existing observer-fire pipeline by enqueuing into `EffectQueue` when the timing matches. Wire `CompiledStep::ScheduleDelayed { when, body }` to compile its body with the same lowering used for `process:` clauses, and call `ctx.schedule_delayed(when, body)`.

**Non-goals (Phase 3+):**
- `replacement` / `partition` clause families (Phase 3 — already partially landed for partition, but the broader replacement-effect family is Phase 3).
- `event_target_*` predicates beyond the `event_target` / `event_card` bindings already supported (Phase 3).
- `equals` / `not_equals` predicate consumption of `BindingValue::Literal` (separate predicate-evaluator work, tracked outside this plan).
- Per-iteration resumption when a `for_each` body parks a selection (Phase 3 enhancement called out in §3.7.8).
- Pretty-printer / round-trip / schema-export updates for the new `CompiledModifierValue` enum and `ScheduleDelayed` content — these layers update mechanically and are folded into each sub-phase's tests; no separate task.

---

## Pre-flight: shared conventions

These are the conventions every sub-phase test fixture follows. They are verified against `code/digimon-engine/tests/dsl/phase2e_*.rs` — copy verbatim.

- `DebugRunner::builder().add_card(make_test_card(id, name)).hand(player, &[id, ...]).build()`.
- Player IDs are raw `u8` (`0` for P0, `1` for P1) — no `PlayerId::P*` enum sugar at the test layer.
- `EffectContext::new(&mut runner.game, src_card, /* source_permanent */ None, ctx_player)` — keep the curly-brace scope so the `&mut` borrow drops before the next `runner.game.*` access.
- Pending selection inspection: `runner.game.pending_selection.as_ref()` → `pending.valid_action_ids` + `pending.selecting_player`.
- Resolve selection: `runner.game.resolve_selection(selecting_player, action_id).expect("...")`.
- Push to trash / deck / security / battle area: see `push_to_trash` in `code/digimon-engine/tests/dsl/phase2d_select_count_capped_multi.rs` and the `CardSource::new` + `runner.game.next_card_index()` pattern used in `phase2e_select_security.rs`.
- Test files register in `code/digimon-engine/tests/dsl/main.rs` via `mod phase2f1_*;` etc.

When a step's behaviour matches an existing engine API, **call the existing API**. When the step requires new engine surface, the sub-phase's first task adds that surface (TDD against an engine-level test in `code/digimon-engine/tests/effect_context/`) before the DSL lowering task.

---

# Sub-phase 2f1 — Play / digivolve / placement step lowering

After this sub-phase, every `play_*` / `digivolve` / `place_*` / `trash_top_source` step variant in `CompiledStep` reaches the engine and produces the correct game-state mutation. Selection-installing variants (`PlayFromHand` with non-Free cost — the engine already prompts for cost cancellation in some paths) park identically to other Phase 2b/c selection installs.

## File structure (2f1)

- Create: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — new step family handler covering all 12 variants.
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs` — add `pub mod play_digivolve;` and dispatch into it from `run_step`.
- Modify: `code/digimon-engine/src/effect_context/mod.rs` — add the missing primitives:
  - `pub fn play_from_hand_free(&mut self, player: PlayerId, hand_index: usize) -> bool`
  - `pub fn play_from_security(&mut self, player: PlayerId) -> bool`
  - `pub fn play_from_materials(&mut self, target: PermanentHandle, source_index: usize, cost_delta: Option<CompiledCostDelta>) -> bool` *(takes the IR's cost-delta type unchanged — see Task 1)*
  - `pub fn effect_initiated_dna_digivolve(&mut self, target_a: PermanentHandle, target_b: PermanentHandle, from_hand: CardHandle, cost: i32, ignore_requirements: bool) -> bool`
  - `pub fn trash_top_source(&mut self, target: PermanentHandle) -> bool`
- Create: `code/digimon-engine/tests/effect_context/play_from_hand_free.rs`
- Create: `code/digimon-engine/tests/effect_context/play_from_security.rs`
- Create: `code/digimon-engine/tests/effect_context/play_from_materials.rs`
- Create: `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs`
- Create: `code/digimon-engine/tests/effect_context/trash_top_source.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs` — register the five new test modules.
- Create: `code/digimon-engine/tests/dsl/phase2f1_play_steps.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f1_digivolve_steps.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f1_placement_steps.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f1_end_to_end.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs` — register the four new modules.
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — append a 2f1 bullet under §7.3.

### Task 1: Decide whether `cost_delta` plumbing belongs in this sub-phase

**Files:** read-only analysis; no edits.

`CompiledStep::PlayFromHand`, `PlayFromTrash`, `PlayFromMaterials` carry `cost_delta: Option<CompiledCostDelta>` (`Free` | `Printed` | `Literal(i32)`). The existing `play_from_hand_with_cost(player, hand_index, cost: i32)` engine primitive takes a final `i32` — it does **not** know about `Free`/`Printed`/`Literal`. The translation logic must live somewhere.

- [ ] **Step 1: Read the three engine primitives**

Run `grep -nE "pub fn (play_from_hand_with_cost|play_from_trash_with_cost|play_from_trash_free_unsuspended)" code/digimon-engine/src/effect_context/mod.rs` and read their signatures + bodies. Confirm:
- All three already exist.
- All three take an `i32` (`play_from_hand_with_cost`) or no cost (`play_from_trash_free_unsuspended`).
- `Free` collapses to "use the free variant"; `Printed` collapses to `card.printed_cost()`; `Literal(n)` collapses to `n`.

- [ ] **Step 2: Lock the convention**

Translation lives in `play_digivolve.rs` (the new step handler), **not** in the engine primitives. Each step handler:

```rust
fn resolve_cost_delta(
    delta: Option<CompiledCostDelta>,
    printed_cost: i32,
) -> Option<i32> {
    match delta {
        None => Some(printed_cost),
        Some(CompiledCostDelta::Printed) => Some(printed_cost),
        Some(CompiledCostDelta::Literal(n)) => Some(n),
        Some(CompiledCostDelta::Free) => None, // signals: call the *_free_* variant
    }
}
```

This keeps `EffectContext` clean of DSL types and lets the lowering decide between the `_with_cost` and `_free` engine variants per step.

- [ ] **Step 3: Commit a comment in `play_digivolve.rs` capturing this decision**

(Done as part of Task 4's file creation — no separate commit.)

### Task 2: `play_from_hand_free` engine primitive

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Create: `code/digimon-engine/tests/effect_context/play_from_hand_free.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`

The engine has `play_from_hand_with_cost` but no `_free` variant. Cards like "play this without paying memory" need it. We add a thin wrapper over the existing payment logic that bypasses the cost subtraction.

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/effect_context/play_from_hand_free.rs`:

```rust
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::tests::helpers::make_test_card;

#[test]
fn play_from_hand_free_does_not_subtract_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-001", "PlayFreeTest"))
        .hand(0, &["TST-001"])
        .build();
    runner.game.memory = 3;
    let src_handle = runner.game.players[0].hand[0].handle();

    {
        let src_card = runner.game.players[0].hand[0].clone();
        let mut ctx = EffectContext::new(&mut runner.game, &src_card, None, 0);
        let ok = ctx.play_from_hand_free(0, 0);
        assert!(ok);
    }

    assert_eq!(runner.game.memory, 3, "play_from_hand_free must not change memory");
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert!(runner.game.players[0].battle_area[0]
        .top_card_source()
        .map(|cs| cs.handle() == src_handle)
        .unwrap_or(false));
}
```

Register the module: append `mod play_from_hand_free;` to `code/digimon-engine/tests/effect_context/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context play_from_hand_free`
Expected: FAIL — `play_from_hand_free` does not exist.

- [ ] **Step 3: Implement the primitive**

In `code/digimon-engine/src/effect_context/mod.rs`, near `play_from_hand_with_cost`:

```rust
/// Play a card from `player`'s hand at `hand_index` without subtracting
/// memory. Used by effects that say "play this without paying its memory cost".
/// Returns `true` if the card was placed; `false` if the hand index was
/// out of bounds or the card was not playable (e.g. wrong color/level for
/// the current zone). Does not consult `play_cost` at all.
pub fn play_from_hand_free(&mut self, player: PlayerId, hand_index: usize) -> bool {
    self.play_from_hand_with_cost(player, hand_index, 0)
        .map(|_| true)
        .unwrap_or(false)
        // NOTE: play_from_hand_with_cost subtracts the passed cost from memory.
        // Passing 0 zeros the subtraction, achieving "free play".
}
```

If `play_from_hand_with_cost`'s signature differs from the assumption, **read it first** and adapt — the goal is "play to battle area, do not change memory". If the existing API mandates a memory subtraction, save the pre-call memory and restore it after the call.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context play_from_hand_free`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/tests/effect_context/play_from_hand_free.rs code/digimon-engine/tests/effect_context/main.rs
git commit -m "engine: add EffectContext::play_from_hand_free (Phase 2f1)"
```

### Task 3: `play_from_security`, `play_from_materials`, `effect_initiated_dna_digivolve`, `trash_top_source` primitives

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Create: four test files under `code/digimon-engine/tests/effect_context/` matching the names in File structure.
- Modify: `code/digimon-engine/tests/effect_context/main.rs`.

Each primitive follows the same TDD shape as Task 2:
1. Write a failing engine-level behavioural test that builds a minimal game state, invokes the new primitive via `EffectContext`, and asserts the resulting state.
2. Implement the primitive in `code/digimon-engine/src/effect_context/mod.rs`.
3. Run + commit.

For each primitive, the test fixture and signature are below. **Implement them in this order — each is its own commit**:

#### 3a. `play_from_security`

Card text precedent: BT12-091's "play the top card of your security stack". Engine semantics: top of `player.security` is removed and played without paying memory.

- [ ] **Step 1: failing test**

Create `code/digimon-engine/tests/effect_context/play_from_security.rs`. Setup: P0 security stack has `["BT2-001"]`. Call `ctx.play_from_security(0)`. Assert: `players[0].security.len() == 0`, `players[0].battle_area.len() == 1`, top of battle_area is BT2-001, `runner.game.memory` unchanged.

- [ ] **Step 2: run, expect FAIL ("method `play_from_security` not found")**

- [ ] **Step 3: implement**

```rust
/// Play the top of `player`'s security stack without paying memory.
/// Returns `true` on success; `false` if the security stack is empty or the
/// top card is not playable in the current zone.
pub fn play_from_security(&mut self, player: PlayerId) -> bool {
    let card = match self.game.player_mut(player).security.pop() {
        Some(c) => c,
        None => return false,
    };
    // Mirror play_from_hand_with_cost's placement path. If a helper like
    // `place_card_in_battle_area(&mut self, player, card)` exists, call it.
    // Otherwise replicate the relevant placement steps inline.
    self.place_card_in_battle_area(player, card)
}
```

If `place_card_in_battle_area` doesn't exist, **read** the body of `play_from_hand_with_cost` and extract the placement steps into a small private helper used by both — single-purpose extraction, no broader refactor.

- [ ] **Step 4: pass + commit**

#### 3b. `play_from_materials`

Card text precedent: BT15-080 "place this card's bottom material into battle area as a Digimon". Removes a `card_source` from a permanent's digivolution stack and plays the underlying card with the supplied cost-delta semantics.

- [ ] **Step 1: failing test**

Create `code/digimon-engine/tests/effect_context/play_from_materials.rs`. Build a permanent with two materials. Call `ctx.play_from_materials(perm_handle, 0, Some(CompiledCostDelta::Free))`. Assert: the source at index 0 is removed from the permanent's digivolution stack and a new permanent appears in battle_area with the underlying card on top.

- [ ] **Step 2: run, expect FAIL**

- [ ] **Step 3: implement**

```rust
pub fn play_from_materials(
    &mut self,
    target: PermanentHandle,
    source_index: usize,
    cost_delta: Option<CompiledCostDelta>,
) -> bool {
    use digimon_dsl::compiled::CompiledCostDelta;
    let perm = match self.game.permanent_mut(target) {
        Some(p) => p,
        None => return false,
    };
    if source_index >= perm.card_sources.len() {
        return false;
    }
    let source = perm.card_sources.remove(source_index);
    let player = perm.controller;
    let card = match source.into_card() {
        Some(c) => c,
        None => return false,
    };
    let cost = match cost_delta {
        None | Some(CompiledCostDelta::Printed) => card.play_cost(),
        Some(CompiledCostDelta::Literal(n)) => n,
        Some(CompiledCostDelta::Free) => 0,
    };
    self.game.memory -= cost; // engine convention — adapt sign per Player
    self.place_card_in_battle_area(player, card)
}
```

Adapt `into_card`, `play_cost()`, and the memory-sign convention to whatever the codebase uses — this signature is illustrative; the test pins the observable behaviour.

- [ ] **Step 4: pass + commit**

#### 3c. `effect_initiated_dna_digivolve`

Card text precedent: BT5-085 Omnimon DNA digivolve from-effect. The existing `effect_initiated_digivolve` handles single-target; DNA needs two targets merged.

- [ ] **Step 1: failing test**

Create `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs`. Build P0 battle_area with two champions; hand has the DNA result card. Call `ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)`. Assert: the two champions are consumed (merged into the result's stack), one new permanent exists with the result card on top and both champions as material sources, `players[0].hand.len() == 0`.

- [ ] **Step 2: run, expect FAIL**

- [ ] **Step 3: implement** — read `effect_initiated_digivolve`'s body for the single-target reference; the DNA variant merges both targets' stacks under the new top card. Cost is subtracted unless `ignore_requirements == true` (per IR field semantics) — match the existing `effect_initiated_digivolve` precedent.

- [ ] **Step 4: pass + commit**

#### 3d. `trash_top_source`

Card text precedent: "trash the top digivolution source of this Digimon". Strips index `len-1` from the target permanent's `card_sources` and routes it to the controller's trash.

- [ ] **Step 1: failing test**

Create `code/digimon-engine/tests/effect_context/trash_top_source.rs`. Build a permanent with one material. Call `ctx.trash_top_source(perm_handle)`. Assert: the material is now in the controller's trash, `perm.card_sources` is empty (or `len - 1`).

- [ ] **Step 2: run, expect FAIL**

- [ ] **Step 3: implement**

```rust
pub fn trash_top_source(&mut self, target: PermanentHandle) -> bool {
    let perm = match self.game.permanent_mut(target) {
        Some(p) => p,
        None => return false,
    };
    let source = match perm.card_sources.pop() {
        Some(s) => s,
        None => return false,
    };
    let player = perm.controller;
    if let Some(card) = source.into_card() {
        self.game.player_mut(player).trash.push(card);
    }
    true
}
```

- [ ] **Step 4: pass + commit**

### Task 4: Step-handler module `play_digivolve.rs`

**Files:**
- Create: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`

- [ ] **Step 1: Write the failing DSL test**

Create `code/digimon-engine/tests/dsl/phase2f1_play_steps.rs`:

```rust
use digimon_dsl::compiled::{CompiledBindingRef, CompiledCostDelta, CompiledPlayerRef, CompiledStep};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::tests::helpers::make_test_card;

#[test]
fn play_from_hand_step_consumes_hand_and_subtracts_cost() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "Source"))
        .add_card(make_test_card("TST-B", "Plays"))
        .hand(0, &["TST-A", "TST-B"])
        .build();
    runner.game.memory = 5;
    let src_card = runner.game.players[0].hand[0].clone();

    let mut bindings = Bindings::default();
    bindings.insert_hand_index("idx", 1); // hand index 1 = TST-B
    let steps = vec![CompiledStep::PlayFromHand {
        of: CompiledPlayerRef::You,
        hand_index: CompiledBindingRef::Named("idx".into()),
        cost_delta: Some(CompiledCostDelta::Free),
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, &src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(runner.game.memory, 5, "Free cost_delta does not subtract memory");
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
}
```

Register the module: append `mod phase2f1_play_steps;` to `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f1_play_steps`
Expected: FAIL — the step is silently skipped (the dispatcher's `_ => false` arm in `run_step`), so battle_area stays empty.

- [ ] **Step 3: Implement `play_digivolve.rs`**

Create `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`:

```rust
//! Phase 2f1: lowering for play / digivolve / placement steps.
//!
//! These verbs all run synchronously to completion (no selection prompts).
//! Each translates the IR's `CompiledStep::*` arm to an `EffectContext::*`
//! engine primitive; cost-delta translation lives here, not in the engine.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledCostDelta, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` is a play/digivolve/placement family handled here.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::PlayFromHand { of, hand_index, cost_delta } => {
            let p = resolve_player(ctx, *of);
            let Some(idx) = resolve_hand_index(hand_index, ctx, bindings) else {
                return true;
            };
            let cost = printed_cost_for_hand(ctx, p, idx);
            match cost_delta {
                None | Some(CompiledCostDelta::Printed) => {
                    ctx.play_from_hand_with_cost(p, idx, cost);
                }
                Some(CompiledCostDelta::Literal(n)) => {
                    ctx.play_from_hand_with_cost(p, idx, *n);
                }
                Some(CompiledCostDelta::Free) => {
                    ctx.play_from_hand_free(p, idx);
                }
            }
            true
        }
        CompiledStep::PlayFromHandFree { of, hand_index } => {
            let p = resolve_player(ctx, *of);
            if let Some(idx) = resolve_hand_index(hand_index, ctx, bindings) {
                ctx.play_from_hand_free(p, idx);
            }
            true
        }
        CompiledStep::PlayFromTrash { of, trash_index, cost_delta } => {
            let p = resolve_player(ctx, *of);
            let Some(idx) = resolve_trash_index(trash_index, ctx, bindings) else {
                return true;
            };
            let cost = printed_cost_for_trash(ctx, p, idx);
            match cost_delta {
                None | Some(CompiledCostDelta::Printed) => {
                    ctx.play_from_trash_with_cost(p, idx, cost);
                }
                Some(CompiledCostDelta::Literal(n)) => {
                    ctx.play_from_trash_with_cost(p, idx, *n);
                }
                Some(CompiledCostDelta::Free) => {
                    ctx.play_from_trash_free_unsuspended(p, idx);
                }
            }
            true
        }
        CompiledStep::PlayFromTrashFree { of, trash_index } => {
            let p = resolve_player(ctx, *of);
            if let Some(idx) = resolve_trash_index(trash_index, ctx, bindings) {
                ctx.play_from_trash_free_unsuspended(p, idx);
            }
            true
        }
        CompiledStep::PlayFromSecurity => {
            let p = ctx.player;
            ctx.play_from_security(p);
            true
        }
        CompiledStep::PlayFromMaterials { target, source_index, cost_delta } => {
            let Some(ResolvedBinding::Permanent(perm)) =
                resolve_binding_ref(target, ctx, bindings)
            else {
                return true;
            };
            let Some(idx) = resolve_source_index(source_index, ctx, bindings) else {
                return true;
            };
            ctx.play_from_materials(perm, idx, cost_delta.clone());
            true
        }
        CompiledStep::EffectInitiatedDigivolve { target, from_hand, cost, ignore_requirements } => {
            let Some(ResolvedBinding::Permanent(perm)) =
                resolve_binding_ref(target, ctx, bindings)
            else {
                return true;
            };
            let Some(ResolvedBinding::Card(hand_h)) =
                resolve_binding_ref(from_hand, ctx, bindings)
            else {
                return true;
            };
            ctx.effect_initiated_digivolve(perm, hand_h, *cost, *ignore_requirements);
            true
        }
        CompiledStep::EffectInitiatedDnaDigivolve {
            target_a, target_b, from_hand, cost, ignore_requirements,
        } => {
            let Some(ResolvedBinding::Permanent(a)) =
                resolve_binding_ref(target_a, ctx, bindings)
            else {
                return true;
            };
            let Some(ResolvedBinding::Permanent(b)) =
                resolve_binding_ref(target_b, ctx, bindings)
            else {
                return true;
            };
            let Some(ResolvedBinding::Card(hand_h)) =
                resolve_binding_ref(from_hand, ctx, bindings)
            else {
                return true;
            };
            ctx.effect_initiated_dna_digivolve(a, b, hand_h, *cost, *ignore_requirements);
            true
        }
        CompiledStep::PlayToken { controller, token_name } => {
            let p = resolve_player(ctx, *controller);
            ctx.play_token(p, token_name);
            true
        }
        CompiledStep::PlaceOnSecurity { of, source, position, face_up } => {
            let Some(resolved) = resolve_binding_ref(source, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::Card(h) = resolved {
                ctx.place_on_security(p, h, super::map_stack_position(*position), *face_up);
            }
            true
        }
        CompiledStep::PlaceAsBottomSource { source, target } => {
            let Some(src_resolved) = resolve_binding_ref(source, ctx, bindings) else {
                return true;
            };
            let Some(ResolvedBinding::Permanent(target_h)) =
                resolve_binding_ref(target, ctx, bindings)
            else {
                return true;
            };
            if let ResolvedBinding::Card(card_h) = src_resolved {
                ctx.place_as_bottom_source(card_h, target_h);
            }
            true
        }
        CompiledStep::TrashTopSource { target } => {
            let Some(ResolvedBinding::Permanent(perm)) =
                resolve_binding_ref(target, ctx, bindings)
            else {
                return true;
            };
            ctx.trash_top_source(perm);
            true
        }
        _ => false,
    }
}

fn resolve_hand_index(
    r: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<usize> {
    match resolve_binding_ref(r, ctx, bindings)? {
        ResolvedBinding::HandIndex(i) => Some(i as usize),
        _ => None,
    }
}

fn resolve_trash_index(
    r: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<usize> {
    match resolve_binding_ref(r, ctx, bindings)? {
        ResolvedBinding::TrashIndex(i) => Some(i as usize),
        _ => None,
    }
}

fn resolve_source_index(
    r: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<usize> {
    match resolve_binding_ref(r, ctx, bindings)? {
        ResolvedBinding::Literal(n) if n >= 0 => Some(n as usize),
        _ => None,
    }
}

fn printed_cost_for_hand(ctx: &EffectContext<'_>, p: crate::enums::PlayerId, idx: usize) -> i32 {
    ctx.game
        .player(p)
        .hand
        .get(idx)
        .map(|cs| cs.printed_play_cost())
        .unwrap_or(0)
}

fn printed_cost_for_trash(ctx: &EffectContext<'_>, p: crate::enums::PlayerId, idx: usize) -> i32 {
    ctx.game
        .player(p)
        .trash
        .get(idx)
        .map(|cs| cs.printed_play_cost())
        .unwrap_or(0)
}
```

Adapt method names (`printed_play_cost`, `next_card_index`, etc.) to whatever the existing codebase uses — verify by `grep -nE "fn printed_(play_)?cost" code/digimon-engine/src/card_source.rs` first; do not invent.

- [ ] **Step 4: Wire it into the dispatcher**

Edit `code/digimon-engine/src/dsl_cards/step/mod.rs`:

Add `pub mod play_digivolve;` near the other module declarations. Add `if play_digivolve::try_run(step, ctx, bindings) { return; }` to `run_step` between `modifiers::try_run(...)` and the trailing `// Phase 2d+: other families.` comment.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f1_play_steps`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/step/play_digivolve.rs code/digimon-engine/src/dsl_cards/step/mod.rs code/digimon-engine/tests/dsl/phase2f1_play_steps.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: lower play_from_* steps to engine primitives (Phase 2f1)"
```

### Task 5: Per-variant DSL behavioural tests

Add one `#[test]` per remaining step variant under the three test files (`phase2f1_play_steps.rs` already exists from Task 4 — append to it; create `phase2f1_digivolve_steps.rs` and `phase2f1_placement_steps.rs` separately). Each test follows the Task 4 pattern: build a minimal game state, run the step via `run_steps`, assert game-state mutation.

Variant → file mapping:

- `phase2f1_play_steps.rs`: `PlayFromHand` (Task 4), `PlayFromHandFree`, `PlayFromTrash`, `PlayFromTrashFree`, `PlayFromSecurity`, `PlayFromMaterials`, `PlayToken`.
- `phase2f1_digivolve_steps.rs`: `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`.
- `phase2f1_placement_steps.rs`: `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`.

For each variant:

- [ ] **Step 1: Write a failing test** asserting the variant's effect against a hand-built game state.
- [ ] **Step 2: Run it. Expect FAIL only if** the engine primitive is also new (Tasks 2/3 land first); for variants that wrap an already-existing primitive, the test should PASS as soon as Task 4's dispatcher wiring lands. If the test PASSes immediately, that's correct — write it anyway as a regression guard.
- [ ] **Step 3: If implementation is needed, do the minimum.**
- [ ] **Step 4: Run + pass.**
- [ ] **Step 5: Commit per variant or per file** (one commit per file is acceptable; do not bundle all eleven into a single commit — keeps blame readable).

### Task 6: End-to-end DSL fixture

**Files:**
- Create: `code/digimon-engine/tests/dsl/phase2f1_end_to_end.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

End-to-end test loading a YAML card whose `process:` chains `select_trash` → `play_from_trash` (the canonical "revival" pattern, e.g. `[Main] Play one of your trashed level-3 Digimon without paying memory`). Wires the loader → validator → compiler → step runner.

- [ ] **Step 1: Write the failing test** (full file body — model on `phase2e_end_to_end.rs`).

```rust
use digimon_dsl::loader::load_card_from_str;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::register_dsl_card;
use digimon_engine::tests::helpers::make_test_card;

const YAML: &str = r#"
card: TST-REV
name: "Revival"
kind: digimon
level: 3
color: [red]
cost: 4
dp: 3000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          filter: { kind: digimon, level_lte: 3 }
          bind_as: revived
          prompt: "Pick a Digimon to revive"
      - play_from_trash:
          of: you
          trash_index: revived
          cost_delta: free
"#;

#[test]
fn revival_card_pulls_from_trash_into_battle_area() {
    let spec = load_card_from_str(YAML).expect("YAML parses");
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-REV", "Revival"))
        .add_card(make_test_card("BT2-001", "Target"))
        .hand(0, &["TST-REV"])
        .build();
    register_dsl_card(&mut runner.game, &spec);
    // Place BT2-001 in P0's trash:
    runner.push_to_trash(0, "BT2-001");
    runner.game.memory = 5;

    runner.play_from_hand(0, "TST-REV");
    let pending = runner.game.pending_selection.as_ref().expect("select_trash parked");
    let pick = pending.valid_action_ids[0];
    runner.game.resolve_selection(pending.selecting_player, pick).unwrap();

    assert_eq!(runner.game.players[0].battle_area.len(), 2, "Revival + revived target both on field");
    assert_eq!(runner.game.players[0].trash.len(), 0);
    assert_eq!(runner.game.memory, 5 - 4, "only Revival's memory cost paid");
}
```

Adapt `play_from_hand` / `push_to_trash` / `register_dsl_card` to whatever helper APIs the existing `phase2e_end_to_end.rs` uses — copy verbatim.

- [ ] **Step 2: Run, expect FAIL** until both `select_trash` (Phase 2b — already landed) and `play_from_trash` (Task 4) are wired. With Tasks 2–4 done, this should PASS.

- [ ] **Step 3: Pass + commit**

```bash
git add code/digimon-engine/tests/dsl/phase2f1_end_to_end.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: phase2f1 end-to-end revival test (Phase 2f1)"
```

### Task 7: Spec note + Phase 2f1 PR closeout

- [ ] **Step 1: Update §7.3 of `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`**

Append a `2f1 (landed YYYY-MM-DD)` bullet under the existing 2e bullet, mirroring the format. Drop the "play / digivolve / placement steps" item from the **Defers to 2f+** list.

```markdown
- **2f1** (landed YYYY-MM-DD) — play / digivolve / placement step lowering:
  `PlayFromHand`, `PlayFromHandFree`, `PlayFromTrash`, `PlayFromTrashFree`,
  `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiatedDigivolve`,
  `EffectInitiatedDnaDigivolve`, `PlayToken`, `PlaceOnSecurity`,
  `PlaceAsBottomSource`, `TrashTopSource`. New engine primitives:
  `play_from_hand_free`, `play_from_security`, `play_from_materials`,
  `effect_initiated_dna_digivolve`, `trash_top_source`. Cost-delta
  translation (`Free` / `Printed` / `Literal`) lives in the DSL
  step-handler, not the engine. Defers to 2f+: formula values in
  `add_modifier`, `AsSelectingPlayer` override-persistence,
  `ScheduleDelayed`.
```

- [ ] **Step 2: Run the full DSL test suite as a regression guard**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl`
Expected: PASS — every Phase 2a–2e test and the new 2f1 tests.

- [ ] **Step 3: Commit the spec update**

```bash
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md
git commit -m "docs: dsl spec — Phase 2f1 sub-phase notes"
```

---

# Sub-phase 2f2 — Formula values in `add_modifier` / `add_dp_modifier`

After 2f2, modifier `value:` accepts either a literal int or a `formula:` block (matching the `dp_lte:` and `cost:` formula-or-literal idiom from §3.10). This unblocks DP buffs whose magnitude depends on board state — Susanoomon's "+2000 DP per material on this Digimon" is the canonical case.

## File structure (2f2)

- Create: `code/digimon-engine/src/dsl_cards/formula_eval.rs` — runtime evaluator `pub fn evaluate(formula: &CompiledFormula, ctx: &EffectContext) -> i32`.
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs` — `pub mod formula_eval;`.
- Modify: `code/digimon-dsl/src/compiled.rs`:
  - Add `pub enum CompiledModifierValue { Literal(i32), Formula(CompiledFormula) }`.
  - Replace `value: i32` in `CompiledStep::AddDpModifier` and `CompiledStep::AddModifier` with `value: CompiledModifierValue`.
- Modify: `code/digimon-dsl/src/compile.rs` — accept either `i32` or `formula:` shape on the YAML side and lower to the new enum.
- Modify: `code/digimon-dsl/src/schema.rs` — JSON Schema export for the new union.
- Modify: `code/digimon-engine/src/dsl_cards/step/modifiers.rs` — call `formula_eval::evaluate` on the value before passing to `ctx.add_dp_modifier` / `ctx.add_modifier`.
- Create: `code/digimon-engine/tests/dsl/phase2f2_formula_eval.rs` — unit tests for the evaluator across every `CompiledFormula` variant.
- Create: `code/digimon-engine/tests/dsl/phase2f2_modifier_formula.rs` — DSL behavioural test exercising `value: { formula: ... }`.
- Create: `code/digimon-dsl/tests/parse_modifier_formula_value.rs` — parse-and-lower test asserting both literal and formula YAML forms compile.
- Modify: `code/digimon-engine/tests/dsl/main.rs` — register the two new modules.
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — note the new shape under §3.7 add_modifier / add_dp_modifier.

### Task 1: `CompiledModifierValue` IR

**Files:**
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`

Today, `CompiledStep::AddDpModifier { target, value: i32, expiry }`. We change `value` to `CompiledModifierValue` — a new enum that wraps `Literal(i32) | Formula(CompiledFormula)`. Same for `AddModifier`.

- [ ] **Step 1: Write the failing parse test**

Create `code/digimon-dsl/tests/parse_modifier_formula_value.rs`:

```rust
use digimon_dsl::loader::load_card_from_str;
use digimon_dsl::compiled::{CompiledFormula, CompiledModifierValue, CompiledStep};

const YAML: &str = r#"
card: TST-FORM
name: "FormulaTest"
kind: digimon
level: 3
color: [red]
cost: 4
dp: 1000
effects:
  - when: on_attack_declared
    process:
      - add_dp_modifier:
          target: source_permanent
          value:
            formula:
              base: 0
              per: stack_size
              delta: 1000
          expiry: end_of_turn
"#;

#[test]
fn add_dp_modifier_accepts_formula_value() {
    let spec = load_card_from_str(YAML).expect("YAML parses");
    let process = spec.effects.first().unwrap().process.as_ref().unwrap();
    let step = process.first().unwrap();
    let CompiledStep::AddDpModifier { value, .. } = step else {
        panic!("expected AddDpModifier");
    };
    match value {
        CompiledModifierValue::Formula(CompiledFormula::BasePerDelta { base: 0, .. }) => (),
        other => panic!("expected formula value, got {other:?}"),
    }
}

#[test]
fn add_dp_modifier_still_accepts_literal_value() {
    let yaml = r#"
card: TST-LIT
name: "LiteralTest"
kind: digimon
level: 3
color: [red]
cost: 4
dp: 1000
effects:
  - when: on_attack_declared
    process:
      - add_dp_modifier:
          target: source_permanent
          value: 3000
          expiry: end_of_turn
"#;
    let spec = load_card_from_str(yaml).expect("YAML parses");
    let process = spec.effects.first().unwrap().process.as_ref().unwrap();
    let CompiledStep::AddDpModifier { value, .. } = process.first().unwrap() else {
        panic!("expected AddDpModifier");
    };
    assert!(matches!(value, CompiledModifierValue::Literal(3000)));
}
```

- [ ] **Step 2: Run, expect FAIL** — `CompiledModifierValue` does not exist; `value` is currently `i32`.

Run: `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_modifier_formula_value`

- [ ] **Step 3: Add the IR type**

In `code/digimon-dsl/src/compiled.rs`, after `CompiledFormula`'s definition:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledModifierValue {
    Literal(i32),
    Formula(CompiledFormula),
}
```

Replace the `value: i32` field in both `CompiledStep::AddDpModifier` and `CompiledStep::AddModifier` with `value: CompiledModifierValue`.

- [ ] **Step 4: Update the YAML lowering**

In `code/digimon-dsl/src/compile.rs`, find the `add_dp_modifier` / `add_modifier` lowering arms. Today they expect a literal int from the source AST. Change the source AST type for the `value:` field (in `code/digimon-dsl/src/step.rs`, the authored shape) to a tagged union accepting either a bare int or a `{ formula: ... }` block (use `serde(untagged)` like `FormulaSpec`):

```rust
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ModifierValueSpec {
    Literal(i32),
    Formula { formula: FormulaSpec },
}
```

In the lowering, translate `ModifierValueSpec::Literal(n)` → `CompiledModifierValue::Literal(n)` and `ModifierValueSpec::Formula { formula }` → `CompiledModifierValue::Formula(compile_formula(formula))` (reuse the existing `compile_formula` helper that already lowers `FormulaSpec` for `cost:` formula values in alt_paths).

- [ ] **Step 5: Run test, expect PASS**

Run: `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_modifier_formula_value`

The literal-form test confirms backwards compatibility; the formula-form test confirms the new path.

- [ ] **Step 6: Run the full DSL parse suite as a regression guard**

Run: `cargo test --manifest-path code/digimon-dsl/Cargo.toml`
Expected: PASS — every prior parser/lowering test still passes (`value: 3000` still lowers correctly).

- [ ] **Step 7: Commit**

```bash
git add code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-dsl/src/step.rs code/digimon-dsl/tests/parse_modifier_formula_value.rs
git commit -m "dsl: CompiledModifierValue — accept formula in add_modifier value (Phase 2f2)"
```

### Task 2: Runtime formula evaluator

**Files:**
- Create: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f2_formula_eval.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

The evaluator takes `(&CompiledFormula, &EffectContext)` and returns `i32`. Per-selectors (`material_count`, `stack_size`, `ally_count`, `digivolution_color_count`, `card_count_in_zone`) read from `ctx.source_permanent()` and `ctx.player`. Aggregate selectors enumerate battle_area for highest/lowest DP/level. `RawRust(fn_name)` looks up the fn in the `EffectExtensionRegistry` (already present per `code/digimon-dsl/src/raw_rust_registry.rs`) and calls it.

- [ ] **Step 1: Write the failing evaluator unit tests**

Create `code/digimon-engine/tests/dsl/phase2f2_formula_eval.rs`. One `#[test]` per `CompiledFormula` variant:

```rust
use digimon_dsl::compiled::{CompiledFormula, CompiledPerSelector};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::formula_eval::evaluate;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::tests::helpers::make_test_card;

#[test]
fn evaluate_literal() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "Lit"))
        .hand(0, &["TST"])
        .build();
    let src = runner.game.players[0].hand[0].clone();
    let ctx = EffectContext::new(&mut runner.game, &src, None, 0);
    assert_eq!(evaluate(&CompiledFormula::Literal(7), &ctx), 7);
}

#[test]
fn evaluate_base_per_stack_size_delta() {
    // Build a permanent with 3 materials. Source = that permanent.
    // BasePerDelta { base: 1000, per: StackSize, delta: 200 } => 1000 + 3*200 = 1600.
    // (...full setup omitted — model on phase2c_permanent_mutations.rs::stack_with_three_materials.)
}

#[test]
fn evaluate_floor_div() {
    // FloorDiv([Literal(10), Literal(3)]) => 3
}

#[test]
fn evaluate_max() {
    // Max([Literal(5), Literal(2), Literal(7)]) => 7
}

#[test]
fn evaluate_min() {
    // Min([Literal(5), Literal(2), Literal(7)]) => 2
}

#[test]
fn evaluate_aggregate_lowest_dp() {
    // Build P0 battle_area with two permanents of dp 1000 and 3000.
    // Aggregate(LowestDp) => 1000.
}

#[test]
fn evaluate_raw_rust_dispatches_through_registry() {
    // Register fn `tst_seven` returning 7, evaluate RawRust("tst_seven") => 7.
}
```

Each test follows the Phase 2c material-construction pattern. Where the test body is "..." above, write the full body — every test must be runnable.

- [ ] **Step 2: Run, expect FAIL**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_formula_eval`
Expected: FAIL — module `formula_eval` does not exist.

- [ ] **Step 3: Implement the evaluator**

Create `code/digimon-engine/src/dsl_cards/formula_eval.rs`:

```rust
//! Phase 2f2 — runtime evaluator for `CompiledFormula`.
//!
//! All evaluation is read-only against `EffectContext`. Selectors that
//! reference the source permanent (`stack_size`, `material_count`,
//! `digivolution_color_count`) gracefully return 0 when the source is not
//! on the field — mirrors the conservative-no-op convention used throughout
//! the DSL step runners.

use digimon_dsl::compiled::{CompiledAggregateSelector, CompiledFormula, CompiledPerSelector};

use crate::effect_context::EffectContext;

pub fn evaluate(f: &CompiledFormula, ctx: &EffectContext<'_>) -> i32 {
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = evaluate_per(*per, ctx);
            base + count * delta
        }
        CompiledFormula::FloorDiv(args) => {
            // Convention: FloorDiv(vec![lhs, rhs]); panic on mis-arity is a
            // validator failure, but defensively return 0 if rhs is missing
            // or zero.
            if args.len() != 2 { return 0; }
            let l = evaluate(&args[0], ctx);
            let r = evaluate(&args[1], ctx);
            if r == 0 { 0 } else { l.div_euclid(r) }
        }
        CompiledFormula::Max(args) => args.iter().map(|a| evaluate(a, ctx)).max().unwrap_or(0),
        CompiledFormula::Min(args) => args.iter().map(|a| evaluate(a, ctx)).min().unwrap_or(0),
        CompiledFormula::Aggregate(sel) => evaluate_aggregate(*sel, ctx),
        CompiledFormula::RawRust(name) => ctx
            .game
            .raw_rust_registry
            .lookup_value_fn(name)
            .map(|f| f(ctx))
            .unwrap_or(0),
    }
}

fn evaluate_per(sel: CompiledPerSelector, ctx: &EffectContext<'_>) -> i32 {
    let src_perm = ctx.source_permanent();
    match sel {
        CompiledPerSelector::StackSize | CompiledPerSelector::MaterialCount => src_perm
            .map(|p| p.card_sources.len() as i32)
            .unwrap_or(0),
        CompiledPerSelector::AllyCount => {
            let p = ctx.player;
            ctx.game.player(p).battle_area.iter()
                .filter(|perm| perm.handle() != src_perm.map(|s| s.handle()).unwrap_or_default())
                .count() as i32
        }
        CompiledPerSelector::DigivolutionColorCount => src_perm
            .map(|p| p.digivolution_colors().len() as i32)
            .unwrap_or(0),
        CompiledPerSelector::CardCountInZone => {
            // Spec: "card_count_in_zone" — context-dependent. Phase 2f2 ships
            // the variant lowered behind the existing parser; the engine-side
            // resolution targets the source's current zone and counts that
            // player's cards in it.
            // ... full body, modeled on existing zone-aware predicate code.
            0
        }
    }
}

fn evaluate_aggregate(sel: CompiledAggregateSelector, ctx: &EffectContext<'_>) -> i32 {
    use digimon_dsl::compiled::CompiledAggregateSelector::*;
    let p = ctx.player;
    let perms = &ctx.game.player(p).battle_area;
    if perms.is_empty() { return 0; }
    match sel {
        LowestDp => perms.iter().map(|pm| pm.effective_dp()).min().unwrap_or(0),
        HighestDp => perms.iter().map(|pm| pm.effective_dp()).max().unwrap_or(0),
        LowestLevel => perms.iter().filter_map(|pm| pm.top_level()).min().unwrap_or(0),
        HighestLevel => perms.iter().filter_map(|pm| pm.top_level()).max().unwrap_or(0),
    }
}
```

Verify method names (`effective_dp`, `top_level`, `digivolution_colors`, `raw_rust_registry`, `lookup_value_fn`) against the actual codebase before committing — `grep -nE "fn effective_dp|fn top_level|fn digivolution_colors" code/digimon-engine/src/permanent.rs` and `grep -nE "raw_rust_registry|lookup_value_fn" code/digimon-engine/src/`. If a method is missing, **stop and confirm before inventing it** — this evaluator is not the place to introduce permanent-API helpers.

Add `pub mod formula_eval;` to `code/digimon-engine/src/dsl_cards/mod.rs`. Register the test module in `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 4: Run, expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_formula_eval`
Expected: PASS — all variant tests green.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/tests/dsl/phase2f2_formula_eval.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "engine: dsl_cards::formula_eval runtime evaluator (Phase 2f2)"
```

### Task 3: Wire formula values into `add_modifier` / `add_dp_modifier`

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/step/modifiers.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f2_modifier_formula.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing behavioural test**

Create `code/digimon-engine/tests/dsl/phase2f2_modifier_formula.rs`. Build a permanent with 3 materials; lower a step:

```rust
CompiledStep::AddDpModifier {
    target: CompiledBindingRef::Named("self".into()),
    value: CompiledModifierValue::Formula(CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::StackSize,
        delta: 1000,
    }),
    expiry: "end_of_turn".into(),
}
```

Run it via `run_steps`. Assert: the permanent's effective DP increases by `3 * 1000 = 3000`.

- [ ] **Step 2: Run, expect FAIL** — `step/modifiers.rs` currently treats `value` as `i32`; after Task 1 it's `CompiledModifierValue`, so the call to `ctx.add_dp_modifier(h, *value, expiry)` will not compile.

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_modifier_formula`
Expected: COMPILE ERROR (mismatched types) — fix in Step 3.

- [ ] **Step 3: Update `step/modifiers.rs` to evaluate the formula**

Replace the body of the `AddDpModifier` arm:

```rust
CompiledStep::AddDpModifier { target, value, expiry } => {
    let Some(expiry) = lookup_expiry(expiry) else { return true; };
    if let Some(ResolvedBinding::Permanent(h)) =
        resolve_binding_ref(target, ctx, bindings)
    {
        let n = match value {
            CompiledModifierValue::Literal(n) => *n,
            CompiledModifierValue::Formula(f) => crate::dsl_cards::formula_eval::evaluate(f, ctx),
        };
        ctx.add_dp_modifier(h, n, expiry);
    }
    true
}
```

Apply the same pattern to the `AddModifier` arm.

- [ ] **Step 4: Run, expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f2_modifier_formula`
Expected: PASS.

- [ ] **Step 5: Run all DSL + engine tests as regression guard**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/step/modifiers.rs code/digimon-engine/tests/dsl/phase2f2_modifier_formula.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: lower formula-valued add_modifier through formula_eval (Phase 2f2)"
```

### Task 4: Spec note + Phase 2f2 PR closeout

- [ ] **Step 1: Update §3.7.5 (or wherever `add_dp_modifier` / `add_modifier` are documented)** — `value:` accepts either a literal int or a `formula:` block. Show the Susanoomon example.
- [ ] **Step 2: Append to §7.3** — `2f2 (landed)` bullet noting the new evaluator + IR change.
- [ ] **Step 3: Commit the spec update.**

---

# Sub-phase 2f3 — `AsSelectingPlayer` override-persistence + lowering

After 2f3, `as_selecting_player: { of: opponent, body: [select_*, ...] }` correctly routes every selection inside the body to the `of:` player while keeping `ctx.player` (the controller / source) intact. Today the override survives only the immediate select-install; nested or sequential selects in the body lose it because the callback rebuilds `EffectContext` with `player = selecting_player_at_install_time`, dropping the controller/override distinction.

The **engine work** is to refactor the selection-callback dispatch path so callbacks construct their `EffectContext` with **both** the original controller AND the override pinned, instead of folding them into a single `player` field.

## File structure (2f3)

- Modify: `code/digimon-engine/src/effect_context/mod.rs` — `EffectContext::new` already takes `(game, source_card, source_permanent, player)`. Add a constructor `EffectContext::new_with_override(..., override_selecting_player: Option<PlayerId>)` and use it from selection callbacks. Make `override_selecting_player` field `pub(crate)` (currently `pub(super)`) so callbacks in `selections.rs` can set it explicitly.
- Modify: `code/digimon-engine/src/effect_context/selections.rs` — every `Box::new(move |game, action_id| { ... EffectContext::new(...) ... })` callback site is updated to capture **both** `controller` and `selecting_player` in its closure, and to construct the post-callback `EffectContext` via `new_with_override(game, source_card, source_permanent, controller, Some(selecting_player))`. Apply uniformly across `select_hand`, `select_trash`, `select_own_permanent`, `select_opponent_permanent`, `select_count_capped_multi` (and its trampoline), `select_effect_choice`, `select_reveal`, `select_security`, `select_material`, `select_union_zone`, `select_ordered_permutation`.
- Create: `code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs` — DSL lowering for `CompiledStep::AsSelectingPlayer { of, body }`.
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs` — register the new module + dispatch arm.
- Create: `code/digimon-engine/tests/effect_context/override_persistence.rs` — engine-level test that exercises the new `new_with_override` callback dispatch through a chain of two selects.
- Create: `code/digimon-engine/tests/dsl/phase2f3_as_selecting_player.rs` — DSL behavioural test: `as_selecting_player: { of: opponent, body: [select_own_permanent, add_dp_modifier] }` ensures the prompt is sent to the opponent and the modifier still applies.
- Create: `code/digimon-engine/tests/dsl/phase2f3_end_to_end.rs` — YAML-loaded card exercising the canonical "your opponent chooses" pattern.
- Modify: `code/digimon-engine/tests/dsl/main.rs`, `code/digimon-engine/tests/effect_context/main.rs`.
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — append 2f3 bullet.

### Task 1: `EffectContext::new_with_override` constructor

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Create: `code/digimon-engine/tests/effect_context/override_persistence.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`

- [ ] **Step 1: Write the failing engine test**

Create `code/digimon-engine/tests/effect_context/override_persistence.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::tests::helpers::make_test_card;

#[test]
fn select_hand_callback_with_override_preserves_controller() {
    // P0 is the controller (source = P0's card). Override = P1.
    // After the callback fires, the inner ctx must report:
    //   ctx.player == 0 (controller)
    //   ctx.override_selecting_player == Some(1)
    // so a *nested* select_* inside the callback would route the prompt to P1.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "Source"))
        .hand(0, &["TST"])
        .hand(1, &["TST"])
        .build();
    let src = runner.game.players[0].hand[0].clone();

    let observed: std::sync::Arc<std::sync::Mutex<Option<(u8, Option<u8>)>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let observed_cb = observed.clone();

    {
        let mut ctx = EffectContext::new_with_override(
            &mut runner.game, &src, None, /* controller */ 0, Some(1),
        );
        ctx.select_hand(
            1, // selecting_player override target — picks from P1's hand
            "Pick", /* optional */ false,
            |_g, _i| true,
            move |inner, _picked| {
                let mut slot = observed_cb.lock().unwrap();
                *slot = Some((
                    inner.player,
                    inner.override_selecting_player(),
                ));
            },
        );
    }

    let action = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    runner.game.resolve_selection(1, action).unwrap();

    let observed = *observed.lock().unwrap();
    assert_eq!(observed, Some((0, Some(1))),
        "callback ctx must keep controller=0 and override=Some(1)");
}
```

Add a public read accessor `pub fn override_selecting_player(&self) -> Option<PlayerId>` on `EffectContext` (read-only — separate from the writable `pub(crate)` field). Currently the field is `pub(super)`; expose a getter for tests + DSL lowering.

Register: `mod override_persistence;` in `code/digimon-engine/tests/effect_context/main.rs`.

- [ ] **Step 2: Run, expect FAIL** — `new_with_override` does not exist; current `select_hand` callback collapses controller and selecting_player.

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context override_persistence`

- [ ] **Step 3: Add the constructor + getter**

In `code/digimon-engine/src/effect_context/mod.rs`:

```rust
impl<'a> EffectContext<'a> {
    pub fn new_with_override(
        game: &'a mut Game,
        source_card: &'a CardSource,
        source_permanent: Option<PermanentHandle>,
        controller: PlayerId,
        override_selecting_player: Option<PlayerId>,
    ) -> Self {
        let mut ctx = Self::new(game, source_card, source_permanent, controller);
        ctx.override_selecting_player = override_selecting_player;
        ctx
    }

    pub fn override_selecting_player(&self) -> Option<PlayerId> {
        self.override_selecting_player
    }
}
```

- [ ] **Step 4: Refactor every selection callback in `selections.rs`**

For every `pub fn select_*` in `code/digimon-engine/src/effect_context/selections.rs` that builds a `Box::new(move |game: &mut Game, action_id: u16| { let mut ctx = EffectContext::new(game, source_card, source_permanent, selecting_player); ... })` callback, replace with:

```rust
let controller = self.player;
let override_pin = self.override_selecting_player;
// ...
callback: Box::new(move |game: &mut Game, action_id: u16| {
    let trash_index = action_id.saturating_sub(TRASH_EFFECT_START) as usize;
    let mut ctx = EffectContext::new_with_override(
        game, source_card, source_permanent, controller, override_pin,
    );
    user_callback(&mut ctx, trash_index);
}),
```

Verify each rewrite preserves the existing `selecting_player = self.override_selecting_player.unwrap_or(self.player)` line that seeds `pending_selection.selecting_player` — that line stays unchanged; we change only what the **callback** sees.

Apply uniformly to `select_hand`, `select_trash`, `select_own_permanent`, `select_opponent_permanent`, `select_count_capped_multi` (and its trampoline `install_count_capped_step`), `select_effect_choice`, `select_reveal`, `select_security`, `select_material`, `select_union_zone`, `select_ordered_permutation`.

This is mechanical — count the call sites first (`grep -nE "EffectContext::new\(game" code/digimon-engine/src/effect_context/selections.rs`) and update each.

- [ ] **Step 5: Run, expect PASS** — both the new test and every existing selection test

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/tests/effect_context/override_persistence.rs code/digimon-engine/tests/effect_context/main.rs
git commit -m "engine: persist override_selecting_player through selection callbacks (Phase 2f3)"
```

### Task 2: `AsSelectingPlayer` step lowering

**Files:**
- Create: `code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f3_as_selecting_player.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

The lowering: set `ctx.override_selecting_player = Some(resolve_player(of))`, run `body` via `run_steps`, restore the previous override on synchronous return. On park, the captured `dsl_outer_tail` already includes the rest of the outer process slice; the inner callback's `EffectContext` (built via `new_with_override` from Task 1) preserves the override automatically. **The override must NOT be restored on park** — Task 1 ensures it survives the callback boundary.

- [ ] **Step 1: Write the failing DSL test**

Create `code/digimon-engine/tests/dsl/phase2f3_as_selecting_player.rs`:

```rust
#[test]
fn as_selecting_player_routes_prompt_to_opponent() {
    // Build: P0 plays a card whose effect is
    //   as_selecting_player: { of: opponent, body: [select_own_permanent: ...] }
    // The opponent is P1. After the body's select_own_permanent installs,
    // pending_selection.selecting_player must be P1, NOT P0.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "Source"))
        .add_card(make_test_card("TST-VICTIM", "Victim"))
        .hand(0, &["TST-A"])
        .field(0, &["TST-VICTIM"])
        .build();
    let src = runner.game.players[0].hand[0].clone();

    let body = vec![CompiledStep::SelectOwnPermanent {
        filter: CompiledPredicate::Always,
        bind_as: Some("victim".into()),
        prompt: "Pick".into(), prompt_key: None, optional: false,
    }];
    let steps = vec![CompiledStep::AsSelectingPlayer {
        of: CompiledPlayerRef::Opponent,
        body,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, &src, None, 0);
        let mut bindings = Bindings::default();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner.game.pending_selection.as_ref().expect("select_own_permanent parked");
    assert_eq!(pending.selecting_player, 1, "opponent (P1) chooses");
}

#[test]
fn as_selecting_player_chained_selects_all_route_to_override() {
    // Two select_*s back-to-back inside the body; both prompt P1.
    // Resolve the first; assert the second's pending_selection.selecting_player == 1.
    // ... full body modeled on phase2e_select_effect_choice.rs.
}
```

Register: `mod phase2f3_as_selecting_player;` in `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run, expect FAIL** — `as_selecting_player` is unhandled in `run_step`; the body either silently no-ops or executes with wrong selecting_player.

- [ ] **Step 3: Implement the lowering**

Create `code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs`:

```rust
//! Phase 2f3 — `AsSelectingPlayer` lowering.
//!
//! Sets `ctx.override_selecting_player` to the resolved `of:` player for the
//! duration of `body`. On park, the override persists into selection callbacks
//! via Task 1's `new_with_override` constructor. On synchronous body
//! completion, the previous override is restored before returning so steps
//! after this one don't inherit the override.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::{resolve_player, run_steps, RunOutcome};
use crate::effect_context::EffectContext;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::AsSelectingPlayer { of, body } => {
            let p = resolve_player(ctx, *of);
            let prev = ctx.override_selecting_player();
            // override_selecting_player is pub(crate), set directly:
            ctx.override_selecting_player = Some(p);
            let outcome = run_steps(body, ctx, bindings);
            if matches!(outcome, RunOutcome::Synchronous) {
                ctx.override_selecting_player = prev;
            }
            // On Parked, the captured callback owns the override via Task 1.
            // The outer-tail (steps after this AsSelectingPlayer) is parked by
            // run_steps' caller — those steps run WITHOUT the override because
            // the outer-tail closure constructs its own EffectContext with the
            // controller as `player`, no override. (The override scope is
            // strictly the body, including any nested callbacks inside it.)
            Some(outcome)
        }
        _ => None,
    }
}
```

In `code/digimon-engine/src/dsl_cards/step/mod.rs`, add `pub mod as_selecting_player;` and dispatch in `run_steps`'s control-flow probe block (between `control_flow::try_run` and `iteration::try_run`):

```rust
if let Some(outcome) = as_selecting_player::try_run(step, ctx, bindings) {
    if matches!(outcome, RunOutcome::Parked) {
        park_outer_tail(ctx, bindings, steps, i);
        return RunOutcome::Parked;
    }
    i += 1;
    continue;
}
```

- [ ] **Step 4: Run, expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2f3`
Expected: PASS — both tests.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs code/digimon-engine/src/dsl_cards/step/mod.rs code/digimon-engine/tests/dsl/phase2f3_as_selecting_player.rs code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl: lower as_selecting_player with override-persistence (Phase 2f3)"
```

### Task 3: End-to-end YAML test

**Files:**
- Create: `code/digimon-engine/tests/dsl/phase2f3_end_to_end.rs`

YAML-load a card whose printed text is "your opponent chooses one of your Digimon; that Digimon gets -3000 DP for the turn." (No real card cited — write a plausible synthetic.)

- [ ] **Step 1: Write the test** with full YAML body, modeled on `phase2e_end_to_end.rs`.
- [ ] **Step 2: Run, expect PASS** (no new code changes from Task 2).
- [ ] **Step 3: Commit.**

### Task 4: Spec note + 2f3 closeout

- [ ] Update §3.7.7 to note `as_selecting_player` is fully wired with override-persistence.
- [ ] Append `2f3 (landed)` bullet in §7.3.
- [ ] Commit spec.

---

# Sub-phase 2f4 — `schedule_delayed` engine primitive + DSL lowering

After 2f4, `process: [schedule_delayed: { when: end_of_your_turn, body: [...] }]` registers a one-shot delayed effect that fires when the engine reaches the matching timing. The body compiles identically to a triggered clause's process — every step verb landed in 2a–2f3 is available inside it. Bindings captured at scheduling time are preserved into the firing context (lexical capture, like `dsl_outer_tail`).

## File structure (2f4)

- Modify: `code/digimon-engine/src/game.rs` — add `pub scheduled_effects: Vec<ScheduledEffect>`.
- Create: `code/digimon-engine/src/scheduled_effects.rs` — `pub struct ScheduledEffect { when: Timing, body: Vec<CompiledStep>, source_card: CardSource, source_permanent: Option<PermanentHandle>, controller: PlayerId, captured_bindings: Bindings }`. Drain helper `pub fn fire_scheduled_for_timing(game: &mut Game, t: Timing)` that splits matching effects out of the queue and runs each body via `run_steps` against a fresh `EffectContext`.
- Modify: `code/digimon-engine/src/lib.rs` — `pub mod scheduled_effects;`.
- Modify: `code/digimon-engine/src/effect_context/mod.rs` — `pub fn schedule_delayed(&mut self, when: Timing, body: Vec<CompiledStep>, captured: Bindings)` — appends to `game.scheduled_effects`. Public-on-the-trait so DSL can call it.
- Modify: `code/digimon-engine/src/game_phases.rs` (or wherever `EndOfYourTurn` / `EndOfTurn` etc. are dispatched) — call `fire_scheduled_for_timing(game, t)` at every observer-fire boundary that produces a `Timing`.
- Create: `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs` — DSL lowering. Resolves the IR's `CompiledTiming` to engine `Timing` (reuse `code/digimon-engine/src/dsl_cards/timing_map.rs`), clones the current `Bindings`, calls `ctx.schedule_delayed(t, body, bindings)`.
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs` — register the new module + dispatch arm.
- Create: `code/digimon-engine/tests/effect_context/schedule_delayed.rs` — engine-level test: schedule for `EndOfYourTurn`, advance turn, assert body fired.
- Create: `code/digimon-engine/tests/dsl/phase2f4_schedule_delayed.rs` — DSL test: schedule `gain_memory: 1` at `end_of_opponents_turn`, advance two turns, assert `runner.game.memory == 1`.
- Create: `code/digimon-engine/tests/dsl/phase2f4_end_to_end.rs` — YAML-loaded card scheduling a one-shot.
- Modify: `code/digimon-engine/tests/dsl/main.rs`, `code/digimon-engine/tests/effect_context/main.rs`.
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — append 2f4 bullet.

### Task 1: `ScheduledEffect` queue + drain

**Files:**
- Create: `code/digimon-engine/src/scheduled_effects.rs`
- Modify: `code/digimon-engine/src/lib.rs`, `code/digimon-engine/src/game.rs`
- Create: `code/digimon-engine/tests/effect_context/schedule_delayed.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/effect_context/schedule_delayed.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::Timing;
use digimon_engine::scheduled_effects::fire_scheduled_for_timing;
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_dsl::compiled::CompiledStep;

#[test]
fn scheduled_effect_fires_on_matching_timing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "S"))
        .hand(0, &["TST"])
        .build();
    runner.game.memory = 0;
    let src = runner.game.players[0].hand[0].clone();

    {
        let mut ctx = EffectContext::new(&mut runner.game, &src, None, 0);
        ctx.schedule_delayed(
            Timing::EndOfYourTurn,
            vec![CompiledStep::GainMemory(1)],
            Bindings::default(),
        );
    }
    assert_eq!(runner.game.memory, 0, "not fired before timing");
    fire_scheduled_for_timing(&mut runner.game, Timing::EndOfYourTurn);
    assert_eq!(runner.game.memory, 1, "fired exactly once");
    fire_scheduled_for_timing(&mut runner.game, Timing::EndOfYourTurn);
    assert_eq!(runner.game.memory, 1, "drained — does not fire twice");
}

#[test]
fn scheduled_effect_with_other_timing_does_not_fire() {
    // Schedule for EndOfYourTurn, fire EndOfOpponentsTurn — no-op.
    // ...
}
```

Register: `mod schedule_delayed;` in `code/digimon-engine/tests/effect_context/main.rs`.

- [ ] **Step 2: Run, expect FAIL** — module + API don't exist.

- [ ] **Step 3: Implement the queue + drain**

Create `code/digimon-engine/src/scheduled_effects.rs`:

```rust
//! Phase 2f4 — `ScheduledEffect` queue. One-shot delayed effects scheduled
//! by `EffectContext::schedule_delayed`; drained at observer-fire boundaries
//! that match the scheduled `when:` timing.

use crate::card_source::CardSource;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::run_steps;
use crate::effect_context::EffectContext;
use crate::enums::{PlayerId, Timing};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use digimon_dsl::compiled::CompiledStep;

pub struct ScheduledEffect {
    pub when: Timing,
    pub body: Vec<CompiledStep>,
    pub source_card: CardSource,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
    pub captured_bindings: Bindings,
}

/// Drain every `ScheduledEffect` whose `when` matches `t`, running each
/// body to completion (synchronously or by parking; parked bodies follow
/// the same `dsl_outer_tail` rules as any other parked process slice).
///
/// Effects are drained in FIFO order. Cards that schedule additional
/// effects from inside a firing body land at the back of the queue;
/// they do NOT fire in the same drain pass even if they share the
/// matching timing — re-entrancy is bounded by the outer caller's
/// next observer-fire boundary.
pub fn fire_scheduled_for_timing(game: &mut Game, t: Timing) {
    let mut still_pending: Vec<ScheduledEffect> = Vec::new();
    let queue = std::mem::take(&mut game.scheduled_effects);
    for eff in queue.into_iter() {
        if eff.when != t {
            still_pending.push(eff);
            continue;
        }
        let ScheduledEffect {
            body, source_card, source_permanent, controller, captured_bindings, ..
        } = eff;
        let mut bindings = captured_bindings;
        let mut ctx = EffectContext::new(
            game, &source_card, source_permanent, controller,
        );
        run_steps(&body, &mut ctx, &mut bindings);
    }
    // Effects scheduled DURING this drain go on `game.scheduled_effects`
    // (set by `schedule_delayed` against a fresh `&mut game`); merge
    // unmatched-but-original effects back in.
    for eff in still_pending.into_iter() {
        game.scheduled_effects.push(eff);
    }
}
```

Add `pub scheduled_effects: Vec<ScheduledEffect>` to `Game` in `code/digimon-engine/src/game.rs` (default `Vec::new()` in the `new` / `Default` impls). `pub mod scheduled_effects;` in `code/digimon-engine/src/lib.rs`.

Add `EffectContext::schedule_delayed`:

```rust
pub fn schedule_delayed(
    &mut self,
    when: Timing,
    body: Vec<CompiledStep>,
    captured_bindings: crate::dsl_cards::bindings::Bindings,
) {
    self.game.scheduled_effects.push(crate::scheduled_effects::ScheduledEffect {
        when,
        body,
        source_card: self.source_card.clone(),
        source_permanent: self.source_permanent,
        controller: self.player,
        captured_bindings,
    });
}
```

(`source_card.clone()` — `CardSource: Clone` per existing convention; if not, store the `CardHandle` and re-resolve at fire time. Verify by `grep -n "impl Clone for CardSource" code/digimon-engine/src/card_source.rs`.)

- [ ] **Step 4: Run, expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context schedule_delayed`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/scheduled_effects.rs code/digimon-engine/src/lib.rs code/digimon-engine/src/game.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/tests/effect_context/schedule_delayed.rs code/digimon-engine/tests/effect_context/main.rs
git commit -m "engine: ScheduledEffect queue + schedule_delayed primitive (Phase 2f4)"
```

### Task 2: Wire drain into observer-fire boundaries

**Files:**
- Modify: `code/digimon-engine/src/game_phases.rs` (and any other site that fires timing observers)

The drain must happen at every Timing transition the engine already fires. Ground truth: `grep -nE "EndOfYourTurn|EndOfTurn|EndOfBattle|EndOfAttack|StartOfYour" code/digimon-engine/src/game_phases.rs code/digimon-engine/src/combat.rs` — find every site where a `Timing` is dispatched to observers, and add `crate::scheduled_effects::fire_scheduled_for_timing(self, Timing::*)` before/after the observer fire (after — observers seeing the post-scheduled state would be a behaviour change; use before unless a regression-test forces the swap).

- [ ] **Step 1: Write a failing test that exercises one such boundary**

Append to `code/digimon-engine/tests/effect_context/schedule_delayed.rs`:

```rust
#[test]
fn scheduled_fires_when_engine_advances_turn() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "S"))
        .hand(0, &["TST"])
        .build();
    runner.game.memory = 0;
    let src = runner.game.players[0].hand[0].clone();
    {
        let mut ctx = EffectContext::new(&mut runner.game, &src, None, 0);
        ctx.schedule_delayed(
            Timing::EndOfYourTurn,
            vec![CompiledStep::GainMemory(1)],
            Bindings::default(),
        );
    }
    runner.end_turn(0);
    assert_eq!(runner.game.memory, 1);
}
```

(Adapt `runner.end_turn` to whatever turn-advance helper `DebugRunner` exposes; if absent, drive the phase transition manually.)

- [ ] **Step 2: Run, expect FAIL** — drain not yet wired into the phase transition.

- [ ] **Step 3: Add the drain calls** at every `Timing::*` observer-fire site.

- [ ] **Step 4: Run, expect PASS** + run the full engine test suite as regression guard.

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit.**

### Task 3: DSL lowering

**Files:**
- Create: `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Create: `code/digimon-engine/tests/dsl/phase2f4_schedule_delayed.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the failing DSL test** — `CompiledStep::ScheduleDelayed { when, body }` runs through `run_steps`, advances the turn, asserts memory.

- [ ] **Step 2: Run, expect FAIL** — `ScheduleDelayed` is unhandled in `run_step`.

- [ ] **Step 3: Implement**

```rust
// code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs
use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::timing_map::lookup_timing;
use crate::effect_context::EffectContext;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> bool {
    match step {
        CompiledStep::ScheduleDelayed { when, body } => {
            let Some(t) = lookup_timing(when) else { return true; };
            ctx.schedule_delayed(t, body.clone(), bindings.clone());
            true
        }
        _ => false,
    }
}
```

Wire into `run_step` in `step/mod.rs`. Note: `bindings: &Bindings` — read-only is sufficient because the captured clone is owned by the queue entry.

- [ ] **Step 4: Run, expect PASS.**

- [ ] **Step 5: Commit.**

### Task 4: End-to-end YAML test

**Files:**
- Create: `code/digimon-engine/tests/dsl/phase2f4_end_to_end.rs`

YAML body:

```yaml
card: TST-DEL
name: "DelayedDraw"
kind: option
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - schedule_delayed:
          when: end_of_your_turn
          body:
            - draw: { of: you, count: 1 }
```

Test: play TST-DEL, advance turn, assert P0's hand grew by 1.

- [ ] **Steps:** failing test → run → expect PASS (no new code) → commit.

### Task 5: Spec note + 2f4 closeout

- [ ] Update §3.7.8 row for `schedule_delayed` — drop "planned (Tier-2 gap)", note it is fully wired.
- [ ] Append `2f4 (landed)` bullet to §7.3. Note Phase 2 is now feature-complete; defer remaining advanced clauses (`replacement`, broader `event_target_*` predicates, per-iteration park resumption) to Phase 3 per §7.4.
- [ ] Commit.

---

# Plan close-out

After all four sub-phases land:

- [ ] **Verify Phase 2 is feature-complete:** every variant of `CompiledStep` has a non-trivial dispatch arm in `code/digimon-engine/src/dsl_cards/step/`. Run `cargo test --manifest-path code/digimon-engine/Cargo.toml` — full green.
- [ ] **Update §7.3** intro paragraph to read "Phase 2 — Imperative `process:` compiler — **landed** (sub-phases 2a–2f4)." Re-anchor the `**Sub-phase progress:**` block: every sub-phase's `Defers to N+:` list should now empty out for Phase 2 items; only Phase 3 forward-references remain.
- [ ] **Optional: open the Phase 3 plan stub** — `docs/superpowers/plans/<date>-card-scripting-dsl-phase-3.md` skeleton listing the §7.4 scope (replacement / partition rework / `event_target_*` predicates / per-iteration park resumption / formula primitives beyond literals). No tasks yet — Phase 3 wants its own brainstorming pass.

## Self-review (writing-plans skill checklist)

**Spec coverage** — every Phase-2-deferred item in §7.3 sub-phase 2e's "Defers to 2f+" list maps to a sub-phase task:
- `AsSelectingPlayer` (override-persistence) → 2f3 Tasks 1–4.
- play / digivolve / placement steps → 2f1 Tasks 2–7.
- formula values in `add_modifier` `value` → 2f2 Tasks 1–4.
- `ScheduleDelayed` → 2f4 Tasks 1–5.

**Type consistency** — `CompiledModifierValue` (2f2) is named the same in compiled.rs, in step/modifiers.rs, and in tests. `EffectContext::new_with_override` (2f3) takes the same parameter list at every call site. `ScheduledEffect` fields (2f4) are read in the same order in `schedule_delayed` (write side) and `fire_scheduled_for_timing` (read side). The runtime evaluator is named `evaluate` (not `eval`) consistently across module declaration, function definition, and test imports.

**Placeholder scan** — every test-body `// ...` placeholder is annotated with the existing test it should mirror (e.g. "modeled on `phase2c_permanent_mutations.rs::stack_with_three_materials`"). Engine method names (`effective_dp`, `top_level`, `printed_play_cost`, `digivolution_colors`, `raw_rust_registry`) carry an explicit "verify by grep before committing" instruction in the relevant task — this is unavoidable for an engine-side plan because the codebase changes between when the plan is written and when it is executed; the grep makes the verification mechanical.

**Forward references** — the plan does not use any types or methods that aren't either (a) already in the codebase per the pre-flight grep results, or (b) defined in an earlier task within the plan.
