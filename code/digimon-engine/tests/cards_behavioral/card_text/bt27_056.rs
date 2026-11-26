//! Source: `data/card_bundles/BT27-056.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Black Lv.2 / cost 0
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [Your Turn] When this Digimon would digivolve into a black Digimon card with the [BEATBREAK] trait, reduce the cost by 1.
//!
//! ### Inherited Effect
//! ＜Reboot＞ (This Digimon also unsuspends in your opponent's unsuspend phase.)
//!
//! ## Official Q&A
//! - No, it does not. Effects do not trigger in the breeding area.

use super::common::rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area;
use digimon_engine::enums::CardColor;

#[test]
fn printed_rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area() {
    rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area(
        "BT27-056",
        CardColor::Black,
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Black;
    super::common::digivolve_paths("BT27-056", 2, &[(Black, 0)], 0);
}

#[test]
fn breeding_area_digivolution_does_not_reduce_the_cost() {
    use super::common::{action, builder, digimon, start};
    use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET};
    let mut target = digimon("TEXT-EVO", 4, 5000, true);
    target.colors = vec![CardColor::Black];
    let mut r = start(builder("BT27-056").add_card(target).hand(0, &["TEXT-EVO"]));
    r.place_in_breeding(0, "BT27-056");
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, BREEDING_TARGET));
    assert_eq!(before - r.memory(), 4);
}

#[test]
fn inherited_reboot_unsuspends_the_evolved_digimon_on_the_opponents_turn() {
    use super::common::{builder, next_turn, start};
    let mut r = start(builder("BT27-056"));
    let host = r.place_stack(0, &["BT27-056", "TEXT-4"]);
    let other = r.place_on_field(0, "TEXT-4", Some(0));
    for h in [host, other] {
        r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    }
    next_turn(&mut r);
    assert!(!r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert!(r.game.players[0].battle_area[other.index as usize].is_suspended);
}
