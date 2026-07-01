//! ST24-07 ShineGreymon / "GeoGrey Sword" — DUAL card (Digimon + Option).
//! Lv.6 Yellow, Light Dragon / DATA SQUAD, DP 12000.
//!
//! # cards.json TRAP
//! cards.json mislabels this as a plain Digimon (card_kind 0) and files the
//! Option-face "Use Requirement / [Main]" text in the INHERITED slot. The card
//! IMAGE (DIGIMON / OPTION DUAL) + DCGO ST24_07.cs are authoritative.
//!
//! # Card text (card image / DCGO — authoritative)
//! DIGIMON FACE — ShineGreymon, Lv.6, DP 12000, Yellow.
//!   <Raid> <Piercing> <Security A. +1>
//!   <Digivolve> Lv.5 w/ [RizeGreymon] in name or [DATA SQUAD] trait: Cost 3
//!     (ignore digivolution requirement).
//!   [When Digivolving] [When Attacking] [Once Per Turn] You may play 1 Tamer
//!     card with a play cost of 5 or less from your hand or trash without paying
//!     the cost. Then, 1 of your opponent's Digimon gets -9000 DP for the turn.
//! OPTION FACE — "GeoGrey Sword", Use 5.
//!   Use Req. [DATA SQUAD] trait (ignore color requirements).
//!   [Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, delete
//!     1 of your opponent's Digimon with 7000 DP or less.
//!   <Arts Digivolve>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Yellow/ST24_07.cs

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "ST24-07";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A DATA SQUAD Tamer (cost ≤5) — a legal free-play target for the Digimon face.
fn ds_tamer(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// A green Lv.5 [DATA SQUAD] base the dual card can digivolve from / arts onto.
fn ds_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Yellow];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-07 in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER", 5))
        .add_card(ds_lv5_base("DS-BASE"))
        .add_card(opp_digimon("OPP-7K", 7000))
        .add_card(opp_digimon("OPP-9K", 9000))
        // A high-DP opponent that SURVIVES the -9000 DP (13000 → 4000) so its DP
        // is observable afterwards (a -9000 to ≤0 DP would delete it).
        .add_card(opp_digimon("OPP-13K", 13000))
        .add_card(filler("FILLER"))
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_07_is_a_dual_card() {
    let runner = builder().memory(10).start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled ST24-07");
    assert_eq!(compiled.kind, CompiledCardKind::Dual, "ST24-07 is DUAL");
    let dual = compiled.dual.as_ref().expect("dual block present");
    assert_eq!(dual.digimon.level, 6);
    assert_eq!(dual.digimon.dp, 12000);
    assert_eq!(dual.option.use_cost, 5);
}

#[test]
fn st24_07_digimon_face_has_keywords_and_alt_path_ignore_requirements() {
    let runner = builder().memory(10).start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");

    // The alt-path is cost 3 and ignores digivolution requirements.
    let alt = compiled
        .alt_paths
        .iter()
        .find(|p| {
            matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(3))
            )
        })
        .expect("Lv.5 cost-3 alt-path present");
    assert!(
        alt.ignore_requirements,
        "alt-path ignores digivolution requirements"
    );

    // Static keywords on the Digimon face (Raid / Piercing / Security A. +1).
    let dual = compiled.dual.as_ref().unwrap();
    let kw_text = format!("{:?}", dual.digimon.effects);
    assert!(kw_text.contains("Raid"), "Digimon face grants <Raid>");
    assert!(
        kw_text.contains("Piercing"),
        "Digimon face grants <Piercing>"
    );
    assert!(
        kw_text.contains("SecurityAttackPlus"),
        "Digimon face grants <Security A. +1>"
    );
}

#[test]
fn st24_07_option_face_has_main_and_arts() {
    let runner = builder().memory(10).start();
    let dual = runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .dual
        .clone()
        .unwrap();
    let has_main = dual.option.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t) if t.when.iter().any(|w| format!("{w:?}").contains("Option") || *w == CompiledTiming::Main))
    });
    assert!(has_main, "Option face carries a [Main] clause");
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "arts_digivolve compiles to an ArtsDigivolve option keyword; got {:?}",
        dual.option.keywords
    );
}

#[test]
fn st24_07_engine_card_data_sees_arts_and_use_requirement() {
    let runner = builder().memory(10).start();
    let data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .expect("ST24-07 card data");
    let dual = data.dual.as_ref().expect("dual data");
    assert!(
        dual.option.keywords.contains(&Keyword::ArtsDigivolve),
        "engine option face carries ArtsDigivolve keyword"
    );
}

// ─── Section 2 — Digimon face [WD][WA][OPT]: play Tamer free, then -9000 DP ───

