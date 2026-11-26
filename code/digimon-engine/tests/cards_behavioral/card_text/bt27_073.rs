//! Source: `data/card_bundles/BT27-073.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Purple Lv.2 / cost 0
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [Your Turn] When this Digimon would digivolve into a purple Digimon card with the [BEATBREAK] trait, reduce the cost by 1.
//!
//! ### Inherited Effect
//! [When Attacking] [Once Per Turn] ＜Draw 1＞ and trash 1 card in your hand.
//!
//! ## Official Q&A
//! - No, it does not. Effects do not trigger in the breeding area.

use super::common::rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area;
use digimon_engine::enums::CardColor;

#[test]
fn printed_rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area() {
    rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area(
        "BT27-073",
        CardColor::Purple,
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Purple;
    super::common::digivolve_paths("BT27-073", 2, &[(Purple, 0)], 0);
}

#[test]
fn breeding_area_digivolution_does_not_reduce_the_cost() {
    use super::common::{action, builder, digimon, start};
    use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET};
    let mut target = digimon("TEXT-EVO", 4, 5000, true);
    target.colors = vec![CardColor::Purple];
    let mut r = start(builder("BT27-073").add_card(target).hand(0, &["TEXT-EVO"]));
    r.place_in_breeding(0, "BT27-073");
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, BREEDING_TARGET));
    assert_eq!(before - r.memory(), 4);
}

#[test]
fn inherited_attack_draws_then_requires_a_discard_only_once_each_turn() {
    use super::common::{action, builder, candidates, ids, start};
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::selection::SelectionKind;
    let mut r = start(builder("BT27-073").hand(0, &["TEXT-3"]));
    let host = r.place_stack(0, &["BT27-073", "TEXT-6"]);
    for first in [true, false] {
        let deck = r.game.players[0].deck.len();
        action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
        if first {
            r.assert_prompt(SelectionKind::Hand, false, 0);
            assert_eq!(candidates(&r), ["TEXT-3", "TEXT-FILLER"]);
            r.choose_prompt_card("TEXT-3", None).unwrap();
            assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-3"]);
        }
        assert_eq!(r.game.players[0].deck.len(), deck - usize::from(first));
        r.game.unsuspend(host);
    }
}
