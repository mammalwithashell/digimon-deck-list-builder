//! P-230 Unique Emblem: Honeycomb Commander — Option, cost 2, green, [LIBERATOR].
//!
//! Printed text (official Bandai DB bundle + DCGO P_230.cs):
//! - [Main] Reveal the top 3 cards of your deck. Add 1 card with [Royal Base] in
//!   its text and 1 [LIBERATOR] trait card among them to the hand. Return the
//!   rest to the bottom of the deck. Then, place this card in the battle area.
//! - [Your Turn] When any of your [Winr]s are played, <Delay> ... 1 of your
//!   Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the
//!   hand with the digivolution cost reduced by 3.
//! - Security Effect [Security] Activate this card's [Main] effects.
//!
//! Official Q&A: "[X] in its text" covers the name, traits and every effect box.
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

fn royal_text_digimon(id: &str) -> CardData {
    // "[Royal Base] in its text": here via an effect box, not the name/trait.
    let mut c = digimon(id, "Text Carrier", 4, 4000, 4, vec![CardColor::Green], &[]);
    c.effect_text = "[On Play] Add 1 [Royal Base] trait card from your trash.".to_string();
    c
}

fn royal_named_option(id: &str) -> CardData {
    card(id, "Royal Base", CardKind::Option, vec![CardColor::Green])
}

fn liberator_digimon(id: &str, level: u8) -> CardData {
    digimon(id, "Liberator Digimon", level, 5000, 5, vec![CardColor::Green], &["LIBERATOR"])
}

fn winr(id: &str) -> CardData {
    let mut c = card(id, "Winr", CardKind::Tamer, vec![CardColor::Green]);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 0;
    c
}

fn builder_with_deck(deck_top_first_ids: &[&str], hand: &[&str]) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("P-230")
        .expect("P-230 loads")
        .add_card(royal_text_digimon("RB-TEXT"))
        .add_card(royal_named_option("RB-OPT"))
        .add_card(liberator_digimon("LIB", 4))
        .add_card(liberator_digimon("LIB7", 7))
        .add_card(winr("WINR"))
        .add_card(card("OTHER-TAMER", "Other Tamer", CardKind::Tamer, vec![CardColor::Green]))
        .add_card(with_evo(
            liberator_digimon("EVO", 4),
            CardColor::Green,
            3,
            3,
        ))
        .add_card(digimon("BASE", "Base Digimon", 3, 3000, 3, vec![CardColor::Green], &[]))
        .add_card(digimon("FILL", "Filler", 3, 1000, 3, vec![CardColor::Red], &[]))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["FILL"; 6])
        .hand(0, hand)
        .memory(10)
}

fn play_p230(r: &mut DebugRunner) {
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
}

#[test]
fn p_230_main_adds_a_royal_base_text_card_of_any_kind_and_a_liberator_card() {
    let mut r = builder_with_deck(&["FILL", "RB-OPT", "LIB", "FILL", "FILL"], &["P-230"]).start();
    play_p230(&mut r);
    // First bucket: the [Royal Base]-named Option qualifies ("1 card", any kind).
    let view = r.pending_selection_view().expect("Royal Base bucket");
    assert_eq!(view.valid_action_ids.len(), 1);
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory when a card matches");
    pick_reveal(&mut r, 0, "RB-OPT");
    let view = r.pending_selection_view().expect("LIBERATOR bucket");
    assert_eq!(view.valid_action_ids.len(), 1);
    pick_reveal(&mut r, 0, "LIB");
    r.auto_resolve().unwrap();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["LIB", "RB-OPT"]);
    // The third revealed card went to the bottom; the deck lost exactly 2.
    assert_eq!(r.deck_size(0), 3);
}

#[test]
fn p_230_in_its_text_matches_an_effect_box_mention_not_only_the_name() {
    let mut r = builder_with_deck(&["RB-TEXT", "FILL", "FILL", "FILL"], &["P-230"]).start();
    play_p230(&mut r);
    let view = r.pending_selection_view().expect("Royal Base bucket");
    assert_eq!(view.valid_action_ids.len(), 1, "the effect-text mention qualifies");
    pick_reveal(&mut r, 0, "RB-TEXT");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["RB-TEXT"]);
}

#[test]
fn p_230_one_card_cannot_fill_both_picks() {
    // A [LIBERATOR] card that also has [Royal Base] in its text counts once.
    let mut both = liberator_digimon("BOTH", 4);
    both.effect_text = "[Security] Royal Base synergy: [Royal Base]".to_string();
    let mut r = builder_with_deck(&["BOTH", "FILL", "FILL", "FILL"], &["P-230"])
        .add_card(both)
        .start();
    play_p230(&mut r);
    let view = r.pending_selection_view().expect("first bucket");
    pick_reveal(&mut r, 0, "BOTH");
    // BOTH is gone from the pool, so the LIBERATOR bucket has no candidate.
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["BOTH"], "added once, not twice");
}

