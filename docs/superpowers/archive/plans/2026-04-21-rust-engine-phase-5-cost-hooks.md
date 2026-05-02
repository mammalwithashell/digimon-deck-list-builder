# Rust Engine Phase 5 — Cost-Reduction Builder Hooks

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `EffectBuilder` with closure-valued cost hooks so card effects can faithfully model dynamic cost reduction (reads live game state) and triggered-effect pay-costs (suspend-Tamer, trash-N-cards, etc.) — all without introducing Python's `_temp_play_cost_reduction` anti-pattern.

**Architecture:** Two new builder hooks — `.cost_reduction_fn(|ctx| i32)` for dynamic reductions and `.pay_cost_fn(|ctx| bool)` for triggered-effect pay-costs. The reduction hook fires during `play_from_hand_with_cost` / `play_from_trash_with_cost` / digivolve-cost paths by scanning battle-area effects with `EffectTiming::BeforePayCost`. The pay-cost hook fires in `run_queued_effect` between condition and process for triggered timings, and during play/digivolve for BeforePayCost. No new `CostDelta` variants; existing pipeline is preserved.

**Tech Stack:** Rust 2021 (`digimon-engine/`), DebugRunner test harness, existing `Box<dyn Fn + Send + Sync + 'static>` closure pattern from `EffectBuilder`.

---

## Background

Phase 5 closes Cluster F from [.claude/plans/recursive-coalescing-candle.md](.claude/plans/recursive-coalescing-candle.md):

- **Dynamic cost reduction** (~30 cards): "*Reduce cost by 1 for each [Machine] in your trash*", "*Reduce digivolve cost by 2 if target has [Royal Knight] trait*". Can't be static because reduction reads live state.
- **`.pay_cost_fn` on triggered effects** (~15 cards): "*Suspend 1 of your Tamers to activate this effect*", "*Trash 3 cards from your hand as cost*". Cost is paid after condition, before process.
- **`.activation_cost` on triggered abilities** — folded into `.pay_cost_fn`; single hook, dispatched at two sites (triggered-effect queue vs. play/digivolve path) based on timing.

**What exists today** (from 2026-04-21 survey):
- `Effect.cost_reduction: i32` — static int, set via `EffectBuilder::cost_reduction(n)` ([effect.rs:311-314](digimon-engine/src/effect.rs:311)). Never scanned by the play pipeline.
- `CostDelta` enum with `Free / Reduce(i16) / Fixed(i16)` variants ([enums.rs:381-399](digimon-engine/src/enums.rs:381)) — used by `play_from_hand_with_cost` for the *caller-supplied* delta (e.g. "play free from hand" effects). Separate from the BeforePayCost scan.
- `play_from_hand_with_cost` / `play_from_trash_with_cost` ([game_actions.rs:63-200](digimon-engine/src/game_actions.rs:63)) — read `printed_cost`, apply `CostDelta`, call `pay_memory`. **No BeforePayCost scan.**
- `run_queued_effect` ([effect_queue.rs:262-318](digimon-engine/src/effect_queue.rs:262)) — validates source, checks condition, runs process. **No pay-cost hook.**
- `EffectTiming::BeforePayCost` variant may or may not exist — confirm at Task 1.

**Python cross-reference:**
- `_temp_play_cost_reduction` instance variable (`effects.py:379-437`) — workaround that leaked across effects. Python Issue 24 (memory snippet). **Rust must not replicate this pattern.**
- Python has no `.pay_cost()` hook. Scripts that need pay-costs hand-roll the selection + payment logic inside a `process` closure and hope the condition caught the affordability case. Rust's `.pay_cost_fn` is net-new.
- Static `cost_reduction: int = 0` parameter on helpers (`effects.py:463`). Same limitation as Rust today.

**Design principles (carry-forward):**
1. No auto-selection — if `.pay_cost_fn` needs the player to pick which Tamer to suspend, the closure must `install_selection` and surface the choice.
2. Closures over flags for runtime predicates.
3. One hook, multiple dispatch points (keep the API surface small).
4. `.pay_cost_fn` returning `false` aborts the effect silently — same contract as a failing condition.
5. Dynamic cost reduction at BeforePayCost must scan **only** `battle_area` effects whose source permanent is still on the field and whose `condition` passes — avoids Python Issue 24's effect-leak.
6. Closure-valued hooks store `Option<Box<dyn Fn(...) + Send + Sync + 'static>>` matching the existing `condition`/`process` pattern.
7. TDD per working rule 18 — failing test first.

