//! Source: `data/card_bundles/BT27-064.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Black Lv.4 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.4 w/[Glowing Dawn] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! {Hand} [Main] If you have a Tamer with [Reina Sakuya] in its name, by placing 1 [Wolvermon] from your trash as any of your [Pristimon]'s bottom digivolution card, that Digimon digivolves into this card, ignoring level. [On Play] [When Digivolving] ＜De-Digivolve 1＞ 1 of your opponent's Digimon. Then, by trashing the bottom face-down card from under any of your Tamers, give 1 of their Digimon "[Start of Your Main Phase] This Digimon attacks." until their turn ends.
//!
//! ### Inherited Effect
//! [All Turns] [Once Per Turn] When attack targets change, if this Digimon has the [Glowing Dawn] trait, by trashing the bottom face-down card from under any of your Tamers, trash your opponent's top security card.
//!
//! ## Official Q&A
//! - A {Hand} effect can be triggered and activated while its card is in your hand. Such effects can't be triggered or activated in areas other than your hand.

use super::common::hand_main_stacks_the_named_material_and_ignores_the_rookies_level;

#[test]
fn printed_hand_main_stacks_the_named_material_and_ignores_the_rookies_level() {
    hand_main_stacks_the_named_material_and_ignores_the_rookies_level(
        "BT27-064", "BT27-094", "BT27-056", "BT27-058",
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Black;
    super::common::digivolve_paths("BT27-064", 4, &[(Black, 3)], 3);
}

#[test]
fn on_play_de_digivolves_and_can_pay_to_make_a_digimon_attack_next_main_phase() {
    use super::common::{action, builder, ids, next_turn, start, stocked_tamer};
    use digimon_engine::selection::SelectionKind;
    for pay in [false, true] {
        let mut r = start(builder("BT27-064").hand(0, &["BT27-064"]));
        stocked_tamer(&mut r, "TEXT-TAMER");
        let foe = r.place_stack(1, &["TEXT-3", "TEXT-4"]);
        action(&mut r, 0, 0);
        r.assert_prompt(SelectionKind::OppField, false, 0);
        r.choose_prompt_card("TEXT-4", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        if pay {
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            r.choose_prompt_card("TEXT-3", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(
            ids(
                &r,
                &r.game.players[1].battle_area[foe.index as usize].card_sources
            ),
            ["TEXT-3"]
        );
        next_turn(&mut r);
        if pay {
            use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
            r.assert_prompt(SelectionKind::Target, false, 1);
            let pick = encode_attack(foe.index as u16, SECURITY_TARGET);
            action(&mut r, 1, pick);
            assert_eq!(r.security_count(0), 4);
        } else {
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn inherited_attack_redirection_can_pay_one_card_to_trash_security() {
    use super::common::{action, builder, pay_tamer_sources, start, stocked_tamer};
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::selection::SelectionKind;
    let mut r = start(builder("BT27-064").dsl_card("BT27-058").unwrap());
    stocked_tamer(&mut r, "TEXT-TAMER");
    let attacker = r.place_stack(0, &["BT27-064", "TEXT-6"]);
    r.place_stack(1, &["BT27-058", "TEXT-4"]);
    let security = r.security_count(1);
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OwnField, true, 1);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 1);
    assert_eq!(r.security_count(1), security - 1);
}