#[test]
fn p_230_main_places_this_card_in_the_battle_area_as_a_winr_gated_delay_option() {
    let mut r = builder_with_deck(&["FILL", "FILL", "FILL", "FILL"], &["P-230"]).start();
    play_p230(&mut r);
    r.auto_resolve().unwrap();
    assert!(field_ids(&r, 0).contains(&"P-230".to_string()), "placed in the battle area");
    let state = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "P-230")
        .map(|p| p.option_state)
        .unwrap();
    assert!(
        matches!(
            state,
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            }
        ),
        "parked as an OnAllyPlayed <Delay>"
    );
}

fn place_then_wait_a_turn(r: &mut DebugRunner) {
    play_p230(r);
    r.auto_resolve().unwrap();
    // Past the placing turn and back to seat 0's turn.
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

#[test]
fn p_230_delay_fires_only_when_your_winr_is_played_after_the_placing_turn() {
    let mut r = builder_with_deck(
        &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL"],
        &["P-230", "WINR", "OTHER-TAMER", "EVO", "LIB7"],
    )
    .start();
    place_then_wait_a_turn(&mut r);

    // A Tamer that is not a [Winr] does not trigger it.
    let other = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "OTHER-TAMER")
        .unwrap();
    assert!(r.game.play_from_hand(0, other).is_some());
    assert!(r.pending_selection().is_none(), "not a [Winr]: the Delay stays parked");

    // Playing a [Winr] offers the Delay (optional per CR 4.3 16-17-2).
    let winr_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "WINR")
        .unwrap();
    assert!(r.game.play_from_hand(0, winr_idx).is_some());
    let view = r.pending_selection_view().expect("the Delay activation prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional());
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT).unwrap();

    // Body: choose the Digimon, then the LIBERATOR card (level <= 6 only).
    let view = r.pending_selection_view().expect("choose the Digimon to digivolve");
    assert!(r.pending_is_optional(), "'may digivolve'");
    r.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();
    let view = r.pending_selection_view().expect("choose the digivolution card");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the level 4 [LIBERATOR] card qualifies (LIB7 is level 7)"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();

    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "P-230"),
        "the Delay trashed this card as its cost"
    );
    assert!(field_ids(&r, 0).contains(&"EVO".to_string()), "digivolved into EVO");
    assert!(
        !hand_ids(&r, 0).contains(&"EVO".to_string()),
        "the digivolution card left the hand"
    );
}

#[test]
fn p_230_delay_does_not_fire_on_the_placing_turn() {
    let mut r = builder_with_deck(
        &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"],
        &["P-230", "WINR", "EVO"],
    )
    .start();
    play_p230(&mut r);
    r.auto_resolve().unwrap();
    let winr_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "WINR")
        .unwrap();
    assert!(r.game.play_from_hand(0, winr_idx).is_some());
    assert!(r.pending_selection().is_none(), "never on the placing turn");
}

#[test]
fn p_230_security_effect_activates_the_main_search_then_places_the_card() {
    use digimon_engine::card_source::CardSource;
    let mut r = builder_with_deck(&["FILL"; 6], &[])
        .add_card(digimon("ATK", "Attacker", 4, 5000, 5, vec![CardColor::Red], &[]))
        .deck(1, &top_first(&["FILL", "RB-OPT", "LIB", "FILL", "FILL"]))
        .start();
    let attacker = r.place_on_field(0, "ATK", Some(0));
    {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "P-230")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[1].security.push(CardSource::new(idx, 1, next));
    }
    r.attack_player(attacker, 1, false);
    // The security owner (seat 1) resolves the [Main] search bucket prompts.
    for _ in 0..8 {
        match r.pending_selection_view() {
            Some(view) => {
                let who = view.selecting_player;
                let action = view
                    .valid_action_ids
                    .iter()
                    .copied()
                    .find(|a| *a != PASS)
                    .unwrap_or(PASS);
                if r.execute_action(who, action).is_err() {
                    break;
                }
            }
            None => break,
        }
    }
    let hand1 = hand_ids(&r, 1);
    assert!(
        hand1.contains(&"RB-OPT".to_string()) && hand1.contains(&"LIB".to_string()),
        "[Security] activated the [Main] search for the security owner: {hand1:?}"
    );
    assert!(
        field_ids(&r, 1).contains(&"P-230".to_string()),
        "and placed the card in the battle area"
    );
}