**Cards motivating Phase 5** (from `.claude/plans/rust-engine-gaps-rocks.md`):
- BT21-055 Sunarizamon — digivolve-cost reduction gated by target's traits
- EX10-033 Pyramidimon — cost reduction contingent on selecting N sources from trash
- EX8-067 Close — "suspend a Tamer to activate" (`.pay_cost_fn` on `OnDigivolve`)
- P-186 Gallantmon — closure reads both players' trashes
- EX11-044 Pyramidimon — `<Digi-Burst N>` keyword, coordinated with `.pay_cost_fn`

---

## File Structure

**Modified:**
- `digimon-engine/src/effect.rs` — add `cost_reduction_fn` field + `pay_cost_fn` field to `Effect` struct; two new builder methods; update `Effect::new`
- `digimon-engine/src/enums.rs` — confirm/add `EffectTiming::BeforePayCost` variant
- `digimon-engine/src/game_actions.rs` — insert BeforePayCost scan into `play_from_hand_with_cost` + `play_from_trash_with_cost` + digivolve cost path
- `digimon-engine/src/effect_queue.rs` — insert `.pay_cost_fn` hook into `run_queued_effect` between condition and process
- `digimon-engine/src/effect_context/mod.rs` — add `CostReductionContext` (or extend `EffectReadContext`) if closures need cost-specific read surface
- `docs/RUST_ENGINE_API.md` — new §Phase 5 Cost Hooks section
- `docs/RUST_PYTHON_PARITY.md` — annotate cost-related divergences; Rust's closure approach is strictly more faithful than Python's `_temp_` workaround
- `.claude/plans/recursive-coalescing-candle.md` — Phase 5 row in cumulative table

**New tests:**
- `digimon-engine/tests/cost_hooks/cost_reduction_static.rs`
- `digimon-engine/tests/cost_hooks/cost_reduction_fn.rs`
- `digimon-engine/tests/cost_hooks/pay_cost_triggered.rs`
- `digimon-engine/tests/cost_hooks/pay_cost_before_pay.rs`
- `digimon-engine/tests/cost_hooks/before_pay_cost_scan_hygiene.rs` (Python Issue 24 regression)
- `digimon-engine/tests/cost_hooks/main.rs` — test harness module

---

## Tasks

### Task 1: Add `cost_reduction_fn` and `pay_cost_fn` fields + builder methods

**Files:**
- Modify: `digimon-engine/src/effect.rs` — add two new fields to `Effect`, two builder methods to `EffectBuilder`
- Modify: `digimon-engine/src/enums.rs` — confirm `EffectTiming::BeforePayCost` exists; add if missing

No dispatch yet — this task adds the surface area only. Dispatch wires up in Tasks 2-4.

- [ ] **Step 1: Confirm/add `EffectTiming::BeforePayCost`**

Read `digimon-engine/src/enums.rs` and grep for `BeforePayCost`. If present, note the discriminant. If absent, add it as a new variant and update any exhaustive match arms to include a placeholder arm (with comment: "Phase 5 — dispatch wires up in Task 2").

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/cost_hooks/main.rs` and `digimon-engine/tests/cost_hooks/static_builder_surface.rs`:

```rust
// static_builder_surface.rs
use digimon_engine::effect::Effect;
use digimon_engine::card_source::CardHandle;
use digimon_engine::enums::EffectTiming;

#[test]
fn effect_builder_exposes_cost_reduction_fn() {
    let effect = Effect::new(EffectTiming::BeforePayCost, CardHandle(0))
        .name("test reduction")
        .condition(|_ctx| true)
        .cost_reduction_fn(|_ctx| 3)  // closure returns i32
        .build();
    assert!(effect.cost_reduction_fn.is_some());
}

#[test]
fn effect_builder_exposes_pay_cost_fn() {
    let effect = Effect::new(EffectTiming::OnPlay, CardHandle(0))
        .name("test pay cost")
        .process(|_ctx| {})
        .pay_cost_fn(|_ctx| true)  // closure returns bool
        .build();
    assert!(effect.pay_cost_fn.is_some());
}
```

Add `mod static_builder_surface;` to `digimon-engine/tests/cost_hooks/main.rs`.

- [ ] **Step 3: Run — expect compile error on missing fields + builder methods**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cost_hooks static_builder_surface`
Expected: compile failure.

