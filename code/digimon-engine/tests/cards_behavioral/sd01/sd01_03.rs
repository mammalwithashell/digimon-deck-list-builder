//! SD01-03 Liskmon — Digimon Lv.4 Red, Cost 6, DP 5000
//! [When Digivolving] This Digimon gets +2000 DP for the turn.
//! Inherited: [Your Turn] Add 1000 to this [ALYSION] trait Digimon's DP deletion effect's maximum. (Q7565, Q7566)

use crate::alysion_common::{action, advance_turn, fixture, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd01_03_when_digivolving_dp_boost_and_inherited_raises_dp_deletion_max() {
    // [When Digivolving] +2000 DP for turn.
    let mut r = start(fixture(&["SD01-03"]).hand(0, &["SD01-03"]));
    let base = r.place_stack(0, &["RED-LV3"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(r.effective_dp(base), Some(7000));
    advance_turn(&mut r);
    assert_eq!(r.effective_dp(base), Some(5000));

    // Inherited: raises SD01-04 Reptilimon's 4000 DP deletion max to 5000 DP (Q7565).
    let mut r2 = start(fixture(&["SD01-03", "SD01-04"]));
    let attacker = r2.place_stack(0, &["SD01-03", "SD01-04"]);
    r2.place_stack(1, &["PLAIN-5K"]); // 5000 DP <= 4000 + 1000
    r2.place_stack(1, &["PLAIN-6K"]); // 6000 DP > 5000
    action(&mut r2, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r2.assert_prompt(SelectionKind::OppField, false, 0);
    r2.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r2.battle_area_size(1), 1);
}
