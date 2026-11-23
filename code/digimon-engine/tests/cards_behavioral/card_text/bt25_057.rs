//! Source: `data/card_bundles/BT25-057.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles:
//! Green Lv.4 / cost 4
//! Black Lv.4 / cost 4
//! Special digivolution condition:
//! [Digivolve] Lv.4 w/[Glowing Dawn] trait: Cost 3
//! Effect:
//! [When Digivolving] [When Attacking] [Once Per Turn] By trashing the bottom face-down card under any of your Tamers, ＜De-Digivolve 1＞ 1 of your opponent's Digimon. [When Digivolving] This Digimon may battle 1 of your opponent's Digimon.
//! DUAL Effect:
//! ＜Use Req. ([Glowing Dawn] trait)＞ (Specified cards let you ignore color requirements.) [Main] 1 of your Digimon gains ＜Rush＞, ＜Security A. +1＞ and +5000 DP until your opponent's turn ends. Then, it may attack.
//! DUAL Rule:
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//! Official Q&A:
//! The effects trigger simultaneously, so the player can choose the activation order.

use super::common::*;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT25-057";

#[test]
fn standard_and_glowing_dawn_special_digivolution_pay_the_printed_costs() {
    for (color, glowing, cost) in [
        (CardColor::Green, false, 4),
        (CardColor::Black, false, 4),
        (CardColor::Purple, true, 3),
    ] {
        let mut base = digimon("TEXT-BASE", 4, 4000, glowing);
        base.colors = vec![color];
        let mut r = start(builder(CARD).add_card(base).hand(0, &[CARD]));
        let h = r.place_on_field(0, "TEXT-BASE", Some(0));
        action(&mut r, 0, encode_digivolve(0, h.index as u16));
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 10 - cost);
        assert_eq!(
            r.game.players[0].battle_area[h.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            CARD
        );
    }
}

#[test]
fn when_digivolving_may_battle_an_unsuspended_opponents_digimon_or_decline() {
    for accept in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        let base = r.place_on_field(0, "TEXT-4", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-TAMER", Some(0));
        action(&mut r, 0, encode_digivolve(0, base.index as u16));
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(0).unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if accept {
            r.accept_optional_trigger().unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            assert_eq!(candidates(&r), ["TEXT-3"]);
            r.choose_prompt_card("TEXT-3", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            r.game.players[1].battle_area.len(),
            if accept { 1 } else { 2 }
        );
        assert!(!r.game.players[0].battle_area[base.index as usize].is_suspended);
        assert_eq!(r.security_count(1), 5);
    }
}

#[test]
fn when_attacking_trashes_bottom_face_down_card_to_de_digivolve_one_only_once_per_turn() {
    let mut r = start(builder(CARD));
    let attacker = r.place_on_field(0, CARD, Some(0));
    stocked_tamer(&mut r, "TEXT-TAMER");
    let target = r.place_stack(1, &["TEXT-4", "TEXT-6"]);
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, target.index as u16),
    );
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-TAMER"]);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-6"]);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-6", "TEXT-4"]);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
    r.game.players[0].battle_area[attacker.index as usize].is_suspended = false;
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
}

#[test]
fn official_qa_when_digivolving_allows_both_activation_orders() {
    for battle_first in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        let base = r.place_on_field(0, "TEXT-4", Some(0));
        stocked_tamer(&mut r, "TEXT-TAMER");
        r.place_stack(1, &["TEXT-4", "TEXT-6"]);
        r.place_on_field(1, "TEXT-3", Some(0));
        action(&mut r, 0, encode_digivolve(0, base.index as u16));
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(0).unwrap();
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        assert_eq!(
            r.pending_selection()
                .unwrap()
                .effect_choices
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        r.execute_branch(usize::from(battle_first)).unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.accept_optional_trigger().unwrap();
        if battle_first {
            r.assert_prompt(SelectionKind::OppField, false, 0);
            r.choose_prompt_card("TEXT-3", None).unwrap();
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.accept_optional_trigger().unwrap();
        }
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        r.assert_prompt(SelectionKind::OppField, false, 0);
        r.choose_prompt_card("TEXT-6", None).unwrap();
        if !battle_first {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.accept_optional_trigger().unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            r.choose_prompt_card("TEXT-3", None).unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[1].battle_area.len(), 1);
        assert_eq!(
            r.game.players[1].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "TEXT-4"
        );
        assert_eq!(r.memory(), 7);
        assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
    }
}

