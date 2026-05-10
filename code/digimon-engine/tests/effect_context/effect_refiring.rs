use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::TimingFilter;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::{CardEffect, CardHandle, Effect, PermanentHandle};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct SourceRecordingWhenDigivolving {
    seen: Arc<Mutex<Vec<(CardHandle, Option<PermanentHandle>)>>>,
}

#[derive(Clone)]
struct SourceRecordingOnPlay {
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

impl CardEffect for SourceRecordingOnPlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_play(card)
            .name("record on play source and gain memory")
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
struct OnAnyPlayedObserver {
    seen: Arc<Mutex<u8>>,
}

#[derive(Clone)]
struct OnDigivolveObserver {
    seen: Arc<Mutex<u8>>,
}

impl CardEffect for OnAnyPlayedObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_any_digimon_played(card)
            .name("record any played")
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

impl CardEffect for OnDigivolveObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_digivolve(card)
            .name("record digivolve observer")
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
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
fn refire_selected_when_digivolving_effect_uses_grantor_source_and_target_carrier() {
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
        ctx.refire_effect_from_permanent(target, "when_digivolving", false)
            .expect("refire effect");
    }

    assert_eq!(r.memory(), 1);
    assert_eq!(*seen.lock().unwrap(), vec![(caller_source, Some(target))]);
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

#[derive(Clone)]
struct OncePerTurnWhenDigivolving;

impl CardEffect for OncePerTurnWhenDigivolving {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("once per turn gain one")
            .once_per_turn()
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn refire_on_play_effect_uses_grantor_source_and_does_not_fire_play_observers() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let observer_seen = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("REFIRE-TARGET", "Refire Target"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .memory(0)
        .start();
    r.register_effect(
        "REFIRE-TARGET",
        Arc::new(SourceRecordingOnPlay {
            seen: Arc::clone(&seen),
        }),
    );
    r.register_effect(
        "OBSERVER",
        Arc::new(OnAnyPlayedObserver {
            seen: Arc::clone(&observer_seen),
        }),
    );

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "REFIRE-TARGET", Some(0));
    let _observer = r.place_on_field(0, "OBSERVER", Some(0));
    let caller_source = r.top_card(source_stack);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        ctx.refire_effect_from_permanent(target, "on_play", false)
            .expect("refire on play effect");
    }

    assert_eq!(r.memory(), 1);
    assert_eq!(*seen.lock().unwrap(), vec![(caller_source, Some(target))]);
    assert_eq!(
        *observer_seen.lock().unwrap(),
        0,
        "refiring an [On Play] effect must not make the target enter play again"
    );
}

#[test]
fn refire_when_digivolving_effect_does_not_fire_digivolve_observers() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let observer_seen = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("REFIRE-TARGET", "Refire Target"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .memory(0)
        .start();
    r.register_effect(
        "REFIRE-TARGET",
        Arc::new(SourceRecordingWhenDigivolving {
            seen: Arc::clone(&seen),
        }),
    );
    r.register_effect(
        "OBSERVER",
        Arc::new(OnDigivolveObserver {
            seen: Arc::clone(&observer_seen),
        }),
    );

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "REFIRE-TARGET", Some(0));
    let _observer = r.place_on_field(0, "OBSERVER", Some(0));
    let caller_source = r.top_card(source_stack);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        assert!(ctx.refire_target_effect(target, TimingFilter::WhenDigivolving, 0, false));
    }

    assert_eq!(r.memory(), 1);
    assert_eq!(*seen.lock().unwrap(), vec![(caller_source, Some(target))]);
    assert_eq!(
        *observer_seen.lock().unwrap(),
        0,
        "refiring a [When Digivolving] effect must not make the target digivolve again"
    );
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

#[test]
fn refire_target_effect_returns_false_when_no_eligible_effects_exist() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("NO-EFFECT-TARGET", "No Effect Target"))
        .memory(0)
        .start();

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "NO-EFFECT-TARGET", Some(0));
    let caller_source = r.top_card(source_stack);

    let queued = {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        ctx.refire_target_effect(target, TimingFilter::Either, 0, false)
    };

    assert!(!queued);
    assert_eq!(r.memory(), 0);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn refire_target_effect_bypass_once_per_turn_invokes_consumed_slot() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-STACK", "Source Stack"))
        .add_card(make_test_card("OPT-TARGET", "OPT Target"))
        .memory(0)
        .start();
    r.register_effect("OPT-TARGET", Arc::new(OncePerTurnWhenDigivolving));

    let source_stack = r.place_on_field(0, "SOURCE-STACK", Some(0));
    let target = r.place_on_field(0, "OPT-TARGET", Some(0));
    let caller_source = r.top_card(source_stack);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        assert!(ctx.refire_target_effect(target, TimingFilter::WhenDigivolving, 0, false));
    }
    assert_eq!(r.memory(), 1);

    {
        let mut ctx = EffectContext::new(&mut r.game, caller_source, Some(source_stack), 0);
        assert!(ctx.refire_target_effect(target, TimingFilter::WhenDigivolving, 0, true));
    }

    assert_eq!(
        r.memory(),
        2,
        "bypass_once_per_turn should invoke even after the target effect slot was consumed"
    );
}
