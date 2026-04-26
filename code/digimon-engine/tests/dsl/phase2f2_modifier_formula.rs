//! Phase 2f2 Task 3 — formula-valued `AddDpModifier` / `AddModifier` lower
//! through `formula_eval::evaluate`.
//!
//! Coverage:
//!   1. `AddDpModifier` with `Formula(BasePerDelta { per: StackSize, delta })`
//!      resolves against the bound target's stack size.
//!   2. `AddModifier` with `Filter(...)` target evaluates the formula PER MATCH
//!      — a permanent with stack size 2 vs. a permanent with stack size 4
//!      receive different stored modifier values (load-bearing for the
//!      "no hoisting outside the for-loop" invariant).
//!   3. `AddDpModifier` with `Literal(_)` is unchanged (regression guard for
//!      Task 1's transitional shape).
//!   4. Large formula values are passed through verbatim — `add_dp_modifier`
//!      and `add_modifier` both take `i32`, so no narrowing/saturation is
//!      necessary at this boundary. (Documented here so a future modifier
//!      width change forces this test to fail.)

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledFormula, CompiledModifierTarget,
    CompiledModifierValue, CompiledPerSelector, CompiledPredicate, CompiledStep,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::ModifierType;

/// Push N-1 extra `CardSource`s onto a permanent so its stack_size == `target`.
fn pad_stack_to(runner: &mut DebugRunner, player: u8, perm_index: usize, target: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card_id registered");
    while runner.game.players[player as usize].battle_area[perm_index]
        .card_sources
        .len()
        < target
    {
        let next = runner.game.next_card_index();
        let src = CardSource::new(data_idx, player, next);
        runner.game.players[player as usize].battle_area[perm_index]
            .card_sources
            .push(src);
    }
}

/// Test 1 — formula-valued `AddDpModifier` resolves against the bound
/// target's stack size. Build a permanent with stack_size = 3 and apply
/// `BasePerDelta { base: 0, per: StackSize, delta: 1000 }`. Effective DP
/// should grow by 3 * 1000 = 3000.
#[test]
fn add_dp_modifier_formula_value_resolves_against_target_stack_size() {
    let card = make_test_card("T-FDP", "T-FDP");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["T-FDP"])
        .build();

    let handle = runner.place_on_field(0, "T-FDP", None);
    pad_stack_to(&mut runner, 0, 0, 3, "T-FDP");
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 3);

    let base_dp = runner.effective_dp(handle).expect("base DP");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: CompiledModifierValue::Formula(CompiledFormula::BasePerDelta {
            base: 0,
            per: CompiledPerSelector::StackSize,
            delta: 1000,
        }),
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&[step], &mut ctx, &mut bindings);
    }

    let after_dp = runner.effective_dp(handle).expect("permanent still on field");
    assert_eq!(
        after_dp,
        base_dp + 3000,
        "formula 0 + 3 (stack) * 1000 = 3000 should be added to base DP",
    );
}

/// Test 2 — LOAD-BEARING. `AddModifier` with `Filter(...)` evaluates the
/// formula per match, NOT once outside the loop. Two digimon on P0 with
/// different stack sizes (2 and 4) and a `BasePerDelta { per: StackSize }`
/// formula should receive different stored modifier values.
#[test]
fn add_modifier_filter_target_formula_evaluated_per_match() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("PA", "PA"))
        .add_card(make_test_card("PB", "PB"))
        .hand(0, &["SRC"])
        .build();

    let h_a = runner.place_on_field(0, "PA", None);
    let h_b = runner.place_on_field(0, "PB", None);
    pad_stack_to(&mut runner, 0, 0, 2, "PA"); // perm A: stack_size = 2
    pad_stack_to(&mut runner, 0, 1, 4, "PB"); // perm B: stack_size = 4

    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 2);
    assert_eq!(runner.game.players[0].battle_area[1].card_sources.len(), 4);

    let src_card = runner.game.players[0].hand[0].handle();

    // Filter matches both digimon on the field. `CannotAttack` is used here
    // because it's exposed in `lookup_modifier_type` AND its registry entry
    // stores the supplied numeric `value` (boolean semantics from the engine
    // perspective, but `ModifierEntry.value` is i32 and observable).
    let step = CompiledStep::AddModifier {
        target: CompiledModifierTarget::Filter(CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            ..CompiledPredicate::default()
        }),
        modifier: "CannotAttack".into(),
        value: CompiledModifierValue::Formula(CompiledFormula::BasePerDelta {
            base: 0,
            per: CompiledPerSelector::StackSize,
            delta: 1,
        }),
        expiry: "EndOfTurn".into(),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&[step], &mut ctx, &mut bindings);
    }

    // Per-match evaluation: A gets value=2, B gets value=4. If the formula
    // were hoisted outside the loop, both would carry the same value (likely
    // the source's stack — 1 — or whichever target was passed).
    let val_a = runner.game.modifiers.sum(h_a, ModifierType::CannotAttack);
    let val_b = runner.game.modifiers.sum(h_b, ModifierType::CannotAttack);
    assert_eq!(val_a, 2, "perm A (stack_size=2) should carry value 2");
    assert_eq!(val_b, 4, "perm B (stack_size=4) should carry value 4");
    assert_ne!(
        val_a, val_b,
        "values must differ per match — proves per-iteration evaluation",
    );
}

/// Test 3 — Regression guard: `AddDpModifier` with `Literal(_)` continues
/// to behave exactly as before Task 3. Effective DP grows by the literal.
#[test]
fn add_dp_modifier_literal_arm_unchanged() {
    let card = make_test_card("T-LIT", "T-LIT");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["T-LIT"])
        .build();

    let handle = runner.place_on_field(0, "T-LIT", None);
    let base_dp = runner.effective_dp(handle).expect("base DP");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: CompiledModifierValue::Literal(3000),
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&[step], &mut ctx, &mut bindings);
    }

    let after_dp = runner.effective_dp(handle).expect("permanent still on field");
    assert_eq!(
        after_dp,
        base_dp + 3000,
        "literal 3000 should still be added verbatim (no behavior change vs. pre-Task-3)",
    );
}

/// Test 4 — Large formula values (above what `i16` would hold) pass through
/// untruncated. Both `add_dp_modifier` and `add_modifier` take `i32`, so no
/// narrowing happens at this boundary. If a future engine change narrows
/// the modifier value type, this test will fail loudly and force the author
/// to add explicit saturation in `step/modifiers.rs`.
#[test]
fn add_dp_modifier_formula_large_value_passes_through() {
    let card = make_test_card("T-BIG", "T-BIG");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["T-BIG"])
        .build();

    let handle = runner.place_on_field(0, "T-BIG", None);
    let base_dp = runner.effective_dp(handle).expect("base DP");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    // 40000 > i16::MAX (32767). Engine takes i32, so this should pass through.
    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: CompiledModifierValue::Formula(CompiledFormula::Literal(40000)),
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&[step], &mut ctx, &mut bindings);
    }

    let after_dp = runner.effective_dp(handle).expect("permanent still on field");
    assert_eq!(
        after_dp,
        base_dp + 40000,
        "value > i16::MAX should pass through — engine uses i32",
    );
}