- [ ] **Step 4: Implement**

In `effect.rs`, add to the `Effect` struct (alongside `condition` and `process`):
```rust
pub cost_reduction_fn: Option<Box<dyn Fn(&EffectReadContext) -> i32 + Send + Sync + 'static>>,
pub pay_cost_fn: Option<Box<dyn Fn(&mut EffectContext) -> bool + Send + Sync + 'static>>,
```

Update `Effect::new` to initialize both to `None`.

Add two builder methods:
```rust
impl EffectBuilder {
    pub fn cost_reduction_fn<F>(mut self, f: F) -> Self
    where F: Fn(&EffectReadContext) -> i32 + Send + Sync + 'static {
        self.effect.cost_reduction_fn = Some(Box::new(f));
        self
    }

    pub fn pay_cost_fn<F>(mut self, f: F) -> Self
    where F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static {
        self.effect.pay_cost_fn = Some(Box::new(f));
        self
    }
}
```

Note: `cost_reduction_fn` takes `&EffectReadContext` (read-only, because the reduction calc should be pure). `pay_cost_fn` takes `&mut EffectContext` (mutable, because paying the cost may trash cards, suspend permanents, install selections, etc.).

- [ ] **Step 5: Run — expect PASS**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cost_hooks`
Expected: 2 tests pass.

- [ ] **Step 6: Full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 499 (current baseline after place_remainder fixes) + 2 = 501 passing.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/effect.rs digimon-engine/src/enums.rs digimon-engine/tests/cost_hooks/main.rs digimon-engine/tests/cost_hooks/static_builder_surface.rs
git commit -m "rust-engine(phase-5): add cost_reduction_fn + pay_cost_fn fields + builder methods"
```

---

### Task 2: Wire BeforePayCost scan into play pipeline

**Files:**
- Modify: `digimon-engine/src/game_actions.rs` — insert BeforePayCost scan into `play_from_hand_with_cost` + `play_from_trash_with_cost`
- Create: `digimon-engine/tests/cost_hooks/cost_reduction_fn.rs`
- Create: `digimon-engine/tests/cost_hooks/before_pay_cost_scan_hygiene.rs`

**Semantics:** Before the `pay_memory` call, scan all battle-area permanents of both players for effects where:
- `effect.timing == EffectTiming::BeforePayCost`
- `effect.condition` evaluates to `true` (or condition is None)
- The scan is scoped to the CURRENT play action's context (source_card = the card being played, source_permanent = None for play-from-hand)

For each matching effect:
- If `effect.cost_reduction_fn.is_some()`: invoke closure → accumulate returned i32 into `total_reduction`
- Else if `effect.cost_reduction != 0` (static): add to `total_reduction`

Compute `effective_cost = max(0, printed_cost - caller_delta - total_reduction)` (existing `CostDelta` still applied first). Call `pay_memory(effective_cost)`.

**Critical invariant (Python Issue 24 avoidance):** The scan must only include effects whose source permanent is currently in battle_area AND whose condition passes. An effect-leak from trash / hand / field-gone would be a regression.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/cost_hooks/cost_reduction_fn.rs`:

```rust
#[test]
fn closure_valued_cost_reduction_reads_live_state() {
    // Set up: player 0 has 2 permanents in trash; installs a permanent with
    // BeforePayCost effect whose closure returns |ctx| ctx.player(0).trash.len() as i32.
    // Play a Lv.5 Digimon from hand with printed_cost = 7.
    // Expected: effective cost = 7 - 2 = 5. Memory goes from 0 to -5.
}

#[test]
fn static_cost_reduction_stacks_with_closure_reduction() {
    // Two different permanents: one with static .cost_reduction(1), one with
    // .cost_reduction_fn(|_| 2). Total reduction = 3.
}

