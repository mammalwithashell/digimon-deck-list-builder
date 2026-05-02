use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};

struct OnOptionPlacedWitness(Arc<Mutex<u32>>);

impl CardEffect for OnOptionPlacedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Royal Knight option placed witness")
            .timing(EffectTiming::OnOptionPlaced)
            .condition(|ctx| {
                ctx.event_card()
                    .and_then(|card| ctx.game.card_data_for_handle(card))
                    .is_some_and(|data| data.traits.iter().any(|t| t == "Royal Knight"))
            })
            .process(move |ctx| {
                ctx.gain_memory(1);
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

struct DelayNoop;

impl CardEffect for DelayNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delay noop")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(|_ctx| {})
            .build()]
    }
}

struct TrainingNoop;

impl CardEffect for TrainingNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Training noop")
            .training()
            .process(|_ctx| {})
            .build()]
    }
}

struct LinkAnyHost;

impl CardEffect for LinkAnyHost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Link any host")
            .link(0, |_ctx, _host| true)
            .process(|_ctx| {})
            .build()]
    }
}

struct OnOptionPlacedChoice;

impl CardEffect for OnOptionPlacedChoice {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("OnOptionPlaced choice")
            .timing(EffectTiming::OnOptionPlaced)
            .process(|ctx| {
                ctx.select_effect_choice("Choose branch", vec!["ok".to_string()], |_ctx, _| {});
            })
            .build()]
    }
}

struct LinkAnyHostWithOnLinkWitness(Arc<Mutex<u32>>);

impl CardEffect for LinkAnyHostWithOnLinkWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Link any host")
                .link(0, |_ctx, _host| true)
                .process(|_ctx| {})
                .build(),
            Effect::on_play(card)
                .name("Linked OnLink witness")
                .timing(EffectTiming::OnLink)
                .linked()
                .process(move |_ctx| {
                    *seen.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

struct InheritedOnOptionPlacedWitness {
    seen: Arc<Mutex<u32>>,
    once: bool,
}

impl CardEffect for InheritedOnOptionPlacedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        let effect = Effect::inherited(card)
            .name("Inherited OnOptionPlaced witness")
            .timing(EffectTiming::OnOptionPlaced)
            .condition(|ctx| ctx.event_card().is_some())
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
            });
        let effect = if self.once {
            effect.once_per_turn()
        } else {
            effect
        };
        vec![effect.build()]
    }
}

fn option_card(card_id: &str, royal_knight: bool) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = 0;
    cd.colors = vec![CardColor::White];
    if royal_knight {
        cd.traits = vec!["Royal Knight".to_string()];
    }
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::White];
    cd
}

fn add_breeding_source(r: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("add_breeding_source: unknown card_id {card_id}"));
    let next_idx = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, next_idx);
    let handle = card.handle();
    let permanent = r.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("breeding permanent exists");
    let top = permanent.card_sources.pop().expect("breeding top exists");
    permanent.card_sources.push(card);
    permanent.card_sources.push(top);
    handle
}

#[test]
fn on_option_placed_fires_for_training_link_and_security_placement_with_event_card() {
    let witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("RK-DELAY", true))
        .add_card(option_card("RK-TRAIN", true))
        .add_card(option_card("RK-LINK", true))
        .add_card(option_card("NON-RK-LINK", false))
        .add_card(option_card("RK-SECURITY", true))
        .add_card(digimon_card("KING-DRASIL"))
        .add_card(digimon_card("HOST"))
        .hand(0, &["RK-DELAY", "RK-TRAIN", "RK-LINK", "NON-RK-LINK"])
        .memory(0)
        .start();
    r.register_effect("RK-DELAY", Arc::new(DelayNoop));
    r.register_effect("RK-TRAIN", Arc::new(TrainingNoop));
    r.register_effect("RK-LINK", Arc::new(LinkAnyHost));
    r.register_effect("NON-RK-LINK", Arc::new(LinkAnyHost));
    r.register_effect(
        "KING-DRASIL",
        Arc::new(OnOptionPlacedWitness(witness.clone())),
    );
    r.place_in_breeding(0, "KING-DRASIL");
    let host = r.place_on_field(0, "HOST", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "Royal Knight Delay placement fires"
    );

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        *witness.lock().unwrap(),
        2,
        "Royal Knight Training placement fires"
    );

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Pending
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action).unwrap();
    assert_eq!(
        *witness.lock().unwrap(),
        3,
        "Royal Knight Link placement fires"
    );

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Pending
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action).unwrap();
    assert_eq!(
        *witness.lock().unwrap(),
        3,
        "non-Royal-Knight Link placement does not match trait"
    );

    r.push_source(host, "RK-SECURITY");
    r.run_inherited_security_effect(host, "RK-SECURITY", |ctx| {
        ctx.place_self_as_delay_option_permanent();
    });
    assert_eq!(
        *witness.lock().unwrap(),
        4,
        "Royal Knight inherited-security placement fires"
    );
}

