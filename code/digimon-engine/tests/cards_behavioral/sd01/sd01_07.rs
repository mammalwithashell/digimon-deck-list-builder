//! SD01-07 Crimson Impale — Option Red, Cost 1
//! [Main] 1 of your Digimon with the [ALYSION] trait can also attack your opponent's unsuspended Digimon for the turn.
//! [Security] Add this card to the hand.

use crate::alysion_common::{action, candidates, fixture, ids, start};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd01_07_main_grants_can_attack_unsuspended_and_security_adds_to_hand() {
    let mut r = start(fixture(&["SD01-07"]).hand(0, &["SD01-07"]));
    let alysion = r.place_stack(0, &["RED-ALYSION"]);
    let _plain = r.place_stack(0, &["PLAIN-5K"]);
    let opp = r.place_stack(1, &["PLAIN-3K"]);

    assert!(build_action_mask(&r.game, 0)[encode_attack(alysion.index as u16, opp.index as u16) as usize] < 0.5);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["RED-ALYSION"]);
    r.choose_prompt_card("RED-ALYSION", None).unwrap();
    assert!(build_action_mask(&r.game, 0)[encode_attack(alysion.index as u16, opp.index as u16) as usize] > 0.5);

    // [Security] Add this card to the hand.
    let mut r2 = start(fixture(&["SD01-07"]).security(1, &["SD01-07"]));
    let attacker = r2.place_stack(0, &["PLAIN-12K"]);
    action(&mut r2, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    assert_eq!(ids(&r2, &r2.game.players[1].hand), ["SD01-07"]);
}
