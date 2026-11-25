//! Source: `data/card_bundles/BT27-050.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Green Lv.4 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.4 w/[Glowing Dawn] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! {Hand} [Main] If you have a Tamer with [Tomoro Tenma] in its name, by placing 1 [Armalizamon] from your trash as any of your [Gekkomon]'s bottom digivolution card, that Digimon digivolves into this card, ignoring level. [On Play] [When Digivolving] You may suspend 1 of your opponent's Digimon or Tamers. Then, by trashing the bottom face-down card from under any of your Tamers, 1 of their Digimon or Tamers can't unsuspend until their turn ends.
//!
//! ### Inherited Effect
//! [End of Attack] [Once Per Turn] By trashing the bottom face-down card from under any of your Tamers, this [Glowing Dawn] trait Digimon unsuspends.
//!
//! ## Official Q&A
//! - A {Hand} effect can be triggered and activated while its card is in your hand. Such effects can't be triggered or activated in areas other than your hand.

use super::common::hand_main_stacks_the_named_material_and_ignores_the_rookies_level;

#[test]
fn printed_hand_main_stacks_the_named_material_and_ignores_the_rookies_level() {
    hand_main_stacks_the_named_material_and_ignores_the_rookies_level(
        "BT27-050", "BT27-091", "BT27-044", "BT27-046",
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Green;
    super::common::digivolve_paths("BT27-050", 4, &[(Green, 3)], 3);
}

#[test]
fn on_play_suspends_and_paid_bottom_card_prevents_unsuspending() {
    use super::common::{action, builder, candidates, ids, next_turn, start, stocked_tamer};
    use digimon_engine::selection::SelectionKind;
    for accept in [false, true] {
        let mut r = start(builder("BT27-050").hand(0, &["BT27-050"]));
        let stock = stocked_tamer(&mut r, "TEXT-TAMER");
        let foe = r.place_on_field(1, "TEXT-TAMER", Some(0));
        r.place_on_field(1, "TEXT-4", Some(0));
        action(&mut r, 0, 0);
        r.assert_prompt(SelectionKind::OppField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-4", "TEXT-TAMER"]);
        r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        if accept {
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            assert_eq!(
                ids(
                    &r,
                    &r.game.players[0].battle_area[stock.index as usize].card_sources
                ),
                ["TEXT-TOP", "TEXT-TAMER"]
            );
        } else {
            r.decline_prompt().unwrap();
        }
        next_turn(&mut r);
        assert_eq!(
            r.game.players[1].battle_area[foe.index as usize].is_suspended,
            accept
        );
    }
}

#[test]
fn inherited_glowing_dawn_attack_can_pay_to_unsuspend_once_per_turn() {
    use super::common::{action, builder, start, stocked_tamer};
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::selection::SelectionKind;
    let mut r = start(builder("BT27-050"));
    stocked_tamer(&mut r, "TEXT-TAMER");
    let host = r.place_stack(0, &["BT27-050", "TEXT-6"]);
    for first in [true, false] {
        action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
        if first {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.accept_optional_trigger().unwrap();
            super::common::pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 1);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            r.game.players[0].battle_area[host.index as usize].is_suspended,
            !first
        );
    }
}
