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
    r.register_step("noop_step", |_ctx, _bindings| {});
    assert!(r.step_fn("noop_step").is_some());
    assert!(r.step_fn("missing").is_none());
}

#[test]
fn register_step_twice_overwrites_and_warns() {
    // We can't easily capture eprintln! from a test, so verify the behavioral
    // contract: the second registration wins (last-write semantics). The
    // warning path itself is exercised here — running `cargo test` with
    // --nocapture shows the WARNING line emitted by register_step.
    use std::sync::atomic::{AtomicU8, Ordering};
    static TAG: AtomicU8 = AtomicU8::new(0);
    TAG.store(0, Ordering::SeqCst);

    let mut r = EngineRawRustRegistry::new();
    r.register_step("dupe", |_ctx, _bindings| TAG.store(1, Ordering::SeqCst));
    assert_eq!(r.step_count(), 1);
    r.register_step("dupe", |_ctx, _bindings| TAG.store(2, Ordering::SeqCst));
    // Count stays at 1 — same key was replaced, not added.
    assert_eq!(r.step_count(), 1);

    // Invoke the registered fn; it must be the second one (tag = 2).
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::bindings::Bindings;
    use digimon_engine::effect_context::EffectContext;
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let f = r.step_fn("dupe").expect("present");
    let mut bindings = Bindings::new();
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        f(&mut ctx, &mut bindings);
    }
    assert_eq!(TAG.load(Ordering::SeqCst), 2, "second registration wins");
}

#[test]
fn register_declarative_twice_overwrites() {
    use digimon_engine::card_source::CardHandle;
    let mut r = EngineRawRustRegistry::new();
    r.register_declarative("dupe", |_card: CardHandle| Vec::new());
    assert_eq!(r.declarative_count(), 1);
    r.register_declarative("dupe", |_card: CardHandle| Vec::new());
    assert_eq!(r.declarative_count(), 1);
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
    reg.register_step("noop", |_, _| {});
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
    reg.register_step("marker", |_ctx, _bindings| {
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

#[test]
fn raw_rust_step_fires_through_full_triggered_lowering_path() {
    // Integration-style: build a CompiledCard whose only effect is a
    // Triggered OnPlay clause whose process is a single CompiledStep::RawRust.
    // Wrap it in DslCardEffect::with_raw_registry, call effects(card), then
    // invoke the resulting Effect's process closure. Must fire the raw fn.
    // This covers the Arc clone into the process closure and the dispatch
    // through lower_triggered::lower -> run_step_with_raw.
    use digimon_dsl::compiled::{
        CompiledCard, CompiledCardKind, CompiledClause, CompiledStep, CompiledScope,
        CompiledTiming, CompiledTriggeredClause,
    };
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::DslCardEffect;
    use digimon_engine::effect::CardEffect;
    use digimon_engine::effect_context::EffectContext;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;

    static TAG: AtomicU8 = AtomicU8::new(0);
    TAG.store(0, Ordering::SeqCst);

    let mut reg = EngineRawRustRegistry::new();
    reg.register_step("e2e_marker", |_ctx, _bindings| {
        TAG.store(42, Ordering::SeqCst);
    });
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
        effects: vec![CompiledClause::Triggered(CompiledTriggeredClause {
            when: vec![CompiledTiming::OnPlay],
            scope: CompiledScope::FaceUp,
            active_when: None,
            condition: None,
            optional: false,
            once_per_turn: false,
            max_per_turn: None,
            process: vec![CompiledStep::RawRust {
                fn_name: "e2e_marker".into(),
                consumes: vec![],
                binds: vec![],
            }],
            summary: None,
            summary_key: None,
        })],
    };

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();

    let dsl = DslCardEffect::with_raw_registry(Arc::new(compiled), reg);
    let effects = <DslCardEffect as CardEffect>::effects(&dsl, card);
    assert_eq!(effects.len(), 1, "one Effect emitted for OnPlay trigger");
    assert_eq!(
        effects[0].timing,
        digimon_engine::enums::EffectTiming::OnPlay,
    );

    // Drive the process closure end-to-end.
    let process = effects[0]
        .process
        .as_ref()
        .expect("triggered lowering attaches a process closure");
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }
    assert_eq!(
        TAG.load(Ordering::SeqCst),
        42,
        "raw_rust step fn fired via DslCardEffect::effects() -> lower_triggered -> run_step_with_raw",
    );
}

#[test]
fn raw_rust_declarative_clause_returns_fn_produced_effects() {
    use digimon_dsl::compiled::{
        CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
        CompiledScope, CompiledTiming,
    };
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::dsl_cards::DslCardEffect;
    use digimon_engine::effect::{CardEffect, Effect};
    use std::sync::Arc;

    let mut reg = EngineRawRustRegistry::new();
    reg.register_declarative("emits_onplay", |card: CardHandle| {
        vec![Effect::on_play(card).name("raw").process(|_| {}).build()]
    });
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
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::RawRust {
                fn_name: "emits_onplay".into(),
                triggers: vec![CompiledTiming::OnPlay],
                scope: CompiledScope::FaceUp,
                summary: None,
                summary_key: None,
            },
        )],
    };
    let dsl = DslCardEffect::with_raw_registry(Arc::new(compiled), reg);
    let effects = <DslCardEffect as CardEffect>::effects(&dsl, CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].timing,
        digimon_engine::enums::EffectTiming::OnPlay,
    );
}

#[test]
fn budget_zero_card_count_does_not_exceed() {
    use digimon_engine::dsl_cards::raw_rust::exceeds_budget;
    assert!(!exceeds_budget(0, 0));
    assert!(!exceeds_budget(5, 0));
}

#[test]
fn budget_under_three_percent_does_not_exceed() {
    use digimon_engine::dsl_cards::raw_rust::exceeds_budget;
    // 3 / 100 = exactly 3% — should NOT exceed.
    assert!(!exceeds_budget(3, 100));
    // 2 / 100 = 2%.
    assert!(!exceeds_budget(2, 100));
}

#[test]
fn budget_over_three_percent_exceeds() {
    use digimon_engine::dsl_cards::raw_rust::exceeds_budget;
    // 4 / 100 = 4% — exceeds.
    assert!(exceeds_budget(4, 100));
    // 10 / 50 = 20%.
    assert!(exceeds_budget(10, 50));
}
