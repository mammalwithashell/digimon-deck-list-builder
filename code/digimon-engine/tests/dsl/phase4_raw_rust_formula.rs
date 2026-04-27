use std::sync::Arc;

use digimon_dsl::compiled::CompiledFormula;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::effect_context::EffectContext;

#[test]
fn raw_rust_formula_uses_registered_value() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_formula("stack_plus_five", |_ctx, _target| 12);
    let registry = Arc::new(registry);

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-FORMULA", "Raw Formula"))
        .build();
    let target = runner.place_on_field(0, "RAW-FORMULA", None);
    let source = runner.game.player(0).battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, source, Some(target), 0);

    let got = formula_eval::evaluate_with_raw(
        &CompiledFormula::RawRust("stack_plus_five".into()),
        &ctx,
        target,
        registry.as_ref(),
    );
    assert_eq!(got, 12);
}

#[test]
fn missing_raw_rust_formula_returns_zero() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-FORMULA", "Raw Formula"))
        .build();
    let target = runner.place_on_field(0, "RAW-FORMULA", None);
    let source = runner.game.player(0).battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, source, Some(target), 0);
    let raw = EngineRawRustRegistry::new();

    let got = formula_eval::evaluate_with_raw(
        &CompiledFormula::RawRust("missing".into()),
        &ctx,
        target,
        &raw,
    );
    assert_eq!(got, 0);
}
