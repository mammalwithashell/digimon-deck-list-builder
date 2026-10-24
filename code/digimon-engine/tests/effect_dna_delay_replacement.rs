//! BT17-095's existing Delay consumer must retain its leave cancellation while
//! the shared hand-partner DNA declaration parks on material order or Would.
use digimon_engine::{
    action::{build_action_mask, space::{PASS, REPLACEMENT_ACCEPT}},
    card_source::CardHandle,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    effect::{CardEffect, Effect},
    enums::{DelayTrigger, GamePhase},
    game::Game,
    permanent::OptionState,
    replacement::ReplacementCause,
};

struct OptionalDnaWindow;
impl CardEffect for OptionalDnaWindow {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_permanent_would_digivolve(card).optional()
            .name("Optional inner DNA cancellation")
            .condition(|ctx| ctx.dna_origin())
            .replacement_process(|ctx| ctx.cancel()).build()]
    }
}

fn choose(game: &mut Game, action: u16) {
    let pending = game.pending_selection.as_ref().expect("visible Delay/DNA choice");
    let player = pending.selecting_player;
    assert_eq!(build_action_mask(game, player)[action as usize], 1.0, "{}", pending.prompt);
    game.resolve_selection(player, action).unwrap();
}

fn choose_only(game: &mut Game) {
    let actions: Vec<_> = game.pending_selection.as_ref().unwrap().valid_action_ids.iter()
        .copied().filter(|action| *action != PASS).collect();
    assert_eq!(actions.len(), 1);
    choose(game, actions[0]);
}

fn delay_consumer(would_cancel: Option<bool>) {
    delay_consumer_departure(would_cancel, false);
}

fn delay_consumer_departure(would_cancel: Option<bool>, bounce: bool) {
    let mut field = make_test_card("DELAY-DNA-FIELD", "WarGreymon");
    field.level = Some(6);
    let mut partner = make_test_card("DELAY-DNA-PARTNER", "MetalGarurumon");
    partner.level = Some(6);
    let mut result = make_test_dna_card("DELAY-DNA-RESULT", "Omnimon", 6, 6, 0);
    result.level = Some(7);
    let mut r = DebugRunner::builder().dsl_card("BT17-095").unwrap()
        .add_card(field).add_card(partner).add_card(result)
        .add_card(make_test_card("DELAY-DNA-DRAW", "Draw"))
        .hand(0, &["DELAY-DNA-RESULT", "DELAY-DNA-PARTNER"])
        .deck(0, &["DELAY-DNA-DRAW"; 12]).deck(1, &["DELAY-DNA-DRAW"; 12])
        .memory(5).start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 3;
    let subject = r.place_on_field(0, "DELAY-DNA-FIELD", None);
    let original = r.game.players[0].battle_area[subject.index as usize].top_card().handle();
    let option = r.place_on_field(0, "BT17-095", None);
    r.game.players[0].battle_area[option.index as usize].option_state = OptionState::Delayed {
        owner: 0, trash_on_turn: 5, trigger: DelayTrigger::EndOfYourNextTurn, placed_on_turn: 1,
    };
    if would_cancel.is_some() { r.register_effect("DELAY-DNA-FIELD", std::sync::Arc::new(OptionalDnaWindow)); }
    r.game.tick_declarative_effects();
    if bounce { r.game.return_to_hand_from_effect(subject, 0); }
    else { r.game.delete_permanent_with_cause(subject, ReplacementCause::OwnEffect); }
    choose(&mut r.game, REPLACEMENT_ACCEPT);
    choose_only(&mut r.game); // Result card.
    choose_only(&mut r.game); // Hand material.
    let routes = r.game.pending_selection.as_ref().expect("both source orders remain visible");
    assert_eq!(routes.valid_action_ids.len(), 2);
    assert!(!routes.is_optional);
    let action = routes.valid_action_ids[0];
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        choose(game, action);
        if let Some(cancel) = would_cancel {
            choose(game, if cancel { REPLACEMENT_ACCEPT } else { PASS });
        }
        assert!(game.pending_selection.is_none());
        assert_eq!(game.memory, 5);
        assert!(game.players[0].trash.iter().any(|card| card.card_id(&game.card_data) == "BT17-095"));
        if would_cancel == Some(true) {
            assert!(game.players[0].trash.iter().any(|card| card.handle() == original));
            assert_eq!(game.players[0].hand.len(), 2);
            assert_eq!(game.players[0].deck.len(), 12);
        } else {
            let merged = game.players[0].battle_area.iter().find(|p|
                p.top_card().card_id(&game.card_data) == "DELAY-DNA-RESULT")
                .expect("successful asynchronous DNA must cancel only its enclosing leave");
            assert!(merged.card_sources.iter().any(|card| card.handle() == original));
            assert!(!game.players[0].trash.iter().any(|card| card.handle() == original));
            assert_eq!(game.players[0].hand.len(), 1);
            assert_eq!(game.players[0].deck.len(), 11);
        }
    }
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
}

#[test]
fn delay_leave_cancellation_survives_dna_material_order_choice() { delay_consumer(None); }
#[test]
fn delay_leave_cancellation_survives_nested_would_digivolve_choice() { delay_consumer(Some(false)); }
#[test]
fn cancelled_dna_does_not_cancel_the_original_delay_leave() { delay_consumer(Some(true)); }
#[test]
fn delay_cancels_a_bounce_after_dna_route_and_would_choices() {
    for would in [None, Some(false)] { delay_consumer_departure(would, true); }
}
