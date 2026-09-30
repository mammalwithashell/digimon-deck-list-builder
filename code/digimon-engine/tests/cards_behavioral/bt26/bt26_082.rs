//! BT26-082 Ravemon — Lv.6 Purple, Cyborg/DATA SQUAD/(Rule) Birdkin.
//!
//! {Security} [End of Opponent's Turn] Play this card without paying the cost.
//! [When Digivolving] [End of Attack] By deleting this Digimon or trashing 2
//! bottom face-down cards from under any of your Tamers, delete 1 of your
//! opponent's highest DP Digimon.
//! [On Deletion] Your opponent trashes 1 card in their hand. Then, if their
//! hand has 7 or fewer cards, you may place this card face up as the bottom
//! security card.
//!
//! DCGO: BT26/Purple/BT26_082.cs. Official Q&A: {Security} works only while the
//! card is FACE UP in the security stack (rule 15-14-5).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-082";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-082")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon("BIG", "Big", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("SMALL", "Small", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
}

fn setup() -> DebugRunner {
    let mut r = builder()
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_082_has_birdkin_rule_trait() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(format!("{:?}", card.traits).contains("Birdkin"));
}

#[test]
fn bt26_082_wd_delete_self_deletes_highest_dp() {
    let mut r = setup();
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rave);
    // No Tamer face-down cards ⇒ the 2-way modal {Delete this | Don't pay}.
    r.execute_branch(0).expect("delete this Digimon");
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()), "Ravemon deleted as the cost");
    assert_eq!(field_ids(&r, 1), vec!["SMALL".to_string()], "the highest-DP Digimon was deleted");
}

#[test]
fn bt26_082_wd_trash_two_face_down_keeps_ravemon() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rave);
    r.execute_branch(1).expect("trash 2 face-down cards");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0, "2 face-down cards trashed");
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(field_ids(&r, 1), vec!["SMALL".to_string()]);
}

#[test]
fn bt26_082_wd_dont_pay() {
    let mut r = setup();
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rave);
    r.execute_branch(1).expect("don't pay (2-way modal)");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(field_ids(&r, 1), vec!["BIG".to_string()]);
}

#[test]
fn bt26_082_on_deletion_discard_then_face_up_bottom_security() {
    let mut r = setup();
    push_hand(&mut r, 1, "JUNK");
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    let sec_before = r.security_count(0);
    // Delete it for real so the card is in the trash when [On Deletion] runs.
    r.game.delete_permanents_batch(vec![rave], digimon_engine::replacement::ReplacementCause::OpponentEffect);
    pick_hand(&mut r, 1, "JUNK");
    r.execute_branch(0).expect("place face up as bottom security");

    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec_before + 1);
    let bottom = &r.game.players[0].security[0];
    assert_eq!(bottom.card_id(&r.game.card_data), CARD_ID, "placed at the bottom");
    assert!(r.game.players[0].face_up_security.contains(&bottom.card_index), "face up");
}

#[test]
fn bt26_082_face_up_security_plays_itself_at_end_of_opponents_turn() {
    let mut r = builder()
        .security(0, &[CARD_ID, "FILLER"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .start();
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    r.end_turn();
    assert!(r.game.players[0].battle_area.is_empty(), "waits for the OPPONENT's turn end");
    r.end_turn();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.security_count(0), 1);
}

#[test]
fn bt26_082_face_down_security_does_nothing() {
    let mut r = builder()
        .security(0, &[CARD_ID, "FILLER"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .start();
    r.end_turn();
    r.end_turn();
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()), "{{Security}} needs face up (15-14-5)");
    assert_eq!(r.security_count(0), 2);
}
