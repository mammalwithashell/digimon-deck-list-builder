//! SD01-04 Reptilimon — Digimon Lv.5 Red, Cost 7, DP 7000
//! [When Attacking] Delete 1 of your opponent's Digimon with 4000 DP or less.
//! Inherited: [Your Turn] This Digimon with the [ALYSION] trait gains <Security A. +1>.

use crate::alysion_common::{action, advance_turn, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd01_04_when_attacking_deletes_4000_or_less_and_inherited_alysion_sa_plus_1() {
    let mut r = start(fixture(&["SD01-04"]));
    let attacker = r.place_stack(0, &["SD01-04"]);
    r.place_stack(1, &["PLAIN-4K"]);
    r.place_stack(1, &["PLAIN-5K"]);
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-4K"]);
    r.choose_prompt_card("PLAIN-4K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);

    // Inherited [Your Turn] <Security A. +1> only on [ALYSION].
    let mut r2 = start(fixture(&["SD01-04"]));
    let alysion = r2.place_stack(0, &["SD01-04", "RED-ALYSION"]);
    let plain = r2.place_stack(0, &["SD01-04", "PLAIN-5K"]);
    r2.game.tick_declarative_effects();
    assert_eq!(r2.game.effective_security_strike(alysion), 2);
    assert_eq!(r2.game.effective_security_strike(plain), 1);
    advance_turn(&mut r2);
    assert_eq!(r2.game.effective_security_strike(alysion), 1);
}
