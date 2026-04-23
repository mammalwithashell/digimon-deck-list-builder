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
