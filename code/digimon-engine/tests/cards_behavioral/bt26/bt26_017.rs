//! BT26-017 Zanbamon — Lv.6 Red/Purple (Wizard / Shambala / TB / TS).
//!
//! <Blocker> <Retaliation> [On Play] [When Digivolving] 1 of your Digimon with
//! the [Shambala] trait gains <Security A. +1> and <Progress> for the turn.
//! [On Deletion] You may play 1 [Shambala] or [TS] trait card with a play cost
//! of 5 or less from your trash without paying the cost.
//! Assembly -4: 2 Lv.5 or lower [Shambala] trait cards w/different levels.
//!
//! DCGO: BT26/Purple/BT26_017.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-017";
const OWN: u16 = digimon_engine::action::space::ATTACK_START;

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-017")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAM3",
            "Sham3",
            CardColor::Red,
            3,
            3,
            &["Shambala"],
        ))
        .add_card(digimon(
            "SHAM7",
            "Sham7",
            CardColor::Red,
            5,
            7,
            &["Shambala"],
        ))
        .add_card(digimon("TSONLY", "TsOnly", CardColor::Red, 4, 5, &["TS"]))
        .add_card(tamer("TSTAMER", "TS Tamer", CardColor::Red, &["TS"]))
        .add_card(digimon("PLAIN3", "Plain3", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("HOST5", "Host5", CardColor::Red, 5, 7, &["TS"]))
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
fn bt26_017_keywords_alt_path_and_assembly() {
    let mut r = setup();
    let z = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(z, Keyword::Blocker));
    assert!(r.game.has_keyword(z, Keyword::Retaliation));
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("level_eq: Some(5)") && alt.contains("TS") && alt.contains("Shambala"),
        "{alt}"
    );
    assert!(alt.contains("Assembly"), "{alt}");
    assert!(
        alt.contains("distinct_by: Some"),
        "distinct-level materials: {alt}"
    );
}

#[test]
fn bt26_017_on_play_shambala_digimon_gains_sa1_and_progress() {
    let mut r = setup();
    let ts = r.place_on_field(0, "TSONLY", Some(0));
    let sham = r.place_on_field(0, "SHAM3", Some(0));
    let z = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, z);
    let v = r.pending_selection_view().expect("target pick");
    assert!(!v.is_optional);
    assert!(
        !v.valid_action_ids.contains(&(OWN + ts.index as u16)),
        "[TS]-only is not [Shambala]"
    );
    assert!(v.valid_action_ids.contains(&(OWN + z.index as u16)));
    r.execute_action(0, OWN + sham.index as u16).unwrap();
    assert!(r.game.has_keyword(sham, Keyword::Progress));
    assert_eq!(r.game.security_attack_keyword_bonus(sham), 1);
    assert!(!r.game.has_keyword(z, Keyword::Progress));
    r.end_turn();
    assert!(!r.game.has_keyword(sham, Keyword::Progress), "for the turn");
    assert_eq!(r.game.security_attack_keyword_bonus(sham), 0);
}

#[test]
fn bt26_017_when_digivolving_can_target_itself() {
    let mut r = setup();
    let z = r.place_stack(0, &["HOST5", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, z);
    pick_first(&mut r, 0);
    assert!(r.game.has_keyword(z, Keyword::Progress));
    assert_eq!(r.game.security_attack_keyword_bonus(z), 1);
}

#[test]
fn bt26_017_on_deletion_plays_shambala_or_ts_cost_5_or_less_from_trash() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAM7"); // cost 7 — not eligible
    push_trash(&mut r, 0, "PLAIN3"); // no trait — not eligible
    push_trash(&mut r, 0, "TSTAMER");
    push_trash(&mut r, 0, "SHAM3");
    let z = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![z], ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("trash play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "TSTAMER + SHAM3");
    pick_trash(&mut r, 0, "TSTAMER");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TSTAMER"]);
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt26_017_on_deletion_may_decline() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAM3");
    let z = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanents_batch(vec![z], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 0).contains(&"SHAM3".to_string()));
}
