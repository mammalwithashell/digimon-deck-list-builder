//! BT26-014 Darumamon — Lv.5 Red/Yellow (Mutant / Shambala / TB).
//!
//! [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 7000
//! DP or less. [On Deletion] You may return 1 [Shambala] trait card from your
//! trash to the hand. Then, you may play 1 [TB] trait Digimon card with 6000
//! DP or less from your hand without paying the cost.
//! Assembly -2: Lv.4 or lower [TB] trait card.
//! Inherited: [On Deletion] You may play 1 [TB] trait Digimon card with 6000
//! DP or less from your hand without paying the cost.
//!
//! DCGO: BT26/Red/BT26_014.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-014";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-014")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAMT",
            "ShamTrash",
            CardColor::Red,
            3,
            3,
            &["Shambala"],
        ))
        // Lv.4 TB → DP 4000 (eligible); Lv.6 TB → DP 6000 (eligible boundary);
        // Lv.7 TB → DP 7000 (too big).
        .add_card(digimon("TB4", "TB4", CardColor::Red, 4, 5, &["TB"]))
        .add_card(digimon("TB6", "TB6", CardColor::Red, 6, 12, &["TB"]))
        .add_card(digimon("TB7", "TB7", CardColor::Red, 7, 13, &["TB"]))
        .add_card(tamer("TBTAMER", "TB Tamer", CardColor::Red, &["TB"]))
        .add_card(digimon("PLAIN4", "Plain4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("OPP7", "Opp7", CardColor::Blue, 7, 13, &[]))
        .add_card(digimon("OPP8", "Opp8", CardColor::Blue, 8, 14, &[]))
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
fn bt26_014_alt_path_and_assembly() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Shambala") && alt.contains("level_eq: Some(4)"),
        "{alt}"
    );
    assert!(alt.contains("Assembly") && alt.contains("TB"), "{alt}");
}

#[test]
fn bt26_014_on_play_deletes_opp_digimon_with_7000_or_less() {
    let mut r = setup();
    r.place_on_field(1, "OPP7", Some(0));
    r.place_on_field(1, "OPP8", Some(0));
    let d = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, d);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only OPP7");
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 1), vec!["OPP8"]);
}

#[test]
fn bt26_014_when_digivolving_no_target_no_prompt() {
    let mut r = setup();
    r.place_on_field(1, "OPP8", Some(0));
    let d = r.place_stack(0, &["PLAIN4", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, d);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP8"]);
}

#[test]
fn bt26_014_on_deletion_returns_shambala_then_plays_tb_free() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAMT");
    push_hand(&mut r, 0, "TB6");
    push_hand(&mut r, 0, "TB7");
    push_hand(&mut r, 0, "TBTAMER");
    let d = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![d], ReplacementCause::OpponentEffect);
    // (a) return a [Shambala] card — Darumamon itself is also eligible now.
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "SHAMT + Darumamon itself");
    pick_trash(&mut r, 0, "SHAMT");
    assert!(hand_ids(&r, 0).contains(&"SHAMT".to_string()));
    // (b) play a [TB] Digimon with <= 6000 DP: only TB6 (not TB7, not the Tamer,
    // not SHAMT).
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "{:?}", hand_ids(&r, 0));
    pick_hand(&mut r, 0, "TB6");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TB6"]);
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt26_014_on_deletion_decline_return_still_offers_play() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAMT");
    push_hand(&mut r, 0, "TB4");
    let d = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanents_batch(vec![d], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    assert!(trash_ids(&r, 0).contains(&"SHAMT".to_string()));
    pick_hand(&mut r, 0, "TB4");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TB4"]);
}

#[test]
fn bt26_014_inherited_on_deletion_plays_tb_free() {
    let mut r = setup();
    push_hand(&mut r, 0, "TB4");
    push_hand(&mut r, 0, "PLAIN4");
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    r.game
        .delete_permanents_batch(vec![c], ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("inherited play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1);
    pick_hand(&mut r, 0, "TB4");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TB4"]);
}

#[test]
fn bt26_014_inherited_may_decline() {
    let mut r = setup();
    push_hand(&mut r, 0, "TB4");
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    r.game
        .delete_permanents_batch(vec![c], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec!["TB4"]);
}
