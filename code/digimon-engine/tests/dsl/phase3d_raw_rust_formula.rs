use digimon_dsl::compiled::CompiledFormula;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::dsl_cards::formula_registry::{FormulaExtensionFn, FormulaExtensionRegistry};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

fn raw_formula_value(_ctx: &EffectContext<'_>, target: PermanentHandle) -> i32 {
    4200 + target.index as i32 + 42
}

fn test_registry() -> FormulaExtensionRegistry {
    FormulaExtensionRegistry::with_entries([(
        "phase3d_raw_formula",
        raw_formula_value as FormulaExtensionFn,
    )])
}

#[test]
fn raw_rust_formula_dispatches_registered_callback() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .build();
    let target = runner.place_on_field(0, "SRC", None);
    runner.set_formula_extensions(test_registry());

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::RawRust("phase3d_raw_formula".to_string());

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 4242);
}

#[test]
fn unknown_raw_rust_formula_evaluates_to_zero() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .build();
    let target = runner.place_on_field(0, "SRC", None);
    runner.set_formula_extensions(test_registry());

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::RawRust("missing_raw_formula".to_string());

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 0);
}
