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

/// make-engine-cloneable (Wave A): the refire-effect choice is resume-driven, so
/// cloning the game at the prompt is faithful — the clone runs the chosen effect
/// while the original is untouched and replays identically.
#[test]
fn refire_effect_choice_clones_faithfully() {
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

    assert!(
        r.game.pending_selection_resume.is_some(),
        "the refire-effect choice must be resume-driven (clone-safe)"
    );
    let mem_before = r.game.memory;

    // Clone at the prompt; run the second effect on the clone only.
    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .expect("clone picks the second effect");
    assert!(clone.pending_selection.is_none(), "clone: refire resolved");
    assert_ne!(
        clone.memory, mem_before,
        "clone: the chosen effect ran (memory changed)"
    );

    // INDEPENDENCE: the original is untouched.
    assert!(
        r.game.pending_selection.is_some(),
        "original's refire choice survives the clone"
    );
    assert_eq!(
        r.game.memory, mem_before,
        "original: no effect run while the clone resolved"
    );

    // REPLAYS IDENTICALLY.
    r.game
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .expect("original picks the second effect");
    assert_eq!(
        r.game.memory, clone.memory,
        "original reaches the clone's state"
    );
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

// ─── Foreign-card refire (BT15-102 Apocalymon — source-card variant) ────────
//
// `activate_foreign_card_effect(card_id, carrier, timing, chooser)` fires an
// effect PRINTED ON a card object that is a digivolution source of `carrier`
// (not a battle-area top card), with the CARRIER as "this Digimon" — DCGO
// `selectedCard.EffectList_ForCard(EffectTiming.OnEnterFieldAnyone, card)`
// followed by `Activate_Optional_Effect_Execute` (BT15_102.cs End of Turn).

#[derive(Clone)]
struct TwoOnPlayEffects;

impl CardEffect for TwoOnPlayEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card)
                .name("gain one")
                .process(|ctx| ctx.gain_memory(1))
                .build(),
            Effect::on_play(card)
                .name("gain three")
                .process(|ctx| ctx.gain_memory(3))
                .build(),
        ]
    }
}

#[test]
fn activate_foreign_card_effect_runs_placed_on_play_with_carrier_as_this_digimon() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("APOC-CARRIER", "Apocalymon Carrier"))
        .add_card(make_test_card("FOREIGN-ONPLAY", "Foreign On Play"))
        .memory(0)
        .start();
    r.register_effect(
        "FOREIGN-ONPLAY",
        Arc::new(SourceRecordingOnPlay {
            seen: Arc::clone(&seen),
        }),
    );

    // FOREIGN-ONPLAY sits UNDER the carrier top — a digivolution source, the
    // position a just-placed bottom card occupies.
    let carrier = r.place_stack(0, &["FOREIGN-ONPLAY", "APOC-CARRIER"]);
    let carrier_top = r.top_card(carrier);

    let fired = {
        let mut ctx = EffectContext::new(&mut r.game, carrier_top, Some(carrier), 0);
        ctx.activate_foreign_card_effect("FOREIGN-ONPLAY", carrier, TimingFilter::OnPlay, 0)
    };

    assert!(fired, "one eligible [On Play] effect runs directly");
    assert_eq!(r.memory(), 1, "the foreign body executed");
    assert_eq!(
        *seen.lock().unwrap(),
        vec![(carrier_top, Some(carrier))],
        "the refired body must see the CARRIER as source card + permanent \
         ('as an effect of this Digimon')"
    );
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn activate_foreign_card_effect_with_two_effects_surfaces_mandatory_choice() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("APOC-CARRIER", "Apocalymon Carrier"))
        .add_card(make_test_card("FOREIGN-TWO", "Foreign Two Effects"))
        .memory(0)
        .start();
    r.register_effect("FOREIGN-TWO", Arc::new(TwoOnPlayEffects));

    let carrier = r.place_stack(0, &["FOREIGN-TWO", "APOC-CARRIER"]);
    let carrier_top = r.top_card(carrier);

    let fired = {
        let mut ctx = EffectContext::new(&mut r.game, carrier_top, Some(carrier), 0);
        ctx.activate_foreign_card_effect("FOREIGN-TWO", carrier, TimingFilter::OnPlay, 0)
    };
    assert!(fired);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("two eligible effects surface an EffectChoice");
    assert!(
        !sel.is_optional,
        "the pick is mandatory once the placement committed (DCGO canNoSelect: false)"
    );
    let choices = sel.effect_choices.as_ref().expect("choice entries");
    assert_eq!(choices.len(), 2);
    assert!(
        choices[0].label.contains("FOREIGN-TWO"),
        "choices are labeled with the FOREIGN card id: {}",
        choices[0].label
    );

    r.game
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .expect("pick the second effect");
    assert_eq!(r.memory(), 3, "the chosen (second) foreign body executed");
}