#[test]
fn cost_reduction_fn_returning_negative_does_not_increase_cost() {
    // Closure returns -5 (defensive). Effective cost clamps at 0 minimum.
    // Verify effective_cost = max(0, printed_cost - (-5)) ... actually this
    // should CLAMP the reduction to >=0, not subtract a negative (which would
    // INCREASE cost). Behavior: treat returned i32 as "reduction amount"; if
    // closure returns -5, treat as 0 reduction.
}
```

Create `digimon-engine/tests/cost_hooks/before_pay_cost_scan_hygiene.rs`:

```rust
#[test]
fn scan_excludes_trashed_sources() {
    // Permanent A has BeforePayCost effect reducing cost by 3.
    // Trash A. Play another Digimon. Expected: no reduction (A is not in battle_area).
}

#[test]
fn scan_excludes_failed_condition() {
    // Permanent A has BeforePayCost effect with condition |ctx| false.
    // Play a Digimon. Expected: no reduction.
}

#[test]
fn scan_applies_to_opponent_effects_too() {
    // Opponent has a card with BeforePayCost that reduces YOUR cost by 1
    // (rare but possible — e.g., some tech cards). Verify both players'
    // battle areas are scanned.
    // (If no real card does this, alternative: verify scan is scoped correctly
    // — e.g., "cards you play" vs. "cards opponent plays".)
}

#[test]
fn scan_excludes_out_of_scope_timing() {
    // Permanent with an OnPlay effect that has cost_reduction = 99 (static).
    // Play a Digimon. Expected: cost_reduction NOT applied (only BeforePayCost
    // timing effects are scanned).
    // This is the Python Issue 24 regression test.
}
```

- [ ] **Step 2: Run — expect compile/assertion failures**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cost_hooks`

- [ ] **Step 3: Implement scan**

In `game_actions.rs`, before the `pay_memory` call in `play_from_hand_with_cost`:

```rust
// Phase 5: scan BeforePayCost effects in battle area of both players
let total_reduction = {
    let mut sum: i32 = 0;
    for player_idx in 0..2 {
        let player_id = PlayerId(player_idx as u8);
        for perm in self.player(player_id).battle_area.clone() {
            let card_id = /* get card_id from perm */;
            if let Some(effects) = self.effects_for_card(&card_id, perm.top_card_handle) {
                for effect in effects.iter() {
                    if effect.timing != EffectTiming::BeforePayCost { continue; }
                    let read_ctx = EffectReadContext::new(self, /* ... */);
                    if let Some(cond) = &effect.condition {
                        if !cond(&read_ctx) { continue; }
                    }
                    if let Some(reduction_fn) = &effect.cost_reduction_fn {
                        sum += reduction_fn(&read_ctx).max(0);
                    } else if effect.cost_reduction != 0 {
                        sum += effect.cost_reduction.max(0);
                    }
                }
            }
        }
    }
    sum
};

let effective_cost = std::cmp::max(0, cost_delta.resolve(printed_cost) - total_reduction as i16);
if !self.pay_memory(effective_cost) { return None; }
```

(Exact implementation depends on the existing `effects_for_card` API and `EffectReadContext::new` signature — adjust to the real API.)

Do the same in `play_from_trash_with_cost`. If digivolve has a separate cost-calc path, apply there too (check `game_actions.rs` for `digivolve_from_hand` or similar).

- [ ] **Step 4: Run — expect PASS on all 7 new tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cost_hooks`

- [ ] **Step 5: Full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 501 + 7 = 508 passing.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/tests/cost_hooks/cost_reduction_fn.rs digimon-engine/tests/cost_hooks/before_pay_cost_scan_hygiene.rs digimon-engine/tests/cost_hooks/main.rs
git commit -m "rust-engine(phase-5): wire BeforePayCost scan into play pipeline with closure reduction support"
```

---

### Task 3: Wire `.pay_cost_fn` into `run_queued_effect`

**Files:**
- Modify: `digimon-engine/src/effect_queue.rs` — insert pay-cost hook between condition check and process execution
- Create: `digimon-engine/tests/cost_hooks/pay_cost_triggered.rs`

