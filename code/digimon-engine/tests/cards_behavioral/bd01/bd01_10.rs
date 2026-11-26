//! BD01-10 Kanata Hondo — Tamer Red, Cost 3
//! [Start of Your Main Phase] If you have a red Digimon with the [ALYSION] trait, gain 1 memory.
//! [Your Turn] All of your red Digimon with the [ALYSION] trait get +1000 DP.
//! [Security] Play this card without paying the cost.

use crate::alysion_common::{advance_turn, assert_security_play, fixture, start};

#[test]
fn bd01_10_start_of_main_memory_aura_dp_and_security_play() {
    assert_security_play("BD01-10");

    // [Your Turn] +1000 DP only to red [ALYSION] Digimon.
    let mut r = start(fixture(&["BD01-10"]));
    r.place_stack(0, &["BD01-10"]);
    let red_a = r.place_stack(0, &["RED-ALYSION"]);
    let blue_a = r.place_stack(0, &["BLUE-ALYSION"]);
    let red_plain = r.place_stack(0, &["RED-LV4"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(red_a), Some(6000));
    assert_eq!(r.effective_dp(blue_a), Some(5000));
    assert_eq!(r.effective_dp(red_plain), Some(5000));

    advance_turn(&mut r);
    assert_eq!(r.effective_dp(red_a), Some(5000));

    // [Start of Your Main Phase] +1 memory when red [ALYSION] Digimon is on field.
    r.game.set_memory(-3);
    r.game.end_turn(); // P0's turn starts with 3 memory -> Start of Main Phase +1 -> 4
    assert_eq!(r.memory(), 4);
}
