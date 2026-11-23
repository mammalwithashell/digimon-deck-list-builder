//! Source: `data/card_bundles/BT27-038.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Yellow Lv.3 / cost 3
//! - Purple Lv.3 / cost 3
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] Tamer w/[Kurata's Army] trait: Cost 2
//!
//! Printed text (official)
//! Effect
//! ＜Armor Purge＞ [On Play] [When Digivolving] [When Attacking] [Once Per Turn] Give 1 of your opponent's Digimon ＜Security A. -1＞ until their turn ends. Then, by choosing 1 of your Digimon, delete the chosen Digimon, and 1 of their Digimon gets -6000 DP until their turn ends.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

#[test]
fn armor_purge_can_save_the_base_or_be_declined() {
    armor_purge_in_battle("BT27-038");
}

#[test]
fn both_printed_standard_digivolution_routes_grant_keywords() {
    standard_routes("BT27-038", CardColor::Yellow, 3, 3, &[Keyword::ArmorPurge]);
}

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_costs_two_and_does_not_trigger() {
    tamer_evolution("BT27-038", 2, &[Keyword::ArmorPurge]);
}

#[test]
fn security_reduction_is_mandatory_even_when_deletion_is_declined() {
    let mut r = start(builder("BT27-038").hand(0, &["BT27-038"]));
    let opponent = r.place_on_field(1, HIGH, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [HIGH]);
    r.choose_prompt_card(HIGH, None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BT27-038"]);
    r.decline_prompt().unwrap();
    assert_eq!(r.effective_dp(opponent), Some(12000));
    assert!(r
        .game
        .has_keyword(opponent, Keyword::SecurityAttackMinus(1)));
    assert!(r.pending_selection().is_none());
}

#[test]
fn deleting_an_ally_can_reduce_a_different_enemy_by_six_thousand_until_their_turn_ends() {
    const CARD: &str = "BT27-038";
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, EVOLUTION, Some(0));
    let checks = r.place_on_field(1, HIGH, Some(0));
    let dp = r.place_on_field(1, BOUNDARY, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card(HIGH, None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card(EVOLUTION, None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card(BOUNDARY, None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].trash), [EVOLUTION]);
    assert!(r.game.has_keyword(checks, Keyword::SecurityAttackMinus(1)));
    assert_eq!(r.dp_of(dp), Some(5000));
    next_turn(&mut r);
    assert_eq!(r.dp_of(dp), Some(5000));
    next_turn(&mut r);
    assert_eq!(r.dp_of(dp), Some(11000));
    assert!(!r.game.has_keyword(checks, Keyword::SecurityAttackMinus(1)));
}

#[test]
fn digivolution_and_attack_share_the_once_per_turn_use() {
    let (mut r, host) = evolve_against_an_enemy("BT27-038");
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card(HIGH, None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    attack_security(&mut r, host);
    assert_eq!(r.security_count(1), 4);
    assert!(r.pending_selection().is_none());
}