**Semantics:** In `run_queued_effect`, after condition passes, BEFORE process:
1. If `effect.pay_cost_fn.is_some()`: invoke closure. If returns `false`, abort (skip process, do not surface any selection the closure didn't install).
2. If returns `true`, continue to process.

The closure receives `&mut EffectContext` so it can pay the cost (trash cards, suspend permanents, install selections that the player must resolve before process fires). If the closure installs a `PendingSelection`, the effect-queue state machine should handle that correctly — the process fires after the selection resolves.

**Design question (answer in implementation):** if `pay_cost_fn` installs a selection AND returns `true`, does `process` fire immediately or after the selection resolves? The safe answer: the closure must either (a) synchronously pay + return true, OR (b) install a selection that on-resolution continues to process via an explicit `ctx.queue_effect_resumption(...)` call. The simpler v1: `pay_cost_fn` is synchronous-only; if you need a selection, install it in `condition` (as a side effect? no — conditions are read-only) or restructure as two chained effects. Document the v1 constraint.

Actually the cleanest v1 design: `pay_cost_fn` may install a selection; if it does, return `false` (defer); the process should be wrapped in a continuation installed by the selection's callback. For v1 ship with: `pay_cost_fn` is synchronous — no selection installations. If a card needs a selection-gated pay-cost, fold it into the `process` closure (matching today's escape hatch).

Document the v1 constraint in a doc comment + add a follow-up item.

- [ ] **Step 1: Failing test**

Create `digimon-engine/tests/cost_hooks/pay_cost_triggered.rs`:

```rust
#[test]
fn pay_cost_returning_true_runs_process() {
    // Effect with .pay_cost_fn(|_| true).process(|ctx| ctx.gain_memory(1))
    // Trigger the effect. Assert memory gained 1.
}

#[test]
fn pay_cost_returning_false_skips_process() {
    // Effect with .pay_cost_fn(|_| false).process(|ctx| ctx.gain_memory(1))
    // Trigger the effect. Assert memory unchanged (process did NOT run).
}

#[test]
fn pay_cost_can_mutate_game_state() {
    // Effect with .pay_cost_fn(|ctx| { ctx.gain_memory(-2); true })
    //             .process(|ctx| ctx.gain_memory(1))
    // Trigger. Assert memory net = 0 + (-2) + 1 = -1.
}

#[test]
fn condition_gates_pay_cost() {
    // Effect with .condition(|_| false).pay_cost_fn(|_| { panic!("should not call") }).process(|_| {})
    // Trigger. Assert pay_cost_fn was NOT called because condition gated it.
}
```

- [ ] **Step 2: Run — expect failures**

- [ ] **Step 3: Implement**

In `effect_queue.rs::run_queued_effect`, after the condition check block:

```rust
// Phase 5: pay-cost hook (synchronous v1 — no selection installation)
if let Some(pay_cost) = &effect.pay_cost_fn {
    let mut ctx = EffectContext::new(self, qe.source_card, qe.source_permanent, qe.controller);
    if !pay_cost(&mut ctx) {
        return;  // cost not paid; skip process
    }
}

// Existing process block
if let Some(process) = &effect.process { ... }
```

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Full suite + commit**

Expected: 508 + 4 = 512 passing.

```bash
git add digimon-engine/src/effect_queue.rs digimon-engine/tests/cost_hooks/pay_cost_triggered.rs
git commit -m "rust-engine(phase-5): wire pay_cost_fn hook between condition and process in run_queued_effect"
```

---

### Task 4: Wire `.pay_cost_fn` into play/digivolve for BeforePayCost timing

**Files:**
- Modify: `digimon-engine/src/game_actions.rs` — if an effect with `timing == BeforePayCost` has `pay_cost_fn`, fire it during the BeforePayCost scan (Task 2) after cost reduction but before `pay_memory`
- Create: `digimon-engine/tests/cost_hooks/pay_cost_before_pay.rs`

**Semantics:** Unifies the two dispatch sites. When scanning BeforePayCost effects in the play/digivolve path, if an effect has `pay_cost_fn`, fire it after cost reduction. If it returns `false`, the play/digivolve action fails entirely (same as not having enough memory).

Typical card: *"When you play a [Machine] Digimon, you may trash 2 cards from your deck to reduce its cost by 2"* — the `pay_cost_fn` is the "trash 2 cards" part; the cost reduction is the benefit. If the deck has fewer than 2 cards, the closure returns `false`; the cost reduction doesn't apply (but the play can still proceed at full cost, or fail if memory is insufficient).

