//! CR 4.3 4-21-3: a digivolution requirement waiver cannot authorize stacking on a token.
//! Regresses the Marcus -> Petrification -> EX4-074 illegal stack observed in the frozen evaluation.
use digimon_engine::{
    card_data::CardData,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, CardSourceRef, CostDelta, PlaySource},
};

fn card(id: &str, kind: CardKind) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![CardColor::Yellow];
    c.level = (kind == CardKind::Digimon).then_some(7);
    c.dp = (kind == CardKind::Digimon).then_some(15000);
    c
}

fn check_target(target_id: &str, from_trash: bool, expect_success: bool) {
    for player in 0..2 {
        let mut r = DebugRunner::builder()
            .add_card(card("RESULT", CardKind::Digimon))
            .add_card(card("BASE", CardKind::Digimon))
            .add_card(card("TAMER", CardKind::Tamer))
            .hand(player, &["RESULT"])
            .deck(player, &["BASE"; 5])
            .memory(5)
            .start();
        let target = r.place_on_field(player, target_id, Some(0));
        let original_top = r.top_card(target);
        let result = r.game.players[player as usize].hand[0].handle();
        let source = if from_trash {
            let c = r.game.players[player as usize].hand.remove(0);
            r.game.players[player as usize].put_in_trash(c);
            CardSourceRef::Trash(player, 0)
        } else {
            CardSourceRef::Hand(player, 0)
        };
        let memory = r.memory();
        let deck = r.game.players[player as usize].deck.clone();
        r.game.drain_events();
        let success = r.game.effect_initiated_digivolve_from_source_ignore_requirements(
            player, source, target, CostDelta::Fixed(3), PlaySource::ByEffect,
        );
        assert_eq!(success, expect_success, "token stacking is a rule, not a waived digivolution requirement");
        if expect_success {
            assert_eq!(r.top_card(target), result);
            assert!(r.game.players[player as usize].battle_area[0].card_sources.iter().any(|c| c.handle() == original_top));
        } else {
            assert_eq!(r.top_card(target), original_top);
            assert_eq!(r.game.players[player as usize].battle_area[0].card_sources.len(), 1);
            assert_eq!(r.memory(), memory, "a rejected token host must not pay memory");
            assert_eq!(r.game.players[player as usize].deck.iter().map(|c| c.handle()).collect::<Vec<_>>(), deck.iter().map(|c| c.handle()).collect::<Vec<_>>(), "no digivolution draw");
            let cards = if from_trash { &r.game.players[player as usize].trash } else { &r.game.players[player as usize].hand };
            assert_eq!(cards.iter().map(|c| c.handle()).collect::<Vec<_>>(), vec![result], "preserve the exact selected physical result card");
            assert!(r.game.pending_selection.is_none());
            assert!(!r.game.drain_events().iter().any(|e| matches!(e, digimon_engine::events::GameEvent::EvalDigivolve { .. })));
        }
    }
}

#[test]
fn waived_hand_digivolution_cannot_stack_onto_a_real_token() {
    check_target("TOKEN_PETRIFICATION", false, false);
}

#[test]
fn waived_trash_digivolution_cannot_stack_onto_a_real_token() {
    check_target("TOKEN_PETRIFICATION", true, false);
}

#[test]
fn waived_digivolution_still_accepts_real_digimon() {
    check_target("BASE", false, true);
}

#[test]
fn waived_digivolution_still_accepts_real_tamers() {
    check_target("TAMER", true, true);
}
