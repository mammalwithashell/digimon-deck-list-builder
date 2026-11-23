//! Source: `data/card_bundles/ST23-15.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles:
//! (none — DigiEgg / Lv.2 base, or DNA/special only)
//! Effect:
//! ＜Use Req. ([BEATBREAK] trait)＞ (Specified cards let you ignore color requirements.) [Main] You may play 1 [BEATBREAK] trait card with a play cost of 4 or less from your hand or trash without paying the cost. Then, place this card in the battle area. [Start of Your Main Phase] By placing this card from the battle area face down under any of your [BEATBREAK] trait Tamers, ＜Draw 1＞ and gain 1 memory.
//! Security Effect:
//! [Security] Activate this card's [Main] effects.
//! Official Q&A:
//! The card is placed on the bottom of the cards under the Tamer.

use super::common::*;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

const CARD: &str = "ST23-15";

#[test]
fn use_requirement_beatbreak_ignores_white_color_requirement_but_unrelated_trait_does_not() {
    for (support, allowed) in [("TEXT-3", true), ("TEXT-OTHER", false)] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        r.place_on_field(0, support, Some(0));
        assert_eq!(build_action_mask(&r.game, 0)[0] > 0.5, allowed);
    }
}

#[test]
fn main_may_play_beatbreak_cost_four_or_less_from_hand_or_trash_free_then_places_itself() {
    for from_trash in [false, true] {
        let mut expensive = digimon("TEXT-EXPENSIVE", 5, 7000, true);
        expensive.play_cost = 5;
        let mut r = start(
            builder(CARD)
                .add_card(expensive)
                .hand(0, &[CARD, "TEXT-3", "TEXT-OTHER", "TEXT-EXPENSIVE"]),
        );
        r.place_on_field(0, "TEXT-TAMER", Some(0));
        r.inject_trash(0, "TEXT-TAMER");
        r.inject_trash(0, "TEXT-OTHER");
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(
            SelectionKind::UnionZone {
                zones: UnionZoneSet::HAND | UnionZoneSet::TRASH,
            },
            true,
            0,
        );
        assert_eq!(candidates(&r), ["TEXT-3", "TEXT-TAMER"]);
        r.choose_prompt_card(if from_trash { "TEXT-TAMER" } else { "TEXT-3" }, None)
            .unwrap();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 7);
        assert_eq!(r.game.players[0].battle_area.len(), 3);
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == CARD));
        assert!(!ids(&r, &r.game.players[0].trash).contains(&CARD.into()));
    }
}

#[test]
fn main_declining_or_having_no_play_target_still_places_this_card_in_battle_area() {
    for target in [false, true] {
        let hand: &[&str] = if target { &[CARD, "TEXT-3"] } else { &[CARD] };
        let mut r = start(builder(CARD).hand(0, hand));
        r.place_on_field(0, "TEXT-TAMER", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        if target {
            r.assert_prompt(
                SelectionKind::UnionZone {
                    zones: UnionZoneSet::HAND | UnionZoneSet::TRASH,
                },
                true,
                0,
            );
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 7);
        assert_eq!(r.game.players[0].battle_area.len(), 2);
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == CARD));
    }
}

#[test]
fn start_of_your_main_phase_may_place_itself_face_down_at_bottom_under_only_beatbreak_tamer_to_draw_and_gain_memory(
) {
    for accept in [false, true] {
        let mut unrelated = make_test_card("TEXT-UNRELATED-TAMER", "Unrelated Tamer");
        unrelated.card_kind = CardKind::Tamer;
        unrelated.colors = vec![CardColor::White];
        let mut r = start(builder(CARD).add_card(unrelated).hand(0, &[CARD]));
        let t = stocked_tamer(&mut r, "TEXT-TAMER");
        r.place_on_field(0, "TEXT-3", Some(0));
        r.place_on_field(0, "TEXT-UNRELATED-TAMER", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        assert!(r.pending_selection().is_none());
        next_turn(&mut r);
        let hand_before = r.hand_size(0);
        next_turn(&mut r);
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-TAMER"]);
        if accept {
            r.choose_prompt_card("TEXT-TAMER", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), if accept { 4 } else { 3 });
        assert_eq!(r.hand_size(0) - hand_before, if accept { 2 } else { 1 });
        let sources = &r.game.players[0].battle_area[t.index as usize].card_sources;
        assert_eq!(sources.len(), if accept { 4 } else { 3 });
        if accept {
            assert_eq!(
                ids(&r, sources),
                [CARD, "TEXT-BOTTOM", "TEXT-TOP", "TEXT-TAMER"]
            );
            assert!(sources[0].face_down);
        }
    }
}

#[test]
fn security_activates_main_for_free_and_places_this_card_in_battle_area() {
    let mut r = start(builder(CARD).security(1, &[CARD]).hand(1, &["TEXT-3"]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.assert_prompt(
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH,
        },
        true,
        1,
    );
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(r.memory(), 10);
    assert_eq!(r.game.players[1].battle_area.len(), 2);
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == CARD));
    assert!(r.pending_selection().is_none());
}
