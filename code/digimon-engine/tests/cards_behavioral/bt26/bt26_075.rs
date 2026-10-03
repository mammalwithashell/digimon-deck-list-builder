//! BT26-075 ScourgeChiropmon // Despair Blast — DUAL (Digimon Lv.5
//! Purple/Yellow DP 8000 // Option Purple use cost 4).
//! Traits: Machine, Glowing Dawn, BEATBREAK.
//! Digivolve: Purple Lv.4 / 4; Yellow Lv.4 / 4; [Digivolve] Lv.4 w/[Glowing Dawn]: 3.
//!
//! <Execute> <Ascension> [Security] [On Deletion] By trashing the bottom
//! face-down card from under any of your Tamers, you may play 1 [Glowing Dawn]
//! trait card with a play cost of 5 or less from your trash without paying the
//! cost.
//! Option: <Use Req. ([Glowing Dawn] trait)> [Main] Delete 1 of your
//! opponent's Digimon with the lowest level. <Arts Digivolve>.
//! Official Q&A: <Ascension> and [On Deletion] trigger simultaneously; the
//! player orders them (Ascension first ⇒ the [On Deletion] is lost).
//!
//! DCGO: BT26/Purple/BT26_075.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-075";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-075")
        .add_card(filler("FILLER"))
        .add_card(tamer("SECT", "SecTamer", CardColor::Red, &[]))
        .add_card(tamer(
            "GDT",
            "GD Tamer",
            CardColor::Purple,
            &["Glowing Dawn"],
        ))
        .add_card(digimon(
            "GD5",
            "GD5",
            CardColor::Purple,
            4,
            5,
            &["Glowing Dawn"],
        ))
        .add_card(digimon(
            "GD6",
            "GD6",
            CardColor::Purple,
            5,
            6,
            &["Glowing Dawn"],
        ))
        .add_card(digimon("P5", "P5", CardColor::Purple, 4, 5, &[]))
        .add_card(digimon("OPP3", "Opp3", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .add_card({
            let mut d = digimon("ATK", "Atk", CardColor::Red, 3, 3, &[]);
            d.dp = Some(1000);
            d
        })
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SECT", "SECT", "SECT"])
        .security(1, &["SECT", "SECT", "SECT"])
        .memory(4)
        .start();
    r.set_first_player(0);
    r
}

fn delete(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .delete_permanents_batch(vec![h], ReplacementCause::OpponentEffect);
}

/// Decline <Ascension>'s "place this card as the top security card?" choice
/// if it is the pending prompt. (The engine's inline drain of the
/// face-down-trash observers can surface it mid-[On Deletion]; see
/// G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS.)
fn decline_ascension_if_pending(r: &mut DebugRunner) {
    let is_ascension = r
        .game
        .pending_selection
        .as_ref()
        .is_some_and(|s| s.prompt.contains("top security card") && s.effect_choices.is_some());
    if is_ascension {
        r.execute_branch(1).expect("decline <Ascension>");
    }
}

/// Resolve the <Ascension> / [On Deletion] trigger-order prompt, putting the
/// [On Deletion] (slot 0) first.
fn on_deletion_first(r: &mut DebugRunner) {
    let (p, a) = {
        let s = r.game.pending_selection.as_ref().expect("order prompt");
        assert!(s.valid_action_ids.len() >= 2, "both triggers offered");
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(p, a).unwrap();
}

#[test]
fn bt26_075_has_execute_and_ascension() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Execute));
    let dbg = format!("{:?}", r.compiled_card(CARD_ID).unwrap().effects);
    assert!(dbg.contains("Ascension"));
}

