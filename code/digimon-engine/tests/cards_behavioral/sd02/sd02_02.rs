//! SD02-02 Renamon — Digimon Lv.3 Blue, Cost 3, DP 1000
//! [On Play] Trash the bottom digivolution card of 1 of your opponent's Digimon.
//! Inherited: [When Attacking] [Once Per Turn] Trash the bottom digivolution card of 1 of your opponent's Digimon.

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd02_02_on_play_and_inherited_when_attacking_opt_strip_bottom_source() {
    let mut r = start(fixture(&["SD02-02"]).hand(0, &["SD02-02"]));
    let opp = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);

    let host = r.place_stack(0, &["SD02-02", "PLAIN-5K"]);
    action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 1);
}