#[test]
fn dual_use_requirement_glowing_dawn_ignores_green_but_unrelated_trait_does_not() {
    for glowing in [false, true] {
        let mut support = make_test_card("TEXT-WHITE", "White support");
        support.colors = vec![CardColor::White];
        if glowing {
            support.traits = vec!["Glowing Dawn".into()];
        }
        let mut r = start(builder(CARD).add_card(support).hand(0, &[CARD]));
        r.place_on_field(0, "TEXT-WHITE", Some(0));
        assert_eq!(build_action_mask(&r.game, 0)[0] > 0.5, glowing);
    }
}

#[test]
fn dual_main_grants_rush_security_attack_and_five_thousand_dp_until_opponents_turn_ends() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let target = r.place_on_field(0, "TEXT-3", Some(0));
    r.place_on_field(0, "TEXT-TAMER", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    r.assert_prompt(SelectionKind::Target, true, 0);
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 6);
    assert_eq!(r.dp_of(target), Some(7000));
    assert!(r.game.has_keyword(target, Keyword::Rush));
    assert_eq!(r.game.security_attack_keyword_bonus(target), 1);
    next_turn(&mut r);
    assert_eq!(r.dp_of(target), Some(7000));
    assert!(r.game.has_keyword(target, Keyword::Rush));
    next_turn(&mut r);
    assert_eq!(r.dp_of(target), Some(2000));
    assert!(!r.game.has_keyword(target, Keyword::Rush));
    assert_eq!(r.game.security_attack_keyword_bonus(target), 0);
}

