//! Source: `data/card_bundles/BT27-048.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Green Lv.3 / cost 3
//! - Purple Lv.3 / cost 3
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] Tamer w/[Kurata's Army] trait: Cost 2
//!
//! Printed text (official)
//! Effect
//! ＜Armor Purge＞ [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may suspend 1 Digimon. Then, by choosing 1 of your Digimon, delete the chosen Digimon, and 1 of your opponent's Digimon or Tamers can't unsuspend until their turn ends. (Rule) Trait: Has [Dinosaur] Type.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

#[test]
fn armor_purge_can_save_the_base_or_be_declined() {
    armor_purge_in_battle("BT27-048");
}

#[test]
fn both_printed_standard_digivolution_routes_grant_keywords() {
    standard_routes("BT27-048", CardColor::Green, 3, 3, &[Keyword::ArmorPurge]);
}

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_costs_two_and_does_not_trigger() {
    tamer_evolution("BT27-048", 2, &[Keyword::ArmorPurge]);
}

#[test]
fn on_play_may_suspend_either_sides_digimon_but_not_a_tamer() {
    let mut r = start(builder("BT27-048").hand(0, &["BT27-048"]));
    let opponent = r.place_on_field(1, HIGH, Some(0));
    r.place_on_field(1, TAMER, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::AnyField, true, 0);
    assert_eq!(candidates(&r), ["BT27-048", HIGH]);
    r.choose_prompt_card(HIGH, None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BT27-048"]);
    r.decline_prompt().unwrap();
    assert!(r.game.players[1].battle_area[opponent.index as usize].is_suspended);
    assert!(r.pending_selection().is_none());
}

#[test]
fn declining_suspension_can_still_delete_an_ally_and_lock_a_tamer() {
    use digimon_engine::enums::ModifierType;
    const CARD: &str = "BT27-048";
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, EVOLUTION, Some(0));
    let tamer = r.place_on_field(1, TAMER, Some(0));
    r.place_on_field(1, HIGH, Some(0));
    r.game.players[1].battle_area[tamer.index as usize].is_suspended = true;
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::AnyField, true, 0);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card(EVOLUTION, None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [HIGH, TAMER]);
    r.choose_prompt_card(TAMER, None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].trash), [EVOLUTION]);
    assert!(r.game.modifiers.has(tamer, ModifierType::CannotUnsuspend));
    next_turn(&mut r);
    assert!(r.game.players[1].battle_area[tamer.index as usize].is_suspended);
    next_turn(&mut r);
    assert!(!r.game.modifiers.has(tamer, ModifierType::CannotUnsuspend));
}

#[test]
fn stegomon_has_the_additional_dinosaur_trait() {
    let r = start(builder("BT27-048"));
    assert!(r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "BT27-048")
        .unwrap()
        .traits
        .iter()
        .any(|t| t == "Dinosaur"));
}

#[test]
fn successful_digivolution_effect_prevents_reuse_when_attacking_in_the_same_turn() {
    let (mut r, host) = evolve_against_an_enemy("BT27-048");
    r.assert_prompt(SelectionKind::AnyField, true, 0);
    r.choose_prompt_card(HIGH, None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    attack_security(&mut r, host);
    assert_eq!(r.security_count(1), 4);
    assert!(r.pending_selection().is_none());
}