#[test]
fn bt26_075_on_deletion_trashes_face_down_and_plays_from_trash() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    push_trash(&mut r, 0, "GD5");
    push_trash(&mut r, 0, "GD6");
    push_trash(&mut r, 0, "P5");
    let s = r.place_on_field(0, CARD_ID, Some(0));
    delete(&mut r, s);
    on_deletion_first(&mut r);
    pick_first(&mut r, 0); // the Tamer (the 'by trashing' cost)
    assert_eq!(sources(&r, t), 0);
    decline_ascension_if_pending(&mut r);
    let v = r.pending_selection_view().expect("trash play pick");
    assert!(v.is_optional, "'you may play'");
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        1,
        "only the [Glowing Dawn] card with play cost <= 5"
    );
    pick_first(&mut r, 0);
    decline_ascension_if_pending(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert!(field_ids(&r, 0).contains(&"GD5".to_string()));
    assert!(
        trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "Ascension declined"
    );
    assert_eq!(r.memory(), 4, "without paying the cost");
}

#[test]
fn bt26_075_on_deletion_cost_may_be_declined() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    push_trash(&mut r, 0, "GD5");
    let s = r.place_on_field(0, CARD_ID, Some(0));
    delete(&mut r, s);
    on_deletion_first(&mut r);
    let v = r.pending_selection_view().expect("Tamer pick");
    assert!(v.is_optional, "the 'by trashing' cost is declinable");
    pass(&mut r, 0);
    decline_ascension_if_pending(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, t), 1);
    assert!(!field_ids(&r, 0).contains(&"GD5".to_string()));
}

#[test]
fn bt26_075_on_deletion_needs_a_face_down_card() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    push_trash(&mut r, 0, "GD5");
    let s = r.place_on_field(0, CARD_ID, Some(0));
    delete(&mut r, s);
    // Only <Ascension> is live (no trigger-order prompt): decline it.
    decline_ascension_if_pending(&mut r);
    assert!(
        r.game.pending_selection.is_none(),
        "no [On Deletion] prompt"
    );
    assert!(!field_ids(&r, 0).contains(&"GD5".to_string()));
}

#[test]
fn bt26_075_security_effect_plays_from_trash() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    push_trash(&mut r, 0, "GD5");
    r.game.players[0].security.clear();
    {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0]
            .security
            .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    }
    r.pass_turn(); // → P1's turn
    let atk = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(atk, 0, false);
    pick_first(&mut r, 0); // the Tamer
    pick_first(&mut r, 0); // GD5 from trash
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0);
    assert!(field_ids(&r, 0).contains(&"GD5".to_string()));
}

// ─── Option face ────────────────────────────────────────────────────────────

#[test]
fn bt26_075_option_deletes_lowest_level_opponent_digimon() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0)); // [Glowing Dawn] use requirement
    r.place_on_field(1, "OPP5", Some(0));
    r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        1,
        "only the lowest level"
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}

#[test]
fn bt26_075_option_ties_offer_every_lowest() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
    let v = r.pending_selection_view().expect("delete pick");
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        2
    );
}

#[test]
fn bt26_075_dual_metadata() {
    let r = setup();
    let dbg = format!("{:?}", r.compiled_card(CARD_ID).unwrap());
    assert!(dbg.contains("ArtsDigivolve"));
}

#[test]
fn bt26_075_ascension_first_loses_the_on_deletion() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    push_trash(&mut r, 0, "GD5");
    let s = r.place_on_field(0, CARD_ID, Some(0));
    delete(&mut r, s);
    let (p, a) = {
        let s = r.game.pending_selection.as_ref().expect("order prompt");
        (s.selecting_player, s.valid_action_ids[1])
    };
    r.execute_action(p, a).unwrap(); // <Ascension> first
    r.execute_branch(0).expect("place as top security");
    let _ = r.auto_resolve();
    assert_eq!(
        security_ids(&r, 0).last().map(String::as_str),
        Some(CARD_ID)
    );
    assert_eq!(
        sources(&r, t),
        1,
        "Q&A: the [On Deletion] can no longer activate"
    );
    assert!(!field_ids(&r, 0).contains(&"GD5".to_string()));
}
