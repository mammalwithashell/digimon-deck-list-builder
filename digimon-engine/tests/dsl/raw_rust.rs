use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;

#[test]
fn empty_registry_reports_no_fns() {
    let r = EngineRawRustRegistry::new();
    assert!(r.step_fn("anything").is_none());
    assert!(r.declarative_fn("anything").is_none());
}

#[test]
fn register_and_lookup_step_fn() {
    let mut r = EngineRawRustRegistry::new();
    r.register_step("noop_step", |_ctx| {});
    assert!(r.step_fn("noop_step").is_some());
    assert!(r.step_fn("missing").is_none());
}

#[test]
fn register_and_lookup_declarative_fn() {
    use digimon_engine::card_source::CardHandle;
    let mut r = EngineRawRustRegistry::new();
    r.register_declarative("noop_decl", |_card: CardHandle| Vec::new());
    assert!(r.declarative_fn("noop_decl").is_some());
}
