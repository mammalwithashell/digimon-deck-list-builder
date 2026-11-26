//! ST26-01 Viximon — Digi-Egg Lv.2 Blue
//! Inherited: [Your Turn] While your opponent has no Digimon with digivolution cards, this [Ice-Snow] trait Digimon can't be deleted in battle. (Q7571, Q7572)

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};

#[test]
fn st26_01_inherited_battle_protection_requires_no_opp_sourced_digimon_and_ice_snow() {
    // Opponent has no Digimon at all (Q7571 met) and security has a 12k Digimon -> [Ice-Snow] survives battle!
    let mut r = start(fixture(&["ST26-01"]).security(1, &["PLAIN-12K"]));
    let ice = r.place_stack(0, &["ST26-01", "ICE-MON"]); // 5000 DP, Ice-Snow
    action(&mut r, 0, encode_attack(ice.index as u16, SECURITY_TARGET));
    assert_eq!(r.battle_area_size(0), 1);

    // Opponent has a Digimon WITH a digivolution card -> protection inactive, deleted by 12k security.
    let mut r2 = start(fixture(&["ST26-01"]).security(1, &["PLAIN-12K"]));
    let ice2 = r2.place_stack(0, &["ST26-01", "ICE-MON"]);
    r2.place_stack(1, &["FILLER", "PLAIN-5K"]);
    action(&mut r2, 0, encode_attack(ice2.index as u16, SECURITY_TARGET));
    assert_eq!(r2.battle_area_size(0), 0);

    // Non-[Ice-Snow] host -> protection inactive even when opponent has no Digimon.
    let mut r3 = start(fixture(&["ST26-01"]).security(1, &["PLAIN-12K"]));
    let plain = r3.place_stack(0, &["ST26-01", "PLAIN-5K"]);
    action(&mut r3, 0, encode_attack(plain.index as u16, SECURITY_TARGET));
    assert_eq!(r3.battle_area_size(0), 0);
}
