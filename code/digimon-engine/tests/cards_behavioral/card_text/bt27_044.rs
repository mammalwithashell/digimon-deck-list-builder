//! Source: `data/card_bundles/BT27-044.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Green Lv.2 / cost 0
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [Your Turn] When this Digimon would digivolve into a green Digimon card with the [BEATBREAK] trait, reduce the cost by 1.
//!
//! ### Inherited Effect
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! ## Official Q&A
//! - No, it does not. Effects do not trigger in the breeding area.

use super::common::rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area;
use digimon_engine::enums::CardColor;

#[test]
fn printed_rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area() {
    rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area(
        "BT27-044",
        CardColor::Green,
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Green;
    super::common::digivolve_paths("BT27-044", 2, &[(Green, 0)], 0);
}

#[test]
fn breeding_area_digivolution_does_not_reduce_the_cost() {
    use super::common::{action, builder, digimon, start};
    use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET};
    let mut target = digimon("TEXT-EVO", 4, 5000, true);
    target.colors = vec![CardColor::Green];
    let mut r = start(builder("BT27-044").add_card(target).hand(0, &["TEXT-EVO"]));
    r.place_in_breeding(0, "BT27-044");
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, BREEDING_TARGET));
    assert_eq!(before - r.memory(), 4);
}

#[test]
fn inherited_two_thousand_dp_only_applies_during_your_turn() {
    use super::common::{builder, next_turn, start};
    let mut r = start(builder("BT27-044"));
    let host = r.place_stack(0, &["BT27-044", "TEXT-4"]);
    assert_eq!(r.effective_dp(host), Some(6000));
    next_turn(&mut r);
    assert_eq!(r.effective_dp(host), Some(4000));
}
