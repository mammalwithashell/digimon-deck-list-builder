//! BD01-06 Rubeusdramon — Digimon Lv.6 Red, Cost 12, DP 11000
//! <Security A. +1>
//! [When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with 4000 DP or less.

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn bd01_06_sa_plus_1_and_when_attacking_opt_deletes_4000_or_less() {
    let mut r = start(fixture(&["BD01-06"]));
    let rubeus = r.place_stack(0, &["BD01-06"]);
    r.place_stack(1, &["PLAIN-4K"]);
    r.place_stack(1, &["PLAIN-5K"]);
    assert_eq!(r.game.effective_security_strike(rubeus), 2);

    action(&mut r, 0, encode_attack(rubeus.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-4K"]);
    r.choose_prompt_card("PLAIN-4K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.security_count(1), 1); // 3 - 2 checks = 1

    // OPT: second attack in same turn does not trigger delete again.
    r.place_stack(1, &["PLAIN-2K"]);
    r.game.players[0].battle_area[rubeus.index as usize].is_suspended = false;
    action(&mut r, 0, encode_attack(rubeus.index as u16, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 2);
}
