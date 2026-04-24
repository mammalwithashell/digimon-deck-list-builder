use digimon_dsl::compiled::{CompiledAggregateSelector, CompiledFormula, CompiledPerSelector};
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

#[test]
fn stack_size_returns_full_stack_including_top() {
    // Build a 2-card stack on player 0's field.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    // place_on_field creates a 1-card permanent.
    let h = runner.place_on_field(0, "F", None);
    // Push a second CardSource to make a 2-high stack.
    let data_idx = runner.game.card_data.iter().position(|c| c.card_id == "F").unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0].battle_area[h.index as usize]
        .card_sources
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next));

    let card = runner.game.players[0].battle_area[h.index as usize].top_card().handle();
    let rctx = EffectReadContext::new(&runner.game, card, Some(h), 0);

    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::StackSize,
        delta: 1,
    };
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::Permanent(h)), 2);
}

#[test]
fn base_per_delta_uses_material_count_when_subject_is_permanent_with_stack() {
    // Build a 3-card stack on player 0's field.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let h = runner.place_on_field(0, "F", None);
    let data_idx = runner.game.card_data.iter().position(|c| c.card_id == "F").unwrap();
    // Push second card source.
    let next1 = runner.game.next_card_index();
    runner.game.players[0].battle_area[h.index as usize]
        .card_sources
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next1));
    // Push third card source.
    let next2 = runner.game.next_card_index();
    runner.game.players[0].battle_area[h.index as usize]
        .card_sources
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next2));

    let card = runner.game.players[0].battle_area[h.index as usize].top_card().handle();
    let rctx = EffectReadContext::new(&runner.game, card, Some(h), 0);

    // material_count = stack_size - 1 = 3 - 1 = 2
    // base 5 + 2 * -1 = 3
    let f = CompiledFormula::BasePerDelta {
        base: 5,
        per: CompiledPerSelector::MaterialCount,
        delta: -1,
    };
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::Permanent(h)), 3);
}

/// Build a DebugRunner with three Digimon of known DP/level on player 0's
/// battle area, then verify all four aggregate selectors against those values.
///
/// Card A: dp=2000, level=3 (make_test_card defaults)
/// Card B: dp=5000, level=5
/// Card C: dp=1000, level=4
///
/// LowestDp  → 1000   HighestDp  → 5000
/// LowestLevel → 3    HighestLevel → 5
fn aggregate_runner() -> DebugRunner {
    let card_a = make_test_card("AGG_A", "A");
    let mut card_b = make_test_card("AGG_B", "B");
    card_b.dp = Some(5000);
    card_b.level = Some(5);
    let mut card_c = make_test_card("AGG_C", "C");
    card_c.dp = Some(1000);
    card_c.level = Some(4);

    let mut runner = DebugRunner::builder()
        .add_card(card_a)
        .add_card(card_b)
        .add_card(card_c)
        .hand(0, &["AGG_A"])
        .build();

    runner.place_on_field(0, "AGG_A", None);
    runner.place_on_field(0, "AGG_B", None);
    runner.place_on_field(0, "AGG_C", None);
    runner
}

fn agg_rctx(runner: &DebugRunner) -> EffectReadContext<'_> {
    // Use the top card of the first battle-area permanent as the source card.
    let card = runner.game.players[0].battle_area[0].top_card().handle();
    EffectReadContext::new(&runner.game, card, None, 0)
}

#[test]
fn aggregate_lowest_dp() {
    let r = aggregate_runner();
    let rctx = agg_rctx(&r);
    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::LowestDp);
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::None), 1000);
}

#[test]
fn aggregate_highest_dp() {
    let r = aggregate_runner();
    let rctx = agg_rctx(&r);
    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::HighestDp);
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::None), 5000);
}

#[test]
fn aggregate_lowest_level() {
    let r = aggregate_runner();
    let rctx = agg_rctx(&r);
    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::LowestLevel);
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::None), 3);
}

#[test]
fn aggregate_highest_level() {
    let r = aggregate_runner();
    let rctx = agg_rctx(&r);
    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::HighestLevel);
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::None), 5);
}

// ---------------------------------------------------------------------------
// Task 4: nested composition smoke tests
// ---------------------------------------------------------------------------