#[test]
fn activate_foreign_card_effect_returns_false_with_no_eligible_effect() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("APOC-CARRIER", "Apocalymon Carrier"))
        .add_card(make_test_card("FOREIGN-VANILLA", "Foreign Vanilla"))
        .memory(0)
        .start();

    let carrier = r.place_stack(0, &["FOREIGN-VANILLA", "APOC-CARRIER"]);
    let carrier_top = r.top_card(carrier);

    let fired = {
        let mut ctx = EffectContext::new(&mut r.game, carrier_top, Some(carrier), 0);
        ctx.activate_foreign_card_effect("FOREIGN-VANILLA", carrier, TimingFilter::OnPlay, 0)
    };
    assert!(!fired, "a card with no [On Play] text fires nothing");
    assert_eq!(r.memory(), 0);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn activate_foreign_card_effect_filters_by_timing() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("APOC-CARRIER", "Apocalymon Carrier"))
        .add_card(make_test_card("FOREIGN-WD", "Foreign When Digivolving"))
        .memory(0)
        .start();
    // The foreign card's only effect is [When Digivolving] — an OnPlay-scoped
    // refire must not fire it.
    r.register_effect(
        "FOREIGN-WD",
        Arc::new(SourceRecordingWhenDigivolving {
            seen: Arc::clone(&seen),
        }),
    );

    let carrier = r.place_stack(0, &["FOREIGN-WD", "APOC-CARRIER"]);
    let carrier_top = r.top_card(carrier);

    let fired = {
        let mut ctx = EffectContext::new(&mut r.game, carrier_top, Some(carrier), 0);
        ctx.activate_foreign_card_effect("FOREIGN-WD", carrier, TimingFilter::OnPlay, 0)
    };
    assert!(!fired, "[When Digivolving] text is not an [On Play] effect");
    assert!(seen.lock().unwrap().is_empty());
    assert_eq!(r.memory(), 0);
}

#[test]
fn clone_mid_foreign_refire_choice_resolves_on_the_clone() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("APOC-CARRIER", "Apocalymon Carrier"))
        .add_card(make_test_card("FOREIGN-TWO", "Foreign Two Effects"))
        .memory(0)
        .start();
    r.register_effect("FOREIGN-TWO", Arc::new(TwoOnPlayEffects));

    let carrier = r.place_stack(0, &["FOREIGN-TWO", "APOC-CARRIER"]);
    let carrier_top = r.top_card(carrier);

    {
        let mut ctx = EffectContext::new(&mut r.game, carrier_top, Some(carrier), 0);
        assert!(ctx.activate_foreign_card_effect("FOREIGN-TWO", carrier, TimingFilter::OnPlay, 0));
    }
    assert!(r.game.pending_selection.is_some());

    // Clone while the EffectChoice is parked (data-driven RefireEffectChoice
    // frame — make-engine-cloneable): the clone must resolve independently.
    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("clone picks the first effect");
    assert_eq!(clone.memory, 1, "clone ran 'gain one'");

    r.game
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .expect("original picks the second effect");
    assert_eq!(r.memory(), 3, "original ran 'gain three' independently");
}
