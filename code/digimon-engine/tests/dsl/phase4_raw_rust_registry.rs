use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::effect::Effect;

#[test]
fn empty_registry_reports_missing_functions() {
    let registry = EngineRawRustRegistry::new();
    assert!(registry.step_fn("missing").is_none());
    assert!(registry.declarative_fn("missing").is_none());
    assert!(registry.formula_fn("missing").is_none());
    assert!(!registry.contains_fn("missing"));
    assert_eq!(registry.registered_fn_count(), 0);
}

#[test]
fn registry_registers_all_three_raw_rust_shapes() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("mark_step", |_ctx, bindings| {
        bindings.insert_literal("called", 1);
    });
    registry.register_declarative("emit_clause", |card: CardHandle| {
        vec![Effect::on_play(card)
            .name("raw clause")
            .process(|_| {})
            .build()]
    });
    registry.register_formula("formula_value", |_ctx, _target| 7);

    assert!(registry.step_fn("mark_step").is_some());
    assert!(registry.declarative_fn("emit_clause").is_some());
    assert!(registry.formula_fn("formula_value").is_some());
    assert!(registry.contains_fn("mark_step"));
    assert!(registry.contains_fn("emit_clause"));
    assert!(registry.contains_fn("formula_value"));
    assert_eq!(registry.registered_fn_count(), 3);
}

#[test]
fn registry_debug_prints_counts_without_closure_values() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("noop", |_ctx, _bindings| {});
    let text = format!("{registry:?}");
    assert!(text.contains("EngineRawRustRegistry"));
    assert!(text.contains("steps"));
    assert!(text.contains("1"));
}
