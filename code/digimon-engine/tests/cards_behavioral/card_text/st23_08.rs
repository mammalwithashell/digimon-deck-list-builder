//! Source: `data/card_bundles/ST23-08.md` (printed text).
//!
//! Standard digivolve cost circles:
//! Green Lv.4 / cost 3
//! Special digivolution condition:
//! [Digivolve] Lv.4 w/[Glowing Dawn] trait: Cost 3
//! Effect:
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other Digimon, this Digimon gains ＜Security A. +1＞ and the suspended Digimon's DP for the attack.) [On Play] [When Digivolving] This Digimon gets +3000 DP until your opponent's turn ends. Then, if it's your turn, by trashing the bottom face-down card from under any of your Tamers, you may play or use 1 [Glowing Dawn] trait card from your hand with the cost reduced by 3.
//! Inherited Effect:
//! [End of Attack] [Once Per Turn] By trashing the bottom face-down card from under any of your Tamers, this [Glowing Dawn] trait Digimon unsuspends.

use super::common::*;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "ST23-08";

#[test]
fn on_play_and_when_digivolving_gain_three_thousand_dp_until_opponents_turn_ends_without_cost() {
    for evolve in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        let index = if evolve {
            let base = r.place_on_field(0, "TEXT-4", Some(0));
            action(&mut r, 0, encode_digivolve(0, base.index as u16));
            base
        } else {
            action(&mut r, 0, PLAY_HAND_START);
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 0,
            }
        };
        assert!(r.pending_selection().is_none());
        assert_eq!(r.dp_of(index), Some(10000));
        advance_turn(&mut r);
        assert_eq!(r.dp_of(index), Some(10000));
        advance_turn(&mut r);
        assert_eq!(r.dp_of(index), Some(7000));
    }
}

#[test]
fn on_play_and_when_digivolving_may_pay_bottom_face_down_card_to_play_glowing_dawn_three_cheaper() {
    for evolve in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD, "TEXT-3", "TEXT-OTHER"]));
        stocked_tamer(&mut r, "TEXT-TAMER");
        if evolve {
            let base = r.place_on_field(0, "TEXT-4", Some(0));
            action(&mut r, 0, encode_digivolve(0, base.index as u16));
        } else {
            action(&mut r, 0, PLAY_HAND_START);
        }
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-TAMER"]);
        r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), ["TEXT-3"]);
        r.choose_prompt_card("TEXT-3", None).unwrap();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), if evolve { 7 } else { 3 });
        assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "TEXT-3"));
    }
}

#[test]
fn on_play_and_when_digivolving_may_use_glowing_dawn_option_three_cheaper_or_decline_after_paying()
{
    for evolve in [false, true] {
        for use_option in [false, true] {
            let mut option = make_test_card("TEXT-OPTION", "Fixture Option");
            option.card_kind = CardKind::Option;
            option.level = None;
            option.dp = None;
            option.play_cost = 5;
            option.colors = vec![CardColor::Green];
            option.traits = vec!["Glowing Dawn".into()];
            let mut r = start(
                builder(CARD)
                    .add_card(option)
                    .hand(0, &[CARD, "TEXT-OPTION"]),
            );
            stocked_tamer(&mut r, "TEXT-TAMER");
            if evolve {
                let base = r.place_on_field(0, "TEXT-4", Some(0));
                action(&mut r, 0, encode_digivolve(0, base.index as u16));
            } else {
                action(&mut r, 0, PLAY_HAND_START);
            }
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            r.assert_prompt(SelectionKind::Hand, true, 0);
            assert_eq!(candidates(&r), ["TEXT-OPTION"]);
            if use_option {
                r.choose_prompt_card("TEXT-OPTION", None).unwrap();
            } else {
                r.decline_prompt().unwrap();
            }
            assert!(r.pending_selection().is_none());
            assert_eq!(
                r.memory(),
                (if evolve { 7 } else { 3 }) - if use_option { 2 } else { 0 }
            );
            assert_eq!(
                ids(&r, &r.game.players[0].trash),
                if use_option {
                    vec!["TEXT-BOTTOM", "TEXT-OPTION"]
                } else {
                    vec!["TEXT-BOTTOM"]
                }
            );
        }
    }
}

#[test]
fn on_play_may_decline_cost_but_still_gets_dp() {
    let mut r = start(builder(CARD).hand(0, &[CARD, "TEXT-3"]));
    stocked_tamer(&mut r, "TEXT-TAMER");
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 3);
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(
        r.dp_of(digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 1
        }),
        Some(10000)
    );
}

#[test]
fn alliance_suspends_another_digimon_for_extra_dp_and_security_check_only_for_the_attack() {
    for accept in [false, true] {
        let mut r = start(builder(CARD));
        let attacker = r.place_on_field(0, CARD, Some(0));
        let ally = r.place_on_field(0, "TEXT-4", Some(0));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        if accept {
            r.choose_prompt_card("TEXT-4", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.security_count(1), if accept { 3 } else { 4 });
        assert_eq!(
            r.game.players[0].battle_area[ally.index as usize].is_suspended,
            accept
        );
        assert_eq!(r.dp_of(attacker), Some(7000));
    }
}

#[test]
fn inherited_end_of_attack_pays_bottom_face_down_card_to_unsuspend_only_once_per_turn() {
    let mut r = start(builder(CARD));
    let attacker = r.place_stack(0, &[CARD, "TEXT-6"]);
    stocked_tamer(&mut r, "TEXT-TAMER");
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    assert!(!r.game.players[0].battle_area[attacker.index as usize].is_suspended);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[attacker.index as usize].is_suspended);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
}
