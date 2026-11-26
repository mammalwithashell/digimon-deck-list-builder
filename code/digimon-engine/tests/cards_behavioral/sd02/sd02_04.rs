//! SD02-04 Sekkamon — Digimon Lv.5 Blue, Cost 7, DP 6000
//! [When Digivolving] Trash the bottom 2 digivolution cards of 1 of your opponent's Digimon.
//! Inherited: [When Attacking] [Once Per Turn] If this Digimon has the [ALYSION] trait, it unsuspends.

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd02_04_when_digivolving_strips_2_and_inherited_unsuspends_alysion_once_per_turn() {
    let mut r = start(fixture(&["SD02-04"]).hand(0, &["SD02-04"]));
    let base = r.place_stack(0, &["BLUE-LV4"]);
    let opp = r.place_stack(1, &["FILLER", "FILLER", "FILLER", "PLAIN-5K"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);

    // Inherited: [ALYSION] unsuspends on first attack, stays suspended on second attack (OPT).
    let alysion = r.place_stack(0, &["SD02-04", "BLUE-ALYSION"]);
    action(&mut r, 0, encode_attack(alysion.index as u16, SECURITY_TARGET));
    assert!(!r.game.players[0].battle_area[alysion.index as usize].is_suspended);
    action(&mut r, 0, encode_attack(alysion.index as u16, SECURITY_TARGET));
    assert!(r.game.players[0].battle_area[alysion.index as usize].is_suspended);

    // Non-[ALYSION] does NOT unsuspend.
    let plain = r.place_stack(0, &["SD02-04", "PLAIN-5K"]);
    action(&mut r, 0, encode_attack(plain.index as u16, SECURITY_TARGET));
    assert!(r.game.players[0].battle_area[plain.index as usize].is_suspended);
}