#[test]
fn dual_main_may_attack_immediately_even_with_digimon_played_this_turn() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let target = r.place_on_field(0, "TEXT-3", None);
    let enemy = r.place_on_field(1, "TEXT-4", Some(0));
    r.game.players[1].battle_area[enemy.index as usize].is_suspended = true;
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    r.assert_prompt(SelectionKind::Target, true, 0);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[1].battle_area.is_empty());
    assert!(r.game.players[0].battle_area[target.index as usize].is_suspended);
    r.game.players[0].battle_area[target.index as usize].is_suspended = false;
    action(&mut r, 0, encode_attack(target.index as u16, 14));
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn dual_immediate_attack_can_digivolve_and_use_another_option() {
    let mut option = make_test_card("TEXT-OPTION", "Fixture Option");
    option.card_kind = digimon_engine::enums::CardKind::Option;
    option.level = None;
    option.dp = None;
    option.play_cost = 5;
    option.colors = vec![CardColor::Green];
    option.traits = vec!["Glowing Dawn".into()];
    let mut r = start(
        builder(CARD)
            .dsl_card("ST23-01")
            .unwrap()
            .dsl_card("ST23-08")
            .unwrap()
            .add_card(option)
            .hand(0, &[CARD, "ST23-08", "TEXT-OPTION"]),
    );
    let attacker = r.place_stack(0, &["ST23-01", "TEXT-4"]);
    stocked_tamer(&mut r, "TEXT-TAMER");
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    r.assert_prompt(SelectionKind::Target, true, 0);
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    assert!(r.game.pending_option.is_none());
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    r.choose_prompt_card("ST23-08", None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    r.choose_prompt_card("TEXT-OPTION", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 3);
    assert_eq!(r.security_count(1), 3);
    assert!(r.game.players[0].battle_area[attacker.index as usize].is_suspended);
    assert_eq!(
        r.game.players[0].battle_area[attacker.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "ST23-08"
    );
    assert!(ids(&r, &r.game.players[0].trash).contains(&"TEXT-OPTION".into()));
}

#[test]
fn dual_immediate_attack_can_accept_arts_then_digivolve_and_use_another_option() {
    for (effect_driven, first_effect) in [false, true]
        .into_iter()
        .flat_map(|effect_driven| (0..3).map(move |first_effect| (effect_driven, first_effect)))
    {
        let mut option = make_test_card("TEXT-OPTION", "Fixture Option");
        option.card_kind = digimon_engine::enums::CardKind::Option;
        option.level = None;
        option.dp = None;
        option.play_cost = 5;
        option.colors = vec![CardColor::Green];
        option.traits = vec!["Glowing Dawn".into()];
        let mut r = start(
            builder(CARD)
                .dsl_card("ST23-01")
                .unwrap()
                .dsl_card("ST23-08")
                .unwrap()
                .add_card(option)
                .add_card(digimon("TEXT-ARTS-HOST", 4, 4000, true))
                .add_card(digimon("TEXT-INITIAL-HOST", 4, 4000, true))
                .hand(
                    0,
                    if effect_driven {
                        &["ST23-08", CARD, "ST23-08", "TEXT-OPTION"]
                    } else {
                        &[CARD, "ST23-08", "TEXT-OPTION"]
                    },
                ),
        );
        let attacker = r.place_stack(0, &["ST23-01", "TEXT-4"]);
        let arts_host = r.place_on_field(0, "TEXT-ARTS-HOST", Some(0));
        if effect_driven {
            let tamer = r.place_stack(0, &["TEXT-BOTTOM", "TEXT-BOTTOM", "TEXT-TOP", "TEXT-TAMER"]);
            for c in &mut r.game.players[0].battle_area[tamer.index as usize].card_sources[..3] {
                c.face_down = true;
            }
            let initial_host = r.place_on_field(0, "TEXT-INITIAL-HOST", Some(0));
            action(&mut r, 0, encode_digivolve(0, initial_host.index as u16));
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            r.assert_prompt(SelectionKind::Hand, true, 0);
            r.choose_prompt_card(CARD, None).unwrap();
        } else {
            stocked_tamer(&mut r, "TEXT-TAMER");
            action(&mut r, 0, PLAY_HAND_START);
        }
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card("TEXT-4", None).unwrap();
        r.assert_prompt(SelectionKind::Target, true, 0);
        assert!(r.game.pending_option.is_some());
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert!(r.game.pending_option.is_some());
        assert_eq!(candidates(&r), ["TEXT-4", "TEXT-ARTS-HOST"]);
        r.choose_prompt_card("TEXT-ARTS-HOST", None).unwrap();
        assert!(r.game.pending_option.is_none());
        assert_eq!(
            r.game.players[0].battle_area[arts_host.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            CARD
        );
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        let choices = r
            .pending_selection()
            .unwrap()
            .effect_choices
            .as_ref()
            .unwrap();
        assert_eq!(choices.len(), 3);
        let inherited = choices
            .iter()
            .position(|c| c.label.starts_with("ST23-01 "))
            .unwrap();
        let arts_effects: Vec<_> = choices
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.label.starts_with("BT25-057 ").then_some(i))
            .collect();
        assert_eq!(arts_effects.len(), 2);
        if first_effect != 0 {
            r.execute_branch(arts_effects[first_effect - 1]).unwrap();
            if first_effect == 1 {
                r.assert_prompt(SelectionKind::Replacement, true, 0);
                r.decline_prompt().unwrap();
            }
            r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
            let inherited = r
                .pending_selection()
                .unwrap()
                .effect_choices
                .as_ref()
                .unwrap()
                .iter()
                .position(|c| c.label.starts_with("ST23-01 "))
                .unwrap();
            r.execute_branch(inherited).unwrap();
        } else {
            r.execute_branch(inherited).unwrap();
        }
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.accept_optional_trigger().unwrap();
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        r.assert_prompt(SelectionKind::Hand, true, 0);
        r.choose_prompt_card("ST23-08", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert!(r.game.pending_option.is_none());
        r.choose_prompt_card("TEXT-OPTION", None).unwrap();
        assert!(r.pending_selection().is_none());
        assert!(r.game.pending_option.is_none());
        assert_eq!(r.memory(), 3);
        assert_eq!(r.security_count(1), 3);
        assert_eq!(
            r.game.players[0].battle_area[attacker.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "ST23-08"
        );
        assert!(ids(&r, &r.game.players[0].trash).contains(&"TEXT-OPTION".into()));
        assert!(!ids(&r, &r.game.players[0].trash).contains(&CARD.into()));
    }
}

#[test]
fn arts_digivolve_may_evolve_a_legal_host_for_free_or_decline_to_trash_the_used_option() {
    for accept in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        let host = r.place_on_field(0, "TEXT-4", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card("TEXT-4", None).unwrap();
        r.assert_prompt(SelectionKind::Target, true, 0);
        r.decline_prompt().unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        if accept {
            r.choose_prompt_card("TEXT-4", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 6);
        assert_eq!(r.hand_size(0), usize::from(accept));
        assert_eq!(
            r.game.players[0].battle_area[host.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            if accept { CARD } else { "TEXT-4" }
        );
        assert_eq!(
            ids(&r, &r.game.players[0].trash).contains(&CARD.into()),
            !accept
        );
    }
}
