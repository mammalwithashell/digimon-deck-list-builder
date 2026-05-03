use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::{CardEffect, CardHandle, Effect, PermanentHandle};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct SourceRecordingWhenDigivolving {
    seen: Arc<Mutex<Vec<(CardHandle, Option<PermanentHandle>)>>>,
}

impl CardEffect for SourceRecordingWhenDigivolving {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::when_digivolving(card)
            .name("record source and gain memory")
            .process(move |ctx| {
                ctx.gain_memory(1);
                seen.lock()
                    .unwrap()
                    .push((ctx.source_card, ctx.source_permanent));
            })
            .build()]
    }
}

#[derive(Clone)]
struct TwoWhenDigivolvingEffects;

impl CardEffect for TwoWhenDigivolvingEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::when_digivolving(card)
                .name("gain one")
                .process(|ctx| ctx.gain_memory(1))
                .build(),
            Effect::when_digivolving(card)
                .name("gain three")
                .process(|ctx| ctx.gain_memory(3))
                .build(),
        ]
    }
}

#[test]
fn refire_selected_when_digivolving_effect_preserves_source_identity() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("REFIRE-TARGET", "Refire Target"))
        .memory(0)
        .start();
    r.register_effect(
        "REFIRE-TARGET",
        Arc::new(SourceRecordingWhenDigivolving {
            seen: Arc::clone(&seen),
        }),
    );

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "REFIRE-TARGET", Some(0));
    let caller_source = r.top_card(source_stack);
    let target_source = r.top_card(target);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        ctx.refire_effect_from_permanent(target, "when_digivolving", false)
            .expect("refire effect");
    }

    assert_eq!(r.memory(), 1);
    assert_eq!(*seen.lock().unwrap(), vec![(target_source, Some(target))]);
}

#[test]
fn optional_refire_can_be_declined_or_accepted() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("REFIRE-TARGET", "Refire Target"))
        .memory(0)
        .start();
    r.register_effect(
        "REFIRE-TARGET",
        Arc::new(SourceRecordingWhenDigivolving {
            seen: Arc::clone(&seen),
        }),
    );

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "REFIRE-TARGET", Some(0));
    let caller_source = r.top_card(source_stack);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        ctx.refire_effect_from_permanent(target, "when_digivolving", true)
            .expect("install optional refire choice");
    }

    assert!(r
        .game
        .pending_selection
        .as_ref()
        .is_some_and(|s| s.is_optional));
    r.game.resolve_selection(0, PASS).expect("decline refire");
    assert_eq!(r.memory(), 0);
    assert!(seen.lock().unwrap().is_empty());

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        ctx.refire_effect_from_permanent(target, "when_digivolving", true)
            .expect("install optional refire choice");
    }
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("accept refire");

    assert_eq!(r.memory(), 1);
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn multiple_refireable_effects_install_visible_choice() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("REFIRE-TARGET", "Refire Target"))
        .memory(0)
        .start();
    r.register_effect("REFIRE-TARGET", Arc::new(TwoWhenDigivolvingEffects));

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "REFIRE-TARGET", Some(0));
    let caller_source = r.top_card(source_stack);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        ctx.refire_effect_from_permanent(target, "when_digivolving", false)
            .expect("install refire choice");
    }

    let pending = r.game.pending_selection.as_ref().expect("refire choice");
    assert_eq!(pending.valid_action_ids.len(), 2);
    assert!(!pending.is_optional);
    r.game
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .expect("choose second refire effect");

    assert_eq!(r.memory(), 3);
}