**Design refinement:** For the play-cost path, `pay_cost_fn` returning `false` means "the discount is not applied", NOT "the play fails". The play proceeds at full cost. This matches Digimon TCG rules: cost reductions are always optional from a rules perspective even when they look mandatory in card text.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn pay_cost_fn_gates_reduction_in_play_path() {
    // Effect in battle area: BeforePayCost timing with
    //   .pay_cost_fn(|ctx| { trash 2 from deck; return true })
    //   .cost_reduction_fn(|_| 2)
    // Play a Digimon with printed_cost = 6.
    // Expected: 2 cards trashed from deck; effective cost = 4.
}

#[test]
fn pay_cost_fn_returning_false_skips_reduction() {
    // Same setup but with only 1 card in deck. Closure returns false.
    // Expected: no cards trashed; effective cost = 6 (full).
    // The play still succeeds (pay_cost_fn returning false does NOT abort play).
}
```

- [ ] **Step 2: Run — failures**

- [ ] **Step 3: Implement**

Extend the scan loop from Task 2:

```rust
for effect in effects.iter() {
    if effect.timing != EffectTiming::BeforePayCost { continue; }
    // Condition check
    if let Some(cond) = &effect.condition {
        if !cond(&read_ctx) { continue; }
    }
    // Compute reduction amount first (read-only)
    let reduction = /* from cost_reduction_fn or cost_reduction */;
    // Then fire pay_cost_fn if present
    if let Some(pay) = &effect.pay_cost_fn {
        let mut ctx = EffectContext::new(self, /* ... */);
        if !pay(&mut ctx) {
            continue;  // cost couldn't be paid; skip reduction
        }
    }
    sum += reduction.max(0);
}
```

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Full suite + commit**

Expected: 512 + 2 = 514 passing.

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/tests/cost_hooks/pay_cost_before_pay.rs
git commit -m "rust-engine(phase-5): unify pay_cost_fn dispatch at BeforePayCost during play/digivolve"
```

---

### Task 5: End-to-end behavioral test

**Files:**
- Create: `digimon-engine/tests/cost_hooks/behavioral_end_to_end.rs`

**Scenario (synthesized from Rocks archetype text):**
> "Your [Machine] Digimon cost 1 less to play for each card named [Destroy Bomber] in your trash. When you play a [Machine] Digimon with ≥5 memory remaining, you may trash 2 cards from the top of your deck to reduce its cost by 2 more."

This exercises:
- Static + closure cost reduction stacking
- Condition-gated BeforePayCost effect (the "≥5 memory remaining" check)
- `pay_cost_fn` side-effect (trashing cards)
- Player-choice aspect (the "may" makes the pay_cost_fn optional — handled by wiring up via the `optional` flag; see note below)

**Note on "may":** v1 `pay_cost_fn` is synchronous, no selections. For "may trash 2 cards" semantics, the closure has to be all-or-nothing — either it always trashes (not "may"), or we add a v2 "optional pay cost" mechanism. For v1 behavioral test, make the closure always fire (non-optional) and add a follow-up item to add player-choice optionality.

The test should:
1. Set up game with the described effects in battle area
2. Set memory = 5 (triggers condition) and deck top with known cards
3. Play a printed-cost-6 Machine Digimon
4. Assert:
   - 2 cards trashed from deck (pay_cost_fn fired)
   - effective cost = 6 - (static reduction 1) - (pay_cost reduction 2) = 3
   - memory = 5 - 3 = 2
