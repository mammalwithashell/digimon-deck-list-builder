//! Source: `data/card_bundles/BT27-080.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Purple Lv.4 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.4 w/[Glowing Dawn] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! {Hand} [Main] If you have a Tamer with [Makoto Kuonji] in its name, by placing 1 [NightChiropmon] from your trash as any of your [Chiropmon]'s bottom digivolution card, that Digimon digivolves into this card, ignoring level. [On Play] [When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon. Then, by trashing the bottom face-down card from under any of your Tamers, this Digimon gains ＜Rush＞ and ＜Piercing＞ for the turn.
//!
//! ### Inherited Effect
//! [On Deletion] You may play 1 play cost 5 or lower [Glowing Dawn] trait card from your trash without paying the cost.
//!
//! ## Official Q&A
//! - A {Hand} effect can be triggered and activated while its card is in your hand. Such effects can't be triggered or activated in areas other than your hand.

use super::common::hand_main_stacks_the_named_material_and_ignores_the_rookies_level;

#[test]
fn printed_hand_main_stacks_the_named_material_and_ignores_the_rookies_level() {
    hand_main_stacks_the_named_material_and_ignores_the_rookies_level(
        "BT27-080", "BT27-097", "BT27-073", "BT27-077",
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Purple;
    super::common::digivolve_paths("BT27-080", 4, &[(Purple, 3)], 3);
}

#[test]
fn on_play_deletes_a_level_four_or_lower_then_can_gain_rush_and_piercing() {
    use super::common::{action, builder, candidates, next_turn, start, stocked_tamer};
    use digimon_engine::enums::Keyword;
    use digimon_engine::selection::SelectionKind;
    for pay in [false, true] {
        let mut r = start(builder("BT27-080").hand(0, &["BT27-080"]));
        stocked_tamer(&mut r, "TEXT-TAMER");
        r.place_on_field(1, "TEXT-4", Some(0));
        r.place_on_field(1, "TEXT-5", Some(0));
        action(&mut r, 0, 0);
        r.assert_prompt(SelectionKind::OppField, false, 0);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        r.choose_prompt_card("TEXT-4", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        if pay {
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        let host = digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 1,
        };
        assert_eq!(r.game.has_keyword(host, Keyword::Rush), pay);
        assert_eq!(r.game.has_keyword(host, Keyword::Piercing), pay);
        next_turn(&mut r);
        assert!(!r.game.has_keyword(host, Keyword::Rush));
        assert!(!r.game.has_keyword(host, Keyword::Piercing));
    }
}

#[test]
fn inherited_deletion_can_revive_only_a_glowing_dawn_card_costing_at_most_five() {
    use super::common::{builder, candidates, digimon, ids, start};
    use digimon_engine::replacement::ReplacementCause;
    use digimon_engine::selection::SelectionKind;
    for accept in [false, true] {
        let mut eligible = digimon("TEXT-ELIGIBLE", 4, 5000, true);
        eligible.play_cost = 5;
        let mut expensive = digimon("TEXT-EXPENSIVE", 4, 5000, true);
        expensive.play_cost = 6;
        let mut r = start(builder("BT27-080").add_card(eligible).add_card(expensive));
        for id in ["TEXT-ELIGIBLE", "TEXT-EXPENSIVE", "TEXT-OTHER"] {
            r.inject_trash(0, id);
        }
        let host = r.place_stack(0, &["BT27-080", "TEXT-6"]);
        let memory = r.memory();
        r.game
            .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        r.game.drain_effect_queue();
        r.assert_prompt(SelectionKind::Trash, true, 0);
        assert_eq!(candidates(&r), ["TEXT-6", "TEXT-ELIGIBLE"]);
        if accept {
            r.choose_prompt_card("TEXT-ELIGIBLE", None).unwrap();
            assert_eq!(
                ids(&r, &r.game.players[0].battle_area[0].card_sources),
                ["TEXT-ELIGIBLE"]
            );
        } else {
            r.decline_prompt().unwrap();
            assert!(r.game.players[0].battle_area.is_empty());
        }
        assert_eq!(r.memory(), memory);
    }
}
