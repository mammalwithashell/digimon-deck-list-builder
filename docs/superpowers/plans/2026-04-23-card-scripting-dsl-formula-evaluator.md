# Card Scripting DSL — Formula Evaluator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Lower `digimon_dsl::compiled::CompiledFormula` into a pure `Fn(&EffectReadContext, FormulaSubject) -> i32` evaluator, unblocking `amount_fn` on cost-reduction, variable-cost DigiXros (BT10-111 / BT12-112 / BT18-102), and DP-formula predicates.

**Architecture:** New engine module `digimon-engine/src/dsl_cards/formula.rs` exposing `eval_formula(&CompiledFormula, &EffectReadContext, FormulaSubject) -> i32`. Pure read-only computation — no engine state mutation. A small `FormulaSubject` enum mirrors `predicate::PredicateSubject` so aggregate/per-selector formulas can reference the current permanent (for `StackSize`, `MaterialCount`, etc.).

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

**Scope:** `CompiledFormula` variants — `Literal`, `BasePerDelta`, `FloorDiv`, `Max`, `Min`, `Aggregate`, `RawRust`. Per-selectors: `MaterialCount`, `StackSize`, `AllyCount`, `DigivolutionColorCount`, `CardCountInZone`. Aggregate selectors: `LowestDp`, `HighestDp`, `LowestLevel`, `HighestLevel`.

**Non-goals:** Wiring the evaluator into any clause yet — consumers (`lower_cost_reduction` amount_fn, DP-formula predicate arms, alt-path formula costs) are separate plans. This plan only ships the evaluator + unit tests.

---

## File structure

- Create: `digimon-engine/src/dsl_cards/formula.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (add `pub mod formula;`)
- Create: `digimon-engine/tests/dsl/formula.rs`
- Modify: `digimon-engine/tests/dsl/main.rs` (add `mod formula;`)

---

## Task 1: Literal + arithmetic (FloorDiv / Max / Min)

**Files:** `formula.rs`, `tests/dsl/formula.rs`

- [ ] **Step 1: Failing tests**

Create `digimon-engine/tests/dsl/formula.rs`:

```rust
use digimon_dsl::compiled::CompiledFormula;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula::{eval_formula, FormulaSubject};
use digimon_engine::effect_context::EffectReadContext;

fn fresh_rctx(runner: &DebugRunner) -> EffectReadContext<'_> {
    let card = runner.game.players[0].hand[0].handle();
    EffectReadContext::new(&runner.game, card, None, 0)
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build()
}

#[test]
fn literal_returns_value() {
    let r = runner();
    let rctx = fresh_rctx(&r);
    assert_eq!(eval_formula(&CompiledFormula::Literal(7), &rctx, FormulaSubject::None), 7);
}

#[test]
fn min_and_max_over_sub_formulas() {
    let r = runner();
    let rctx = fresh_rctx(&r);
    let f_min = CompiledFormula::Min(vec![
        CompiledFormula::Literal(3),
        CompiledFormula::Literal(7),
        CompiledFormula::Literal(5),
    ]);
    let f_max = CompiledFormula::Max(vec![
        CompiledFormula::Literal(3),
        CompiledFormula::Literal(7),
        CompiledFormula::Literal(5),
    ]);
    assert_eq!(eval_formula(&f_min, &rctx, FormulaSubject::None), 3);
    assert_eq!(eval_formula(&f_max, &rctx, FormulaSubject::None), 7);
}

#[test]
fn floor_div_divides_first_by_second_truncating_toward_zero() {
    let r = runner();
    let rctx = fresh_rctx(&r);
    let f = CompiledFormula::FloorDiv(vec![
        CompiledFormula::Literal(10),
        CompiledFormula::Literal(3),
    ]);
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::None), 3);
}
```

Add `mod formula;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run — expect FAIL**

```
cargo test --manifest-path digimon-engine/Cargo.toml --test dsl formula
```

- [ ] **Step 3: Implement arithmetic core**

Create `digimon-engine/src/dsl_cards/formula.rs`:

```rust
//! Pure formula evaluator — `CompiledFormula → i32` against a read-only
//! context. No engine-state mutation.

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledFormula, CompiledPerSelector,
};

use crate::effect_context::EffectReadContext;
use crate::permanent::PermanentHandle;

/// Subject for per-selector/aggregate variants. `None` means no specific
/// permanent is in scope (caller is at the effect-source level).
#[derive(Debug, Clone, Copy)]
pub enum FormulaSubject {
    Permanent(PermanentHandle),
    None,
}

pub fn eval_formula(
    f: &CompiledFormula,
    rctx: &EffectReadContext<'_>,
    subject: FormulaSubject,
) -> i32 {
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = resolve_per(*per, rctx, subject);
            base + (count as i32) * delta
        }
        CompiledFormula::FloorDiv(args) => {
            if args.len() != 2 {
                return 0;
            }
            let a = eval_formula(&args[0], rctx, subject);
            let b = eval_formula(&args[1], rctx, subject);
            if b == 0 { 0 } else { a / b }
        }
        CompiledFormula::Max(args) => args
            .iter()
            .map(|a| eval_formula(a, rctx, subject))
            .max()
            .unwrap_or(0),
        CompiledFormula::Min(args) => args
            .iter()
            .map(|a| eval_formula(a, rctx, subject))
            .min()
            .unwrap_or(0),
        CompiledFormula::Aggregate(sel) => resolve_aggregate(*sel, rctx),
        CompiledFormula::RawRust(_) => 0, // Phase 4 wires raw_rust dispatch.
    }
}

fn resolve_per(
    _sel: CompiledPerSelector,
    _rctx: &EffectReadContext<'_>,
    _subject: FormulaSubject,
) -> u32 {
    // Task 2 implements per-selectors. Return 0 so BasePerDelta degrades to
    // its base until Task 2 fills in the real count source.
    0
}

fn resolve_aggregate(
    _sel: CompiledAggregateSelector,
    _rctx: &EffectReadContext<'_>,
) -> i32 {
    // Task 3 implements aggregates.
    0
}
```

Add `pub mod formula;` to `digimon-engine/src/dsl_cards/mod.rs`.

- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit**

```
git add digimon-engine/src/dsl_cards/formula.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/formula.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl: formula evaluator — literal + FloorDiv + Max + Min"
```

---

## Task 2: Per-selectors (MaterialCount / StackSize / AllyCount / DigivolutionColorCount / CardCountInZone)

**Files:** `formula.rs`, `tests/dsl/formula.rs`

- [ ] **Step 1: Failing tests**

Append to `tests/dsl/formula.rs`:

```rust
use digimon_dsl::compiled::CompiledPerSelector;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn base_per_delta_uses_material_count_when_subject_is_permanent_with_stack() {
    // Build a runner with a 3-high stack on player 0's field, then form
    // a PermanentHandle pointing at it.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F", "F", "F"])
        .build();
    // Move all 3 hand cards into one permanent as a digivolution stack.
    let cs0 = runner.game.players[0].hand.remove(0);
    let cs1 = runner.game.players[0].hand.remove(0);
    let cs2 = runner.game.players[0].hand.remove(0);
    runner.game.players[0].battle_area.push(
        digimon_engine::permanent::Permanent {
            card_sources: vec![cs0, cs1, cs2],
            ..Default::default()
        }
    );
    let h = PermanentHandle { player: 0, index: 0 };
    let card = runner.game.players[0].battle_area[0].top_card().handle();
    let rctx = EffectReadContext::new(&runner.game, card, Some(h), 0);

    // base 5 + material_count(3) * -1 = 2
    // material_count = stack_size - 1 (excludes the top card itself)
    let f = CompiledFormula::BasePerDelta {
        base: 5,
        per: CompiledPerSelector::MaterialCount,
        delta: -1,
    };
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::Permanent(h)), 3);
}

#[test]
fn stack_size_returns_full_stack_including_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F", "F"])
        .build();
    let cs0 = runner.game.players[0].hand.remove(0);
    let cs1 = runner.game.players[0].hand.remove(0);
    runner.game.players[0].battle_area.push(
        digimon_engine::permanent::Permanent {
            card_sources: vec![cs0, cs1],
            ..Default::default()
        }
    );
    let h = PermanentHandle { player: 0, index: 0 };
    let card = runner.game.players[0].battle_area[0].top_card().handle();
    let rctx = EffectReadContext::new(&runner.game, card, Some(h), 0);

    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::StackSize,
        delta: 1,
    };
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::Permanent(h)), 2);
}
```

**Note to implementer:** Verify `Permanent`'s default constructor exists. If not, look at prior Phase 1c `any_permanent_matches_if_any_battle_area_perm_matches` test — the agent found `runner.place_on_field(...)` as the right helper. Use that instead of manual `Permanent` construction.

- [ ] **Step 2: Run — FAIL.**

- [ ] **Step 3: Implement `resolve_per`**

Replace `resolve_per` in `formula.rs`:

```rust
fn resolve_per(
    sel: CompiledPerSelector,
    rctx: &EffectReadContext<'_>,
    subject: FormulaSubject,
) -> u32 {
    match sel {
        CompiledPerSelector::MaterialCount => subject_stack(rctx, subject)
            .map(|n| n.saturating_sub(1)) // top card excluded
            .unwrap_or(0),
        CompiledPerSelector::StackSize => subject_stack(rctx, subject).unwrap_or(0),
        CompiledPerSelector::AllyCount => {
            // Count Digimon on rctx.player's battle area excluding the subject.
            let subject_idx = match subject {
                FormulaSubject::Permanent(h) if h.player == rctx.player => Some(h.index),
                _ => None,
            };
            let n = rctx.game.player(rctx.player).battle_area.len();
            let mut count = 0u32;
            for i in 0..n {
                if subject_idx == Some(i as u8) { continue; }
                count += 1;
            }
            count
        }
        CompiledPerSelector::DigivolutionColorCount => {
            let Some(h) = match subject {
                FormulaSubject::Permanent(h) => Some(h),
                _ => None,
            } else { return 0; };
            let Some(perm) = rctx.game.player(h.player).battle_area.get(h.index as usize) else { return 0; };
            let mut colors: std::collections::HashSet<crate::enums::CardColor> = Default::default();
            for cs in &perm.card_sources {
                for c in cs.colors(&rctx.game.card_data) {
                    colors.insert(*c);
                }
            }
            colors.len() as u32
        }
        CompiledPerSelector::CardCountInZone => {
            // Defaults to the acting player's hand when no zone annotation
            // is available at the formula level. Phase 2+ refines this when
            // the formula carries a zone tag.
            rctx.game.player(rctx.player).hand.len() as u32
        }
    }
}

fn subject_stack(
    rctx: &EffectReadContext<'_>,
    subject: FormulaSubject,
) -> Option<u32> {
    let FormulaSubject::Permanent(h) = subject else { return None; };
    let perm = rctx.game.player(h.player).battle_area.get(h.index as usize)?;
    Some(perm.card_sources.len() as u32)
}
```

- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit**

```
git add digimon-engine/src/dsl_cards/formula.rs digimon-engine/tests/dsl/formula.rs
git commit -m "dsl: formula — per-selectors (MaterialCount/StackSize/AllyCount/DigivolutionColorCount/CardCountInZone)"
```

---

## Task 3: Aggregate selectors (LowestDp / HighestDp / LowestLevel / HighestLevel)

**Files:** `formula.rs`, `tests/dsl/formula.rs`

- [ ] **Step 1: Failing test**

Append tests exercising LowestDp/HighestDp/LowestLevel/HighestLevel against a battle area with known DP/level cards. Build 2-3 permanents using `runner.place_on_field` (verify helper name) and assert the aggregate matches the expected min/max.

- [ ] **Step 2: Implement `resolve_aggregate`**

```rust
fn resolve_aggregate(
    sel: CompiledAggregateSelector,
    rctx: &EffectReadContext<'_>,
) -> i32 {
    let players = 0..rctx.game.players.len() as crate::enums::PlayerId;
    let mut dp: Vec<i32> = Vec::new();
    let mut lv: Vec<u8> = Vec::new();
    for p in players {
        for perm in &rctx.game.player(p).battle_area {
            let top = perm.top_card();
            if let Some(d) = top.dp(&rctx.game.card_data) { dp.push(d); }
            if let Some(l) = top.level(&rctx.game.card_data) { lv.push(l); }
        }
    }
    match sel {
        CompiledAggregateSelector::LowestDp => dp.into_iter().min().unwrap_or(0),
        CompiledAggregateSelector::HighestDp => dp.into_iter().max().unwrap_or(0),
        CompiledAggregateSelector::LowestLevel => lv.into_iter().min().unwrap_or(0) as i32,
        CompiledAggregateSelector::HighestLevel => lv.into_iter().max().unwrap_or(0) as i32,
    }
}
```

- [ ] **Step 3: Commit**

```
git add digimon-engine/src/dsl_cards/formula.rs digimon-engine/tests/dsl/formula.rs
git commit -m "dsl: formula — aggregate selectors (LowestDp/HighestDp/LowestLevel/HighestLevel)"
```

---

## Task 4: BasePerDelta + nested composition smoke test

**Files:** `tests/dsl/formula.rs`

- [ ] **Step 1: Test nested formula composition**

Add a test exercising `FloorDiv([BasePerDelta{...}, Literal(2)])` and `Max([Aggregate(LowestDp), Literal(1000)])` to ensure recursion works.

- [ ] **Step 2: Commit**

```
git add digimon-engine/tests/dsl/formula.rs
git commit -m "dsl: formula — nested composition test"
```

---

## Task 5: Exit test — full formula surface exercised

Add a final test that builds each `CompiledFormula` variant once and asserts `eval_formula` returns without panic. Ensures every match arm is live.

```
git commit -m "dsl: formula — exhaustive variant coverage smoke test"
```

---

## Self-Review

**Spec coverage:**
- Literal, FloorDiv, Max, Min — Task 1
- Per-selectors — Task 2
- Aggregate selectors — Task 3
- Composition — Task 4
- Exhaustive — Task 5

**Explicit deferrals:**
- `RawRust` formula body returns 0 until Phase 4 raw_rust dispatch lands
- Consumers (cost_reduction.amount_fn, DP-formula predicates, alt-path variable cost) are separate plans — do NOT wire this evaluator into them in this plan

**Type consistency:** `eval_formula(&CompiledFormula, &EffectReadContext, FormulaSubject) -> i32`. `FormulaSubject::{Permanent, None}`.
