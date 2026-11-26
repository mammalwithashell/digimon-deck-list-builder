//! SD01-06 Kanata Hondo — Tamer Red, Cost 4
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Your Turn] Add 2000 to your [ALYSION] trait Digimon's DP deletion effects' maximums.
//! [Security] Play this card without paying the cost.

use crate::alysion_common::{action, assert_memory_setter, assert_security_play, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd01_06_memory_setter_security_play_and_board_aura_raises_alysion_deletion_max() {
    assert_memory_setter("SD01-06");
    assert_security_play("SD01-06");

    // [Your Turn] board aura adds +2000 to SD01-04 Reptilimon's 4000 DP deletion max -> 6000 DP!
    let mut r = start(fixture(&["SD01-06", "SD01-04"]));
    r.place_stack(0, &["SD01-06"]);
    let attacker = r.place_stack(0, &["SD01-04"]);
    r.place_stack(1, &["PLAIN-6K"]);
    r.place_stack(1, &["PLAIN-7K"]);
    r.game.tick_declarative_effects();
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-6K"]);
    r.choose_prompt_card("PLAIN-6K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
}
