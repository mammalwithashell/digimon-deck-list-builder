//! BT26-103 Jupitermon: Wrath Mode — Lv.7 Yellow/Red/Black.
//!
//! <Piercing> <Reboot> <Blocker> <Succession ([Jupitermon])>
//! [When Digivolving] [Counter] [Once Per Turn] Trash your top security card
//! and <Recovery +2>.
//! [All Turns] [Once Per Turn] When security stacks are removed from, 1 of your
//! opponent's Digimon gets -15000 DP until their turn ends.
//! <Succession ([Jupitermon])>: G-ENGINE-SUCCESSION-KEYWORD.
//!
//! DCGO: BT26/Yellow/BT26_103.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-103";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-103")
        .dsl_card("BT26-033")
        .expect("BT26-033 Jupitermon")
        .add_card(filler("FILLER"))
        .add_card(tamer("TS-T", "Ts Tamer", CardColor::Yellow, &["TS"]))
        .add_card({
            let mut c = digimon("OPP", "Opp", CardColor::Red, 6, 9, &[]);
            c.dp = Some(20000);
            c
        })
        .add_card(digimon(
            "OLY",
            "Oly",
            CardColor::Yellow,
            6,
            9,
            &["Olympos XII"],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn drain(r: &mut DebugRunner) {
    while let Some(v) = r.pending_selection_view() {
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let _ = r.auto_resolve();
}

#[test]
fn bt26_103_keywords_and_alt_path() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let effs = format!("{:?}", card.effects);
    for kw in ["Piercing", "Reboot", "Blocker"] {
        assert!(effs.contains(kw), "missing {kw}");
    }
    assert!(format!("{:?}", card.alt_paths).contains("Olympos XII"));
}

#[test]
fn bt26_103_wd_trashes_top_security_then_recovers_two() {
    let mut r = setup();
    let j = r.place_stack(0, &["OLY", CARD_ID]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, j);
    drain(&mut r);
    assert_eq!(r.security_count(0), 4, "3 - 1 + 2");
    // The security removal fired the [All Turns] clause: -15000 on OPP.
    assert_eq!(r.effective_dp(opp), Some(5000), "20000 - 15000");
}

#[test]
fn bt26_103_wd_and_counter_share_once_per_turn() {
    let mut r = setup();
    let j = r.place_stack(0, &["OLY", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, j);
    drain(&mut r);
    assert_eq!(r.security_count(0), 4);
    fire(&mut r, EffectTiming::CounterEffect, j);
    drain(&mut r);
    assert_eq!(
        r.security_count(0),
        4,
        "[Once Per Turn] across WD and Counter"
    );
}

#[test]
fn bt26_103_opponent_security_removal_debuffs_until_their_turn_ends() {
    let mut r = setup();
    let j = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnOpponentSecurityRemoved, j);
    let v = r.pending_selection_view().expect("mandatory pick");
    assert!(!v.is_optional);
    drain(&mut r);
    assert!(r.effective_dp(opp) == Some(5000));
    // Once per turn across both security stacks.
    fire(&mut r, EffectTiming::OnOwnSecurityRemoved, j);
    assert!(r.game.pending_selection.is_none(), "once per turn");
    // Lasts through the end of my turn (until THEIR turn ends).
    r.end_turn();
    assert!(
        r.effective_dp(opp) == Some(5000),
        "still debuffed on their turn"
    );
}

// ─── <Succession ([Jupitermon])> ────────────────────────────────────────────

#[test]
fn bt26_103_succession_gains_jupitermon_keywords() {
    let mut r = setup();
    let w = r.place_stack(0, &["BT26-033", CARD_ID]);
    for kw in [Keyword::Raid, Keyword::Alliance, Keyword::Engage] {
        assert!(r.game.has_keyword(w, kw), "adopted {kw:?}");
    }
    // Its own keywords are still there.
    assert!(r.game.has_keyword(w, Keyword::Blocker));
}

#[test]
fn bt26_103_no_jupitermon_source_gains_nothing() {
    let mut r = setup();
    let w = r.place_stack(0, &["OLY", CARD_ID]);
    assert!(!r.game.has_keyword(w, Keyword::Raid));
    assert!(r.game.succession_source_indices(w).is_empty());
}

#[test]
fn bt26_103_succession_fires_adopted_when_digivolving() {
    let mut r = setup();
    let w = r.place_stack(0, &["BT26-033", CARD_ID]);
    let hand = r.game.players[0].hand.len();
    fire(&mut r, EffectTiming::WhenDigivolving, w);
    drain(&mut r);
    // Jupitermon's WD: top security to hand. Wrath Mode's own WD: trash top
    // security, <Recovery +2>.  3 - 1 - 1 + 2 = 3.
    assert_eq!(r.game.players[0].hand.len(), hand + 1, "adopted WD ran");
    assert_eq!(r.security_count(0), 3);
}

#[test]
fn bt26_103_succession_adopts_only_the_topmost_jupitermon() {
    let mut r = setup();
    let w = r.place_stack(0, &["BT26-033", "BT26-033", CARD_ID]);
    assert_eq!(r.game.succession_source_indices(w), vec![1], "topmost only");
    let hand = r.game.players[0].hand.len();
    fire(&mut r, EffectTiming::WhenDigivolving, w);
    drain(&mut r);
    assert_eq!(
        r.game.players[0].hand.len(),
        hand + 1,
        "adopted WD ran once"
    );
}

#[test]
fn bt26_103_jupitermon_on_top_does_not_adopt_itself() {
    // Succession only works from the card PRINTING it while it is the top card.
    let mut r = setup();
    let j = r.place_stack(0, &[CARD_ID, "BT26-033"]);
    assert!(r.game.succession_source_indices(j).is_empty());
    assert!(
        !r.game.has_keyword(j, Keyword::Blocker),
        "Wrath Mode is only a source"
    );
}

#[test]
fn bt26_103_succession_gains_jupitermon_leave_protection() {
    // Jupitermon's [All Turns] replacement, now Wrath Mode's: "by placing THIS
    // Digimon's top stacked card as the bottom security card" — this Digimon
    // is the carrier, so its top card is Wrath Mode itself.
    let mut r = setup();
    let w = r.place_stack(0, &["BT26-033", CARD_ID]);
    let t = r.place_on_field(0, "TS-T", Some(0));
    r.game.delete_permanents_batch(
        vec![t],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let v = r
        .pending_selection_view()
        .expect("adopted replacement prompt");
    assert!(v.is_optional);
    let accept = non_pass(&r)[0];
    r.execute_action(0, accept).unwrap();
    let _ = r.auto_resolve();
    let ids = field_ids(&r, 0);
    assert!(ids.contains(&"TS-T".to_string()), "tamer stayed: {ids:?}");
    assert!(
        ids.contains(&"BT26-033".to_string()),
        "carrier de-stacked to Jupitermon"
    );
    let bottom = &r.game.players[0].security[0];
    assert_eq!(bottom.card_id(&r.game.card_data), CARD_ID);
    let _ = w;
}