/// POSITIVE: with a DATA SQUAD Tamer (cost ≤5) in hand and an opponent Digimon,
/// the WD/WA clause plays the Tamer free and -9000s the opponent Digimon.
#[test]
fn st24_07_digimon_wd_plays_tamer_free_then_minus_9000() {
    let mut r = builder().hand(0, &["DS-TAMER"]).memory(10).start();
    let me = r.place_stack(0, &["DS-BASE", CARD_ID]);
    let opp = r.place_on_field(1, "OPP-13K", Some(0));
    r.game.enter_main_phase();

    let dp_before = r.dp_of(opp).unwrap();
    let tamers_before = r.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Tamer)
        .count();

    fire(&mut r, EffectTiming::WhenDigivolving, me);

    // Prompt 1: the optional Tamer free-play (hand/trash union). Pick the Tamer.
    let v = r
        .pending_selection_view()
        .expect("optional Tamer free-play installs");
    assert!(v.is_optional, "'you may play' ⇒ declinable");
    let pick = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .expect("DS-TAMER is a legal free-play pick");
    r.execute_action(0, pick).unwrap();

    // Prompt 2 (mandatory): pick the opponent Digimon for -9000 DP.
    let _ = r.auto_resolve();

    let tamers_after = r.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Tamer)
        .count();
    assert_eq!(
        tamers_after,
        tamers_before + 1,
        "the Tamer was played for free"
    );
    assert_eq!(
        r.dp_of(opp).unwrap(),
        dp_before - 9000,
        "the opponent Digimon gets -9000 DP for the turn"
    );
}

/// The Tamer free-play is declinable, but the -9000 DP still applies (it runs
/// unconditionally after the optional play).
#[test]
fn st24_07_tamer_play_declinable_but_minus_9000_still_applies() {
    let mut r = builder().hand(0, &["DS-TAMER"]).memory(10).start();
    let me = r.place_stack(0, &["DS-BASE", CARD_ID]);
    let opp = r.place_on_field(1, "OPP-13K", Some(0));
    r.game.enter_main_phase();
    let dp_before = r.dp_of(opp).unwrap();

    fire(&mut r, EffectTiming::WhenAttacking, me);
    let v = r
        .pending_selection_view()
        .expect("Tamer free-play installs");
    assert!(v.is_optional);
    r.execute_action(0, PASS).expect("decline the Tamer play");
    let _ = r.auto_resolve();

    assert_eq!(
        r.dp_of(opp).unwrap(),
        dp_before - 9000,
        "the -9000 DP runs even when the Tamer play is declined"
    );
}

// ─── Section 3 — Option face [Main]: -6000 DP, then delete ≤7000 ──────────────

/// POSITIVE: Option [Main] -6000s 1 opponent Digimon, then deletes 1 opponent
/// Digimon with 7000 DP or less.
#[test]
fn st24_07_option_main_minus_6000_then_delete_le_7000() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    // A DATA SQUAD permanent on our field satisfies the Option's <Use Req.>.
    r.place_on_field(0, "DS-BASE", Some(0));
    // Two opponent Digimon: one 7000 (deletable), one 9000 (not deletable).
    let weak = r.place_on_field(1, "OPP-7K", Some(0));
    let _strong = r.place_on_field(1, "OPP-9K", Some(0));
    r.game.enter_main_phase();

    let opp_field_before = r.game.players[1].battle_area.len();

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "Option [Main] parks a selection"
    );

    // Prompt 1 (mandatory): -6000 DP target (any opponent Digimon).
    let v = r.pending_selection_view().expect("-6000 DP pick installs");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();

    // Prompt 2 (optional): delete 1 opponent Digimon with ≤7000 DP. Only the
    // 7000 Digimon is legal (the 9000 is excluded).
    let v2 = r.pending_selection_view().expect("delete pick installs");
    let legal: Vec<_> = v2
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .collect();
    // Pick the deletable Digimon.
    let pick = legal[0];
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();

    let _ = weak;
    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_field_before - 1,
        "exactly 1 opponent Digimon (≤7000 DP) was deleted"
    );
}

/// The delete half is optional (DCGO canNoSelect:true): declining it leaves both
/// opponent Digimon on the field (the -6000 still applied).
#[test]
fn st24_07_option_main_delete_is_optional() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    // A DATA SQUAD permanent on our field satisfies the Option's <Use Req.>.
    r.place_on_field(0, "DS-BASE", Some(0));
    let _weak = r.place_on_field(1, "OPP-7K", Some(0));
    r.game.enter_main_phase();
    let opp_field_before = r.game.players[1].battle_area.len();

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);

    // -6000 DP pick (mandatory).
    let v = r.pending_selection_view().expect("-6000 DP pick installs");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();

    // Decline the optional delete.
    let v2 = r.pending_selection_view().expect("delete pick installs");
    assert!(v2.is_optional, "the delete is optional (canNoSelect:true)");
    r.execute_action(0, PASS).expect("decline the delete");
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_field_before,
        "declined delete ⇒ the opponent Digimon stays on the field"
    );
}
