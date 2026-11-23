//! Source: `data/card_bundles/ST23-12.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles:
//! Purple Lv.2 / cost 0
//! Special digivolution condition:
//! [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0
//! Effect:
//! [On Play] By trashing the bottom face-down card from under any of your Tamers, you may return 1 Digimon card with the [Glowing Dawn] trait from your trash to the hand.
//! Inherited Effect:
//! ＜Retaliation＞ (When only this Digimon is deleted in battle, delete the Digimon it battled.)
//! Official Q&A:
//! Yes, you can return it.

use super::common::*;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "ST23-12";

#[test]
fn on_play_trashes_bottom_face_down_card_and_returns_only_glowing_dawn_digimon() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    for id in ["TEXT-3", "TEXT-5", "TEXT-OTHER", "TEXT-TAMER"] {
        r.inject_trash(0, id);
    }
    let t = stocked_tamer(&mut r, "TEXT-TAMER");
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-TAMER"]);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    r.assert_prompt(SelectionKind::Trash, true, 0);
    assert_eq!(candidates(&r), ["TEXT-3", "TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-5"]);
    assert!(ids(&r, &r.game.players[0].trash).contains(&"TEXT-BOTTOM".into()));
    assert_eq!(
        ids(
            &r,
            &r.game.players[0].battle_area[t.index as usize].card_sources
        ),
        ["TEXT-TOP", "TEXT-TAMER"]
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn on_play_may_decline_without_trashing_a_card() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.inject_trash(0, "TEXT-3");
    let t = stocked_tamer(&mut r, "TEXT-TAMER");
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.decline_prompt().unwrap();
    assert_eq!(
        r.game.players[0].battle_area[t.index as usize]
            .card_sources
            .len(),
        3
    );
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-3"]);
    assert!(r.pending_selection().is_none());
}

#[test]
fn on_play_cannot_return_without_a_face_down_card_under_a_tamer() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.inject_trash(0, "TEXT-3");
    r.place_stack(0, &["TEXT-BOTTOM", "TEXT-TAMER"]);
    action(&mut r, 0, PLAY_HAND_START);
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-3"]);
    assert!(r.game.players[0].hand.is_empty());
}

#[test]
fn official_qa_can_return_the_digimon_just_trashed_to_pay_the_cost() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let t = r.place_stack(0, &["TEXT-3", "TEXT-TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    r.assert_prompt(SelectionKind::Trash, true, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-3"]);
    assert!(r.game.players[0].trash.is_empty());
}

#[test]
fn inherited_retaliation_deletes_the_digimon_that_wins_the_battle() {
    let mut r = start(builder(CARD));
    let defender = r.place_stack(1, &[CARD, "TEXT-4"]);
    r.game.players[1].battle_area[defender.index as usize].is_suspended = true;
    let attacker = r.place_on_field(0, "TEXT-5", Some(0));
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, defender.index as u16),
    );
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-5"]);
}
