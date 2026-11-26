//! SD01-01 Tokomon — Digi-Egg Lv.2 Red
//! Inherited: [Your Turn] This Digimon with the [ALYSION] trait gets +1000 DP.

use crate::alysion_common::{advance_turn, fixture, start};

#[test]
fn sd01_01_inherited_dp_applies_only_to_alysion_on_your_turn() {
    let mut r = start(fixture(&["SD01-01"]));
    let alysion = r.place_stack(0, &["SD01-01", "RED-ALYSION"]);
    let plain = r.place_stack(0, &["SD01-01", "PLAIN-5K"]);
    assert_eq!(r.effective_dp(alysion), Some(6000));
    assert_eq!(r.effective_dp(plain), Some(5000));
    advance_turn(&mut r);
    assert_eq!(r.effective_dp(alysion), Some(5000));
}
