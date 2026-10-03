//! BT26-013 Musyamon — Lv.4 Red/Purple (Wizard / Shambala / TB / TS).
//!
//! <Blocker> [On Play] [On Deletion] By trashing 1 card in your hand, delete
//! 1 of your opponent's Digimon with 6000 DP or less.
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! DCGO: BT26/Purple/BT26_013.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-013";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-013")
        .add_card(filler("FILLER"))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPP6", "Opp6", CardColor::Blue, 6, 12, &[]))
        .add_card(digimon("OPP7", "Opp7", CardColor::Blue, 7, 13, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_013_blocker_and_alt_path() {
    let mut r = setup();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(m, Keyword::Blocker));
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Shambala") && alt.contains("TS") && alt.contains("level_eq: Some(3)"),
        "{alt}"
    );
}

#[test]
fn bt26_013_on_play_trash_hand_card_deletes_6000_or_less() {
    let mut r = setup();
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP6", Some(0));
    r.place_on_field(1, "OPP7", Some(0));
    let m = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, m);
    let v = r.pending_selection_view().expect("trash cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "JUNK");
    assert_eq!(trash_ids(&r, 0), vec!["JUNK"]);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only OPP6");
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 1), vec!["OPP7"]);
}

#[test]
fn bt26_013_declining_cost_deletes_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP6", Some(0));
    let m = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, m);
    pass(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0), vec!["JUNK"]);
    assert_eq!(field_ids(&r, 1), vec!["OPP6"]);
}

#[test]
fn bt26_013_empty_hand_no_prompt() {
    let mut r = setup();
    r.place_on_field(1, "OPP6", Some(0));
    let m = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, m);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP6"]);
}

#[test]
fn bt26_013_on_deletion_trash_hand_card_deletes() {
    let mut r = setup();
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP6", Some(0));
    let m = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanents_batch(vec![m], ReplacementCause::OpponentEffect);
    pick_hand(&mut r, 0, "JUNK");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert!(trash_ids(&r, 0).contains(&"JUNK".to_string()));
}

#[test]
fn bt26_013_inherited_plus_2000_your_turn_only() {
    let mut r = setup();
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    assert_eq!(r.effective_dp(c), Some(5000 + 2000));
    r.end_turn();
    assert_eq!(r.effective_dp(c), Some(5000));
}