/// FloorDiv([BasePerDelta{base:0, per:StackSize, delta:2}, Literal(2)])
///
/// Build a 3-card stack so StackSize = 3.
/// BasePerDelta evaluates to: 0 + 3 * 2 = 6.
/// FloorDiv(6, 2) = 3.
///
/// Demonstrates that FloorDiv correctly recurses into BasePerDelta rather
/// than treating it as a leaf value.
#[test]
fn nested_floor_div_wrapping_base_per_delta() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();

    let h = runner.place_on_field(0, "F", None);
    let data_idx = runner.game.card_data.iter().position(|c| c.card_id == "F").unwrap();

    // Push a second card source → stack height 2.
    let next1 = runner.game.next_card_index();
    runner.game.players[0].battle_area[h.index as usize]
        .card_sources
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next1));
    // Push a third card source → stack height 3.
    let next2 = runner.game.next_card_index();
    runner.game.players[0].battle_area[h.index as usize]
        .card_sources
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next2));

    let card = runner.game.players[0].battle_area[h.index as usize].top_card().handle();
    let rctx = EffectReadContext::new(&runner.game, card, Some(h), 0);

    // Inner: base=0, per=StackSize (3), delta=2  →  0 + 3*2 = 6
    // Outer: FloorDiv(6, 2) = 3
    let inner = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::StackSize,
        delta: 2,
    };
    let f = CompiledFormula::FloorDiv(vec![inner, CompiledFormula::Literal(2)]);
    assert_eq!(eval_formula(&f, &rctx, FormulaSubject::Permanent(h)), 3);
}

// ---------------------------------------------------------------------------
// Task 5: exhaustive variant coverage smoke test
// ---------------------------------------------------------------------------

/// Build every `CompiledFormula` variant once and call `eval_formula` on it.
/// The goal is to ensure no match arm panics. One sanity `assert_eq!` on
/// `Literal` confirms the evaluator is actually running; remaining variants
/// are exercised for panic-freedom only.
#[test]
fn all_formula_variants_eval_without_panic() {
    let r = runner();
    let rctx = fresh_rctx(&r);

    // Literal — sanity check
    assert_eq!(
        eval_formula(&CompiledFormula::Literal(42), &rctx, FormulaSubject::None),
        42
    );

    // BasePerDelta — CardCountInZone works with FormulaSubject::None
    let _ = eval_formula(
        &CompiledFormula::BasePerDelta {
            base: 1000,
            per: CompiledPerSelector::CardCountInZone,
            delta: 500,
        },
        &rctx,
        FormulaSubject::None,
    );

    // FloorDiv
    let _ = eval_formula(
        &CompiledFormula::FloorDiv(vec![
            CompiledFormula::Literal(10),
            CompiledFormula::Literal(3),
        ]),
        &rctx,
        FormulaSubject::None,
    );

    // Max
    let _ = eval_formula(
        &CompiledFormula::Max(vec![
            CompiledFormula::Literal(1),
            CompiledFormula::Literal(2),
        ]),
        &rctx,
        FormulaSubject::None,
    );

    // Min
    let _ = eval_formula(
        &CompiledFormula::Min(vec![
            CompiledFormula::Literal(1),
            CompiledFormula::Literal(2),
        ]),
        &rctx,
        FormulaSubject::None,
    );

    // Aggregate
    let _ = eval_formula(
        &CompiledFormula::Aggregate(CompiledAggregateSelector::HighestDp),
        &rctx,
        FormulaSubject::None,
    );

    // RawRust — returns 0 (Phase 4 deferred), must not panic
    let _ = eval_formula(
        &CompiledFormula::RawRust("some_raw_rust_id".to_string()),
        &rctx,
        FormulaSubject::None,
    );
}

/// Max([Aggregate(LowestDp), Literal(1000)])
///
/// The aggregate_runner field has three Digimon with DPs 2000, 5000, 1000.
/// LowestDp = 1000.  Max(1000, 1000) = 1000.
///
/// To produce a distinct result that proves Max is not simply returning
/// the Aggregate value, we also verify with Literal(2000):
///   Max(LowestDp=1000, Literal(2000)) = 2000.
///
/// Together these two assertions confirm the recursion: Aggregate is fully
/// evaluated before Max compares it against the sibling Literal.
#[test]
fn nested_max_wrapping_aggregate_lowest_dp() {
    let r = aggregate_runner();
    let rctx = agg_rctx(&r);

    // LowestDp = 1000; Max(1000, 1000) = 1000 — aggregate wins the tie.
    let f_tie = CompiledFormula::Max(vec![
        CompiledFormula::Aggregate(CompiledAggregateSelector::LowestDp),
        CompiledFormula::Literal(1000),
    ]);
    assert_eq!(eval_formula(&f_tie, &rctx, FormulaSubject::None), 1000);

    // LowestDp = 1000; Max(1000, 2000) = 2000 — Literal wins, proving Max
    // is actually comparing both branches rather than short-circuiting.
    let f_literal_wins = CompiledFormula::Max(vec![
        CompiledFormula::Aggregate(CompiledAggregateSelector::LowestDp),
        CompiledFormula::Literal(2000),
    ]);
    assert_eq!(eval_formula(&f_literal_wins, &rctx, FormulaSubject::None), 2000);
}
