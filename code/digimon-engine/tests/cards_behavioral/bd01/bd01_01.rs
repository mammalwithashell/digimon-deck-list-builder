//! BD01-01 Tokomon — Digi-Egg Lv.2 Red
//! Inherited: [Your Turn] Raise the limit of the DP deletion effect for this Digimon with the [ALYSION] trait by +1000.

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn bd01_01_inherited_raises_alysion_dp_deletion_limit_by_1000() {
    let mut r = start(fixture(&["BD01-01", "SD01-04"]));
    let attacker = r.place_stack(0, &["BD01-01", "SD01-04"]); // 4000 + 1000 = 5000
    r.place_stack(1, &["PLAIN-5K"]);
    r.place_stack(1, &["PLAIN-6K"]);
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-5K"]);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
}