5. Repeat with memory = 4 (condition fails)
6. Assert:
   - 0 cards trashed (pay_cost_fn didn't fire because condition gated the whole effect)
   - effective cost = 6 - 1 = 5 (only static reduction applied from a different effect)
   - memory = 4 - 5 = -1

Register in `digimon-engine/tests/cost_hooks/main.rs`.

- [ ] **Step 1: Write test**
- [ ] **Step 2: Run — PASS (no implementation needed; Tasks 1-4 should cover it)**

If it fails, investigate — something's integration-wrong.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/cost_hooks/behavioral_end_to_end.rs digimon-engine/tests/cost_hooks/main.rs
git commit -m "rust-engine(phase-5): end-to-end behavioral test — static + closure reduction + pay_cost_fn"
```

---

### Task 6: Docs + roadmap update

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — add §Phase 5 Cost Hooks section with 3 subsections (`.cost_reduction_fn`, `.pay_cost_fn`, `EffectTiming::BeforePayCost` dispatch semantics) + worked examples
- Modify: `docs/RUST_PYTHON_PARITY.md` — annotate: closure-valued cost reduction + `.pay_cost_fn` hook are net-new in Rust; Python's `_temp_play_cost_reduction` anti-pattern is deliberately not replicated
- Modify: `.claude/plans/recursive-coalescing-candle.md` — flip Phase 5 table row to ✅ LANDED with commit range; add Phase 5 entry to Immediate Next Steps

- [ ] **Step 1: RUST_ENGINE_API.md**

Add three subsections under a new `## Phase 5 — Cost Hooks` heading. For each:
- Signature
- Semantics (2-4 sentences)
- v1 constraints (no selection installation inside `pay_cost_fn`)
- Worked example using a Rocks-style card text

- [ ] **Step 2: RUST_PYTHON_PARITY.md**

Add an entry under cost-related divergences:
```
§5.1 Cost-reduction closures + pay_cost_fn hook — Rust-only (Phase 5).
Status (2026-04-21): Rust implements closure-valued cost reduction at
BeforePayCost and a synchronous pay_cost_fn hook on triggered effects.
Python uses a _temp_play_cost_reduction instance variable that leaks across
effects (Issue 24). Rust intentionally does not replicate this pattern;
scripts requiring dynamic reduction must use .cost_reduction_fn.
```

- [ ] **Step 3: Roadmap**

Update Phase 5 row in the cumulative readiness table to `✅ Landed 2026-04-21 (re-audit pending)`.

Add entry 6 to Immediate Next Steps:
```
6. **Phase 5 — cost-reduction builder hooks** → ✅ LANDED (2026-04-21). Plan: `docs/superpowers/plans/2026-04-21-rust-engine-phase-5-cost-hooks.md`. Two new `EffectBuilder` methods (`.cost_reduction_fn(|ctx| i32)` + `.pay_cost_fn(|ctx| bool)`), BeforePayCost scan wired into play/digivolve paths, pay_cost_fn hook wired into run_queued_effect between condition and process. Avoids Python Issue 24's _temp_play_cost_reduction leak by design. ~50 cards unblocked (Rocks cost-reduction shell + various trash-N-cards triggered effects). N commits from `<first>`..`<last>`.
```

Update "Suggested next phase" to **Phase 6 (flood-gate + restriction modifiers)**.

- [ ] **Step 4: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_PYTHON_PARITY.md .claude/plans/recursive-coalescing-candle.md docs/superpowers/plans/2026-04-21-rust-engine-phase-5-cost-hooks.md
git commit -m "docs(phase-5): RUST_ENGINE_API/PARITY + roadmap — Phase 5 cost hooks landed"
```

---

## Verification

After all tasks land:

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green, +15 new tests beyond Phase 4's 499 (target ~514)
2. Grep for `_temp_play_cost_reduction` — zero matches in Rust sources (parity avoidance confirmed)
3. `docs/RUST_ENGINE_API.md` has a §Phase 5 section with 3 subsections
4. `docs/RUST_PYTHON_PARITY.md` has an entry for §5.1
5. `.claude/plans/recursive-coalescing-candle.md` Phase 5 row = ✅ LANDED

## Non-Goals (deferred)

- **`pay_cost_fn` installing selections.** v1 is synchronous. Scripts needing player choice for pay-cost (e.g., "choose 2 cards from hand to trash") fold that logic into `process` for now. Follow-up phase may add a `pay_cost_fn_async` variant that returns a continuation installing selections then calling back into the process.
- **Optional ("may") pay costs.** Card text like *"you may trash 2 cards to reduce cost"* requires an explicit RL-visible "accept / decline" selection before the pay fires. Requires the async selection path above. Defer.
- **Cost increase effects.** "Your opponent's Digimon cost 1 more" — semantically the inverse of cost reduction but requires a separate `cost_increase_fn` or a signed closure return. No audited meta card uses this pattern; defer until one appears.
- **Per-target digivolve cost reduction closures.** "Reduce digivolve cost by 2 if target has [trait]" — the read context needs the pre-digivolution target. Requires extending `EffectReadContext` with an optional `target: Option<PermanentHandle>` field. Flag as a follow-up; Task 2 can skip this sub-case for now and test simple "reduce by N" closures.
