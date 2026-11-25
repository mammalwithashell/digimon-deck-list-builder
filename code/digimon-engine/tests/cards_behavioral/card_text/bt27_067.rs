//! Source: `data/card_bundles/BT27-067.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Black Lv.5 / cost 4
//!   - Red Lv.5 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.5 w/[Glowing Dawn] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Raid＞ ＜Piercing＞ ＜Engage＞ (At the end of your turn, this Digimon may attack.) [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by trashing 2 bottom face-down cards from under any of your Tamers, it doesn't leave. [All Turns] [Once Per Turn] When attack targets change or effects trash cards from under your Tamers, you may activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! ＜Use Req. ([Glowing Dawn] trait)＞ (Specified conditions meet color requirements.) [Main] ＜De-Digivolve 2＞ 1 of your opponent's Digimon. Then, 1 of your Digimon unsuspends and gets +5000 DP until your turn ends.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - A [Main] effect shown on the DUAL card's Option side will activate. Activated effects on the Option side are treated as Option card effects.

use super::common::{action, builder, ids, start};
use digimon_engine::selection::SelectionKind;

#[test]
fn artiomon_option_main_de_digivolves_then_unsuspends() {
    let mut r = start(builder("BT27-067").hand(0, &["BT27-067"]));
    let ally = r.place_on_field(0, "TEXT-3", Some(0));
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    let foe = r.place_stack(1, &["TEXT-3", "TEXT-4", "TEXT-5"]);
    action(&mut r, 0, 0);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    let enemy = r.prompt_card_candidates()[0].0;
    action(&mut r, 0, enemy);
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    let target = r.prompt_card_candidates()[0].0;
    action(&mut r, 0, target);
    assert_eq!(
        ids(
            &r,
            &r.game.players[1].battle_area[foe.index as usize].card_sources
        ),
        ["TEXT-3"]
    );
    assert!(!r.game.players[0].battle_area[ally.index as usize].is_suspended);
    assert_eq!(r.effective_dp(ally), Some(7000));
}

#[test]
fn end_turn_engage_can_raid_and_fire_the_option_side_before_piercing() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::enums::{EffectSourceKind, Keyword};
    for accept in [false, true] {
        let mut r = start(builder("BT27-067"));
        let artiomon = r.place_on_field(0, "BT27-067", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        r.game.end_turn();
        assert_eq!(
            r.game.pending_end_of_turn_keyword_decision(),
            Some(Keyword::Engage)
        );
        r.execute_action(0, encode_attack(artiomon.index as u16, SECURITY_TARGET))
            .unwrap();
        r.assert_prompt(SelectionKind::OppField, true, 0);
        r.choose_prompt_card("TEXT-3", None).unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        assert_eq!(
            r.pending_selection().unwrap().source_kind,
            EffectSourceKind::Option
        );
        if accept {
            r.accept_optional_trigger().unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            r.choose_prompt_card("TEXT-3", None).unwrap();
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            r.choose_prompt_card("BT27-067", None).unwrap();
        } else {
            r.decline_optional_trigger().unwrap();
            assert_eq!(r.effective_dp(artiomon), Some(12000));
        }
        assert_eq!(r.security_count(1), 4);
        assert_eq!(r.effective_dp(artiomon), Some(12000));
        assert!(r.game.players[1].battle_area.is_empty());
    }
}

#[test]
fn glowing_dawn_satisfies_option_use_without_matching_color_and_allows_arts_digivolve() {
    use super::common::digimon;
    use digimon_engine::enums::CardColor;
    for accept in [false, true] {
        let mut off_color = digimon("TEXT-OFF-COLOR", 5, 6000, true);
        off_color.colors = vec![CardColor::Blue];
        let mut r = start(
            builder("BT27-067")
                .add_card(off_color)
                .hand(0, &["BT27-067"]),
        );
        let host = r.place_on_field(0, "TEXT-OFF-COLOR", Some(0));
        action(&mut r, 0, 0);
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card("TEXT-OFF-COLOR", None).unwrap();
        assert_eq!(r.effective_dp(host), Some(11000));
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        if accept {
            r.choose_prompt_card("TEXT-OFF-COLOR", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.memory(), 5);
        let stack = &r.game.players[0].battle_area[host.index as usize].card_sources;
        assert_eq!(stack.len(), 1 + usize::from(accept));
        assert_eq!(
            stack.last().unwrap().card_id(&r.game.card_data),
            if accept { "BT27-067" } else { "TEXT-OFF-COLOR" }
        );
        assert_eq!(
            ids(&r, &r.game.players[0].trash).contains(&"BT27-067".to_owned()),
            !accept
        );
    }
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::{Black, Red};
    super::common::digivolve_paths("BT27-067", 5, &[(Black, 4), (Red, 4)], 3);
}

#[test]
fn two_bottom_face_down_cards_replace_leaving_but_only_once_per_turn() {
    use digimon_engine::replacement::ReplacementCause;
    for accept in [false, true] {
        let mut r = start(builder("BT27-067"));
        let artiomon = r.place_on_field(0, "BT27-067", Some(0));
        let stock = super::common::stocked_tamer(&mut r, "TEXT-TAMER");
        assert!(r
            .game
            .has_keyword(artiomon, digimon_engine::enums::Keyword::Raid));
        assert!(r
            .game
            .has_keyword(artiomon, digimon_engine::enums::Keyword::Piercing));
        assert!(r
            .game
            .has_keyword(artiomon, digimon_engine::enums::Keyword::Engage));
        r.game
            .delete_permanent_with_cause(artiomon, ReplacementCause::OpponentEffect);
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if !accept {
            r.decline_optional_trigger().unwrap();
            assert!(ids(&r, &r.game.players[0].trash).contains(&"BT27-067".to_owned()));
            assert!(ids(&r, &r.game.players[0].battle_area[0].card_sources)
                .contains(&"TEXT-BOTTOM".to_owned()));
            continue;
        }
        r.accept_optional_trigger().unwrap();
        super::common::pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 2);
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.decline_optional_trigger().unwrap();
        assert_eq!(
            ids(
                &r,
                &r.game.players[0].battle_area[artiomon.index as usize].card_sources
            ),
            ["BT27-067"]
        );
        assert_eq!(
            ids(
                &r,
                &r.game.players[0].battle_area[stock.index as usize].card_sources
            ),
            ["TEXT-TAMER"]
        );
        super::common::stocked_tamer(&mut r, "TEXT-TAMER");
        r.game
            .delete_permanent_with_cause(artiomon, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection().is_none());
        assert!(ids(&r, &r.game.players[0].trash).contains(&"BT27-067".to_owned()));
    }
}
