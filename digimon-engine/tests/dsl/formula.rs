use digimon_dsl::compiled::{CompiledFormula, CompiledPerSelector};
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
