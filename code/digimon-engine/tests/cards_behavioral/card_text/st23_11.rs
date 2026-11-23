//! Source: `data/card_bundles/ST23-11.md` (printed text).
//!
//! Standard digivolve cost circles:
//! Black Lv.3 / cost 2
//! Special digivolution condition:
//! [Digivolve] Lv.3 w/[Glowing Dawn] trait: Cost 2
//! Effect:
//! ＜Blocker＞ [Your Turn] When this Digimon would digivolve into a [Glowing Dawn] trait Digimon card, by trashing the bottom face-down card from under any of your Tamers, reduce the cost by 2.
//! Inherited Effect:
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)

use super::common::*;
use digimon_engine::action::space::{encode_attack, encode_digivolve};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "ST23-11";

#[test]
fn your_turn_digivolve_glowing_dawn_reduces_cost_by_two_only_if_paid() {
    for (stocked, accept, target, paid) in [
        (true, true, "TEXT-5", 2),
        (true, false, "TEXT-5", 4),
        (false, false, "TEXT-5", 4),
        (true, false, "TEXT-OTHER", 4),
    ] {
        let mut r = start(builder(CARD).hand(0, &[target]));
        let base = r.place_on_field(0, CARD, Some(0));
        if stocked {
            stocked_tamer(&mut r, "TEXT-TAMER");
        }
        action(&mut r, 0, encode_digivolve(0, base.index as u16));
        if stocked && target == "TEXT-5" {
            r.assert_prompt(SelectionKind::EffectChoice, true, 0);
            if accept {
                r.execute_branch(0).unwrap();
                r.assert_prompt(SelectionKind::OwnField, false, 0);
                assert_eq!(candidates(&r), ["TEXT-TAMER"]);
                r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            } else {
                r.decline_prompt().unwrap();
            }
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            10 - r.memory(),
            paid,
            "stocked={stocked}, accept={accept}, target={target}"
        );
        assert_eq!(
            ids(&r, &r.game.players[0].trash),
            if accept { vec!["TEXT-BOTTOM"] } else { vec![] }
        );
        assert_eq!(
            r.game.players[0].battle_area[base.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            target
        );
    }
}

#[test]
fn blocker_and_inherited_blocker_can_redirect_an_attack() {
    for inherited in [false, true] {
        let mut r = start(builder(CARD));
        let blocker = if inherited {
            r.place_stack(1, &[CARD, "TEXT-6"])
        } else {
            r.place_on_field(1, CARD, Some(0))
        };
        assert!(r.game.has_keyword(blocker, Keyword::Blocker));
        let attacker = r.place_on_field(0, "TEXT-3", Some(0));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::OwnField, true, 1);
        assert_eq!(candidates(&r), [if inherited { "TEXT-6" } else { CARD }]);
        r.choose_prompt_card(if inherited { "TEXT-6" } else { CARD }, None)
            .unwrap();
        assert_eq!(r.security_count(1), 5);
        assert!(r.game.players[1].battle_area[blocker.index as usize].is_suspended);
        assert!(r.game.players[0].battle_area.is_empty());
    }
}

#[test]
fn special_digivolve_from_glowing_dawn_level_three_costs_two() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let base = r.place_on_field(0, "TEXT-3", Some(0));
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(r.memory(), 8);
    assert!(r.pending_selection().is_none());
}