#[test]
fn link_on_option_placed_selection_resumes_on_link_after_choice_resolves() {
    let linked_on_link = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-WITH-ONLINK", true))
        .add_card(digimon_card("HOST"))
        .add_card(digimon_card("PLACED-CHOICE"))
        .hand(0, &["LINK-WITH-ONLINK"])
        .memory(0)
        .start();
    r.register_effect(
        "LINK-WITH-ONLINK",
        Arc::new(LinkAnyHostWithOnLinkWitness(linked_on_link.clone())),
    );
    r.register_effect("PLACED-CHOICE", Arc::new(OnOptionPlacedChoice));
    r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "PLACED-CHOICE", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Pending
    );
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, host_action).unwrap();
    assert!(
        r.game.pending_selection.is_some(),
        "OnOptionPlaced observer should park an effect-choice selection"
    );
    assert_eq!(
        *linked_on_link.lock().unwrap(),
        0,
        "OnLink must wait until the placed-option selection settles"
    );

    let choice_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, choice_action).unwrap();
    assert_eq!(
        *linked_on_link.lock().unwrap(),
        1,
        "OnLink resumes after the placed-option selection resolves"
    );
}

#[test]
fn on_option_placed_scans_inherited_sources_under_breeding_top_card() {
    let seen = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("RK-DELAY", true))
        .add_card(digimon_card("BREEDING-TOP"))
        .add_card(digimon_card("BREEDING-SOURCE"))
        .hand(0, &["RK-DELAY"])
        .memory(0)
        .start();
    r.register_effect("RK-DELAY", Arc::new(DelayNoop));
    r.register_effect(
        "BREEDING-SOURCE",
        Arc::new(InheritedOnOptionPlacedWitness {
            seen: seen.clone(),
            once: false,
        }),
    );
    r.place_in_breeding(0, "BREEDING-TOP");
    add_breeding_source(&mut r, 0, "BREEDING-SOURCE");
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        *seen.lock().unwrap(),
        1,
        "breeding inherited OnOptionPlaced observer fires"
    );
}

#[test]
fn once_per_turn_breeding_on_option_placed_observer_fires_once_not_zero() {
    let seen = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("RK-DELAY-A", true))
        .add_card(option_card("RK-DELAY-B", true))
        .add_card(digimon_card("BREEDING-TOP"))
        .add_card(digimon_card("BREEDING-SOURCE"))
        .hand(0, &["RK-DELAY-A", "RK-DELAY-B"])
        .memory(0)
        .start();
    r.register_effect("RK-DELAY-A", Arc::new(DelayNoop));
    r.register_effect("RK-DELAY-B", Arc::new(DelayNoop));
    r.register_effect(
        "BREEDING-SOURCE",
        Arc::new(InheritedOnOptionPlacedWitness {
            seen: seen.clone(),
            once: true,
        }),
    );
    r.place_in_breeding(0, "BREEDING-TOP");
    add_breeding_source(&mut r, 0, "BREEDING-SOURCE");
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        *seen.lock().unwrap(),
        1,
        "breeding observer max_per_turn should allow first fire and block the second"
    );
}
