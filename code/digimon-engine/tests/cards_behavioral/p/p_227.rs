//! P-227 Unique Emblem: Primal Impact — Option, cost 2, red, [LIBERATOR].
//!
//! Printed text (official Bandai DB bundle + DCGO P_227.cs):
//! - [Main] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//!   [Tyrannomon] in its name or the [Reptile] or [Dinosaur] trait and 1
//!   [LIBERATOR] trait card among them to the hand. Return the rest to the
//!   bottom of the deck. Then, place this card in the battle area.
//! - [Your Turn] When any of your [Ryutaro Williams] are played, <Delay> ... 1
//!   of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait
//!   card in the hand with the digivolution cost reduced by 3.
//! - Security Effect [Security] Activate this card's [Main] effects.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn red_digimon(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    digimon(id, name, level, 5000, 5, vec![CardColor::Red], traits)
}

fn ryutaro(id: &str) -> CardData {
    let mut c = card(id, "Ryutaro Williams", CardKind::Tamer, vec![CardColor::Red]);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 0;
    c
}

fn builder(deck_top_first_ids: &[&str], hand: &[&str]) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("P-227")
        .expect("P-227 loads")
        .add_card(red_digimon("TYRA", "MasterTyrannomon", 5, &[]))
        .add_card(red_digimon("REPT", "Reptile Digimon", 4, &["Reptile"]))
        .add_card(red_digimon("DINO", "Dinosaur Digimon", 4, &["Dinosaur"]))
        .add_card(card("TYRA-OPT", "Tyrannomon Option", CardKind::Option, vec![CardColor::Red]))
        .add_card(red_digimon("LIB", "Liberator Digimon", 4, &["LIBERATOR"]))
        .add_card(with_evo(red_digimon("EVO", "Liberator Evo", 4, &["LIBERATOR"]), CardColor::Red, 3, 3))
        .add_card(ryutaro("RYU"))
        .add_card(digimon("BASE", "Base Digimon", 3, 3000, 3, vec![CardColor::Red], &[]))
        .add_card(digimon("FILL", "Filler", 3, 1000, 3, vec![CardColor::Blue], &[]))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["FILL"; 6])
        .hand(0, hand)
        .memory(10)
}

fn play_p227(r: &mut DebugRunner) {
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
}

#[test]
fn p_227_main_adds_a_tyrannomon_reptile_or_dinosaur_digimon_and_a_liberator_card() {
    let mut r = builder(&["TYRA", "LIB", "FILL", "FILL"], &["P-227"]).start();
    play_p227(&mut r);
    let view = r.pending_selection_view().expect("dinosaur-family bucket");
    assert_eq!(view.valid_action_ids.len(), 1, "[Tyrannomon]-name Digimon");
    assert!(!view.is_optional, "mandatory when a candidate exists");
    pick_reveal(&mut r, 0, "TYRA");
    pick_reveal(&mut r, 0, "LIB");
    r.auto_resolve().unwrap();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["LIB", "TYRA"]);
    assert!(field_ids(&r, 0).contains(&"P-227".to_string()), "placed in the battle area");
}

#[test]
fn p_227_reptile_and_dinosaur_traits_qualify_but_a_named_option_is_not_a_digimon_card() {
    let mut r = builder(&["REPT", "DINO", "TYRA-OPT", "FILL"], &["P-227"]).start();
    play_p227(&mut r);
    let view = r.pending_selection_view().expect("bucket");
    assert_eq!(view.valid_action_ids.len(), 2, "[Reptile] and [Dinosaur] Digimon; not the Option");
}

#[test]
fn p_227_one_card_cannot_fill_both_picks() {
    let both = red_digimon("BOTH", "Reptile Liberator", 4, &["Reptile", "LIBERATOR"]);
    let mut r = builder(&["BOTH", "FILL", "FILL", "FILL"], &["P-227"]).add_card(both).start();
    play_p227(&mut r);
    pick_reveal(&mut r, 0, "BOTH");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["BOTH"]);
}

#[test]
fn p_227_delay_is_parked_and_fires_on_a_ryutaro_williams_play_after_the_placing_turn() {
    let mut r = builder(
        &["FILL"; 8],
        &["P-227", "RYU", "EVO"],
    )
    .start();
    play_p227(&mut r);
    r.auto_resolve().unwrap();
    let state = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "P-227")
        .map(|p| p.option_state)
        .unwrap();
    assert!(matches!(
        state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
            ..
        }
    ));
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "RYU")
        .unwrap();
    assert!(r.game.play_from_hand(0, idx).is_some());
    let view = r.pending_selection_view().expect("Delay activation prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT).unwrap();
    let mut guard = 0;
    while let Some(view) = r.pending_selection_view() {
        guard += 1;
        assert!(guard < 6);
        r.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();
    }
    assert!(field_ids(&r, 0).contains(&"EVO".to_string()), "digivolved into the LIBERATOR card");
    assert!(trash_ids(&r, 0).contains(&"P-227".to_string()), "the Delay trashed the Emblem");
}
