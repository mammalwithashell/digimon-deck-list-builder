//! P-232 Unique Emblem: Melting Recital — Option, cost 2, purple, [LIBERATOR].
//!
//! Printed text (official Bandai DB bundle + DCGO P_232.cs):
//! - [Main] Reveal the top 3 cards of your deck. Add 1 [Evil], [Dark Dragon] or
//!   [Evil Dragon] trait Digimon card and 1 [LIBERATOR] trait card among them to
//!   the hand. Return the rest to the bottom of the deck. Then, place this card
//!   in the battle area.
//! - [Your Turn] When any of your [Yuuki]s are played, <Delay> ... 1 of your
//!   Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the
//!   hand OR TRASH with the digivolution cost reduced by 3.
//! - Security Effect [Security] Activate this card's [Main] effects.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn purple_digimon(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    digimon(id, name, level, 5000, 5, vec![CardColor::Purple], traits)
}

fn yuuki(id: &str) -> CardData {
    let mut c = card(id, "Yuuki", CardKind::Tamer, vec![CardColor::Purple]);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 0;
    c
}

fn builder(deck_top_first_ids: &[&str], hand: &[&str]) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("P-232")
        .expect("P-232 loads")
        .add_card(purple_digimon("EVIL", "Evil Digimon", 4, &["Evil"]))
        .add_card(purple_digimon("DDRAGON", "Dark Dragon Digimon", 4, &["Dark Dragon"]))
        .add_card(purple_digimon("EDRAGON", "Evil Dragon Digimon", 4, &["Evil Dragon"]))
        .add_card(card("EVIL-OPT", "Evil Option", CardKind::Option, vec![CardColor::Purple]))
        .add_card(purple_digimon("LIB", "Liberator Digimon", 4, &["LIBERATOR"]))
        .add_card(purple_digimon("LIB7", "High Liberator", 7, &["LIBERATOR"]))
        .add_card(with_evo(purple_digimon("EVO", "Liberator Evo", 4, &["LIBERATOR"]), CardColor::Purple, 3, 3))
        .add_card(with_evo(purple_digimon("EVO-T", "Liberator Evo T", 4, &["LIBERATOR"]), CardColor::Purple, 3, 3))
        .add_card(yuuki("YUUKI"))
        .add_card(digimon("BASE", "Base Digimon", 3, 3000, 3, vec![CardColor::Purple], &[]))
        .add_card(digimon("FILL", "Filler", 3, 1000, 3, vec![CardColor::Red], &[]))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["FILL"; 6])
        .hand(0, hand)
        .memory(10)
}

fn play_p232(r: &mut DebugRunner) {
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
}

#[test]
fn p_232_main_adds_an_evil_family_digimon_and_a_liberator_card() {
    let mut r = builder(&["EDRAGON", "LIB", "FILL", "FILL"], &["P-232"]).start();
    play_p232(&mut r);
    let view = r.pending_selection_view().expect("Evil-family bucket");
    assert_eq!(view.valid_action_ids.len(), 1);
    assert!(!view.valid_action_ids.contains(&PASS));
    pick_reveal(&mut r, 0, "EDRAGON");
    pick_reveal(&mut r, 0, "LIB");
    r.auto_resolve().unwrap();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["EDRAGON", "LIB"]);
    assert!(field_ids(&r, 0).contains(&"P-232".to_string()), "placed in the battle area");
}

#[test]
fn p_232_each_of_the_three_evil_traits_qualifies_and_an_option_does_not() {
    let mut r = builder(&["EVIL", "DDRAGON", "EVIL-OPT", "FILL"], &["P-232"]).start();
    play_p232(&mut r);
    let view = r.pending_selection_view().expect("Evil-family bucket");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "[Evil] and [Dark Dragon] Digimon qualify; the Option is not a Digimon card"
    );
}

#[test]
fn p_232_one_card_cannot_fill_both_picks() {
    let mut both = purple_digimon("BOTH", "Evil Liberator", 4, &["Evil", "LIBERATOR"]);
    let mut r = builder(&["BOTH", "FILL", "FILL", "FILL"], &["P-232"]).add_card(both).start();
    play_p232(&mut r);
    pick_reveal(&mut r, 0, "BOTH");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["BOTH"]);
}

fn place_then_wait_a_turn(r: &mut DebugRunner) {
    play_p232(r);
    r.auto_resolve().unwrap();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

fn play_yuuki_and_accept_delay(r: &mut DebugRunner) {
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "YUUKI")
        .unwrap();
    assert!(r.game.play_from_hand(0, idx).is_some());
    let view = r.pending_selection_view().expect("the Delay activation prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional());
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT).unwrap();
    // Choose the Digimon base.
    let view = r.pending_selection_view().expect("choose the Digimon");
    r.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();
}

#[test]
fn p_232_delay_digivolves_from_the_hand_with_cost_minus_3() {
    let mut r = builder(
        &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL"],
        &["P-232", "YUUKI", "EVO", "LIB7"],
    )
    .start();
    place_then_wait_a_turn(&mut r);
    let memory = r.memory();
    play_yuuki_and_accept_delay(&mut r);
    let view = r.pending_selection_view().expect("choose the digivolution card");
    assert_eq!(view.valid_action_ids.len(), 1, "level 4 [LIBERATOR] only (LIB7 is level 7)");
    r.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert!(field_ids(&r, 0).contains(&"EVO".to_string()));
    // Printed digivolve cost 3, reduced by 3 -> free.
    assert_eq!(r.memory(), memory, "the digivolution cost was reduced by 3 (to 0)");
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "P-232"));
}

#[test]
fn p_232_delay_can_digivolve_from_the_trash_too() {
    let mut r = builder(
        &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL"],
        &["P-232", "YUUKI"],
    )
    .start();
    r.inject_trash(0, "EVO-T");
    place_then_wait_a_turn(&mut r);
    play_yuuki_and_accept_delay(&mut r);
    let view = r
        .pending_selection_view()
        .expect("the trash is a legal source: the digivolution card is chosen from hand/trash");
    r.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert!(
        field_ids(&r, 0).contains(&"EVO-T".to_string()),
        "digivolved into the card from the TRASH"
    );
    assert!(
        !trash_ids(&r, 0).contains(&"EVO-T".to_string()),
        "the digivolution card left the trash"
    );
}

#[test]
fn p_232_delay_ignores_a_non_yuuki_tamer() {
    let mut other = card("OT", "Other Tamer", CardKind::Tamer, vec![CardColor::Purple]);
    other.play_cost = 0;
    let mut r = builder(
        &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL"],
        &["P-232", "OT", "EVO"],
    )
    .add_card(other)
    .start();
    place_then_wait_a_turn(&mut r);
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "OT")
        .unwrap();
    assert!(r.game.play_from_hand(0, idx).is_some());
    assert!(r.pending_selection().is_none(), "only a [Yuuki] triggers the Delay");
}

#[test]
fn p_232_delay_is_parked_as_an_event_gated_option_after_main() {
    let mut r = builder(&["FILL"; 4], &["P-232"]).start();
    play_p232(&mut r);
    r.auto_resolve().unwrap();
    let state = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "P-232")
        .map(|p| p.option_state)
        .unwrap();
    assert!(matches!(
        state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
            ..
        }
    ));
}
