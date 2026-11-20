//! BT11-046 Agumon — Lv.3 green Rookie, DP 1000, cost 3 (digivolve: green Lv.2 / 0).
//!
//! Printed text (official Bandai DB bundle + DCGO BT11_046.cs):
//! - [On Play] Reveal the top 4 cards of your deck. Add 1 Tamer card among them
//!   to your hand. Place the rest at the bottom of your deck in any order.
//! - Inherited: [Your Turn] While you have a Tamer in play, this Digimon gets
//!   +2000 DP.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn runner(deck_top_first_ids: &[&str], hand: &[&str]) -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("BT11-046")
        .expect("BT11-046 loads")
        .add_card(card("TAMER-A", "Tamer A", CardKind::Tamer, vec![CardColor::Green]))
        .add_card(card("TAMER-B", "Tamer B", CardKind::Tamer, vec![CardColor::Red]))
        .add_card(card("OPT", "Option", CardKind::Option, vec![CardColor::Green]))
        .add_card(digimon(
            "MON",
            "Plain Digimon",
            3,
            3000,
            3,
            vec![CardColor::Green],
            &[],
        ))
        .add_card(digimon(
            "HOST",
            "Host Rookie",
            2,
            2000,
            2,
            vec![CardColor::Green],
            &[],
        ))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["MON"; 5])
        .hand(0, hand)
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

#[test]
fn bt11_046_on_play_adds_one_tamer_among_the_top_four_and_bottoms_the_rest() {
    let mut r = runner(&["MON", "TAMER-A", "OPT", "MON", "OPT"], &["BT11-046"]);
    r.play(0, 0).expect("Agumon plays");
    let view = r.pending_selection_view().expect("the Tamer pick is a prompt");
    // Only the Tamer is a legal choice; the add is mandatory (no PASS).
    assert_eq!(view.valid_action_ids.len(), 1, "only the revealed Tamer qualifies");
    assert!(!view.is_optional, "the Tamer add is mandatory (no 'may' in the printed text)");
    pick_reveal(&mut r, 0, "TAMER-A");
    // The rest (3 cards) go to the bottom in an order the player chooses.
    let order = r.pending_selection_view().expect("bottom order is the player's choice");
    assert_eq!(order.valid_action_ids.len(), 3);
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["TAMER-A"]);
    // 5th card (not revealed) is the new top; the three revealed non-Tamers
    // sit at the bottom.
    let deck = deck_top_first(&r, 0);
    assert_eq!(deck.len(), 4);
    assert_eq!(deck[0], "OPT", "the unrevealed 5th card is now the top card");
}

#[test]
fn bt11_046_with_two_tamers_only_one_may_be_added() {
    let mut r = runner(&["TAMER-A", "TAMER-B", "MON", "MON"], &["BT11-046"]);
    r.play(0, 0).unwrap();
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 2, "either Tamer may be chosen");
    pick_reveal(&mut r, 0, "TAMER-B");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["TAMER-B"], "exactly one Tamer is added");
    assert_eq!(r.deck_size(0), 3, "the other 3 revealed cards were returned");
}

#[test]
fn bt11_046_with_no_tamer_revealed_adds_nothing_and_still_orders_the_rest() {
    let mut r = runner(&["MON", "OPT", "MON", "OPT", "TAMER-A"], &["BT11-046"]);
    r.play(0, 0).unwrap();
    let view = r.pending_selection_view().expect("all four must still be ordered");
    assert_eq!(view.valid_action_ids.len(), 4);
    r.auto_resolve().unwrap();
    assert!(hand_ids(&r, 0).is_empty(), "no Tamer among the top 4: nothing added");
    assert_eq!(deck_top_first(&r, 0)[0], "TAMER-A", "the 5th card was never revealed");
}

#[test]
fn bt11_046_inherited_gains_2000_dp_only_on_your_turn_with_a_tamer_in_play() {
    let mut r = runner(&["MON"; 4], &[]);
    let host = r.place_stack(0, &["BT11-046", "HOST"]);
    let base = r.effective_dp(host).unwrap();
    assert_eq!(base, 2000, "no Tamer: no bonus");

    let tamer = r.place_on_field(0, "TAMER-A", None);
    assert_eq!(
        r.effective_dp(host).unwrap(),
        base + 2000,
        "[Your Turn] with a Tamer in play: +2000 DP"
    );

    // Opponent's turn: the [Your Turn] gate closes.
    r.game.turn_player_idx = 1;
    r.game.memory_pair = (1, 0);
    assert_eq!(r.effective_dp(host).unwrap(), base, "not your turn: no bonus");
}

#[test]
fn bt11_046_an_opponents_tamer_does_not_enable_the_inherited_bonus() {
    let mut r = runner(&["MON"; 4], &[]);
    let host = r.place_stack(0, &["BT11-046", "HOST"]);
    r.place_on_field(1, "TAMER-A", None);
    assert_eq!(r.effective_dp(host).unwrap(), 2000, "only YOUR Tamer counts");
}

#[test]
fn bt11_046_top_card_does_not_grant_its_own_inherited_bonus() {
    let mut r = runner(&["MON"; 4], &[]);
    let agumon = r.place_on_field(0, "BT11-046", None);
    r.place_on_field(0, "TAMER-A", None);
    assert_eq!(
        r.effective_dp(agumon).unwrap(),
        1000,
        "the inherited effect only works from under another Digimon"
    );
}
