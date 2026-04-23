use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use std::sync::Arc as StdArc;

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

#[test]
fn dsl_card_effect_accepts_raw_registry_and_stores_arc() {
    use digimon_dsl::compiled::{CompiledCard, CompiledCardKind};
    use digimon_engine::dsl_cards::DslCardEffect;
    use std::sync::Arc;

    let mut reg = EngineRawRustRegistry::new();
    reg.register_step("noop", |_| {});
    let reg = Arc::new(reg);

    let compiled = CompiledCard {
        card: "F".into(),
        name: "F".into(),
        kind: CompiledCardKind::Digimon,
        level: None,
        color: vec![],
        cost: None,
        dp: None,
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![],
    };
    let dsl = DslCardEffect::with_raw_registry(Arc::new(compiled), reg.clone());
    assert!(dsl.raw_registry().and_then(|r: &EngineRawRustRegistry| r.step_fn("noop")).is_some());
}

#[test]
fn raw_rust_step_invokes_registered_fn() {
    use digimon_dsl::compiled::CompiledStep;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::bindings::Bindings;
    use digimon_engine::effect_context::EffectContext;
    use std::sync::atomic::{AtomicBool, Ordering};

    static CALLED: AtomicBool = AtomicBool::new(false);
    CALLED.store(false, Ordering::SeqCst);

    let mut reg = EngineRawRustRegistry::new();
    reg.register_step("marker", |_ctx| {
        CALLED.store(true, Ordering::SeqCst);
    });
    let reg = StdArc::new(reg);

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();

    let step = CompiledStep::RawRust {
        fn_name: "marker".into(),
        consumes: vec![],
        binds: vec![],
    };
    let mut bindings = Bindings::new();
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        digimon_engine::dsl_cards::step::run_step_with_raw(
            &step,
            &mut ctx,
            &mut bindings,
            Some(reg.as_ref()),
        );
    }
    assert!(CALLED.load(Ordering::SeqCst));
}
