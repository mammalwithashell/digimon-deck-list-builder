//! Source: `data/card_bundles/BT27-040.md` (printed text).
//!
//! ### Effect
//! ＜Overclock ([Chuumon]/[Sukamon] in name)＞ [Security] [On Play] [When Digivolving] To 1 of your opponent's Digimon, give -3000 DP for the turn for each of your Digimon with [Chuumon], [Sukamon] or [Etemon] in their names.
//!
//! ### Special Play Condition
//! Assembly -3: Lv.4 Digimon card w/[Sukamon] in name
//!
//! ### Inherited Effect
//! [All Turns] [Once Per Turn] When any of your other Digimon with [Chuumon], [Sukamon] or [Etemon] in their names are played, ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//!
//! ## Official Q&A
//! - Yes, its [Security] effect activates, then it battles.

use super::common::*;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-040";

#[test]
fn overclock_offers_tokens_and_chuumon_or_sukamon_name_costs() {
    let mut token = make_test_card("TEXT-TOKEN", "Token");
    token.card_kind = CardKind::Token;
    let mut r = start(
        builder(CARD)
            .add_card(token)
            .add_card(make_test_card("CHUU", "Chuumon"))
            .add_card(make_test_card("SUKA", "Sukamon")),
    );
    r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "TEXT-TOKEN", Some(0));
    r.place_on_field(0, "CHUU", Some(0));
    r.place_on_field(0, "SUKA", Some(0));
    r.place_on_field(0, "TEXT-5", Some(0));
    r.pass_turn();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["CHUU", "SUKA", "TEXT-TOKEN"]);
    r.decline_prompt().unwrap();
    assert_eq!(r.battle_area_size(0), 5);
}

#[test]
fn on_play_counts_each_matching_ally_and_ignores_the_opponents_family() {
    let mut r = start(
        builder(CARD)
            .add_card(make_test_card("SUKA", "Sukamon"))
            .hand(0, &[CARD]),
    );
    r.place_on_field(0, "SUKA", Some(0));
    r.place_on_field(1, "SUKA", Some(0));
    let victim = r.place_on_field(1, "TEXT-6", Some(0));
    r.play(0, 0);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["SUKA", "TEXT-6"]);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    assert_eq!(r.dp_of(victim), Some(6000));
}

#[test]
fn inherited_de_digivolve_requires_another_matching_digimon_and_fires_once() {
    let mut suka = make_test_card("SUKA", "Sukamon");
    suka.play_cost = 0;
    let mut r = start(
        builder(CARD)
            .add_card(suka)
            .hand(0, &["TEXT-3", "SUKA", "SUKA"]),
    );
    r.place_stack(0, &[CARD, "TEXT-6"]);
    let enemy = r.place_stack(1, &["TEXT-3", "TEXT-4", "TEXT-5"]);
    r.play(0, 0);
    assert!(r.pending_selection().is_none());
    r.play(0, 0);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(r.dp_of(enemy), Some(4000));
    r.play(0, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.dp_of(enemy), Some(4000));
}

#[test]
fn security_dp_reduction_resolves_before_battling_the_attacker() {
    let mut r = start(builder(CARD).security(1, &[CARD]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    r.place_on_field(1, CARD, Some(0));
    r.attack_player(attacker, 1, false);
    r.assert_prompt(SelectionKind::OppField, false, 1);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    assert_eq!(r.dp_of(attacker), Some(9000));
    assert_eq!(r.security_count(1), 0);
    assert_eq!(ids(&r, &r.game.players[1].trash), [CARD]);
}
