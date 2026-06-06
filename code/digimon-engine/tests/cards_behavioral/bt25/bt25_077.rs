//! BT25-077 Bacchusmon — Digimon, Lv.6, Black/Green, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Virus.
//!
//! # Card text (card image BT25-077 — authoritative for printed text)
//! When this card would be played, if there are 12 or more levels' total worth
//! of Digimon, reduce the cost by 5.   (OMITTED — G-DSL-BOARD-LEVEL-SUM)
//! [On Play] [When Digivolving] You may play 1 [TS] trait Digimon card with 6000
//! DP or less from your hand without paying the cost.
//! [All Turns] [Once Per Turn] When any Digimon are played or digivolve, you may
//! suspend 1 Digimon. Then, if played or digivolved by an effect, delete 1 of
//! your opponent's lowest DP Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_077.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - C1 play-from-hand free; observer suspend; effect-gated lowest-DP delete

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, CostDelta, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-077";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-077 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("TS-LOW", 4, 5000, &["TS"]))
        .add_card(make_digimon("TS-BIG", 5, 9000, &["TS"]))
        .add_card(make_digimon("OWN-PLAY", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-SMALL", 3, 2000, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 5, 9000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_077_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Bacchusmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_077_has_op_wd_play_free_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD play-free clause present");
    assert!(clause.optional, "printed 'you may' -> optional");
    assert!(clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. })));
}

#[test]
fn bt25_077_has_all_turns_suspend_delete_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    || t.when.contains(&CompiledTiming::OnDigivolve))
                    && t.optional
                    && t.once_per_turn =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[All Turns] suspend/delete clause present");
    assert!(
        clause.process.iter().any(|s| matches!(s, CompiledStep::Suspend { .. })),
        "clause has a top-level suspend step"
    );
    // The lowest-DP delete is nested inside the `if event_is_effect_initiated`
    // gate, so it appears as a CompiledStep::If at the clause top level.
    assert!(
        clause.process.iter().any(|s| matches!(s, CompiledStep::If { .. })),
        "clause has an effect-initiated gate (If) wrapping the lowest-DP delete"
    );
}

// ─── Behavior: [OP][WD] play a TS <=6000 Digimon from hand free ─────────────

#[test]
fn bt25_077_on_play_offers_ts_play_from_hand() {
    let mut runner = base()
        .hand(0, &["TS-LOW"]) // an eligible [TS] <=6000 Digimon in hand
        .memory(12)
        .start();
    let bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bac.index as usize);

    assert!(
        runner.pending_selection().is_some(),
        "OP/WD installs the hand-play selection when an eligible [TS] <=6000 Digimon is in hand"
    );
}

#[test]
fn bt25_077_on_play_no_prompt_without_eligible_ts_card() {
    let mut runner = base()
        .hand(0, &["TS-BIG"]) // [TS] but 9000 DP -> not eligible
        .memory(12)
        .start();
    let bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bac.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no eligible [TS] <=6000 Digimon -> the optional play clause installs no prompt"
    );
}

// ─── Behavior: [All Turns] effect-play -> suspend + delete lowest ───────────

#[test]
fn bt25_077_all_turns_fires_on_effect_play() {
    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));

    let played = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, played);

    // The observer installs (optional suspend prompt at minimum).
    assert!(
        runner.pending_selection().is_some(),
        "[All Turns] clause fires when a Digimon is played by effect"
    );
}

#[test]
fn bt25_077_all_turns_no_fire_on_normal_hand_play() {
    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));

    runner
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByHand);

    // The suspend is offered on ANY play/digivolve; but with no eligible suspend
    // target chosen and not by-effect, the delete must NOT fire. We assert the
    // clause does not delete the opponent's Digimon on a normal hand play.
    runner.auto_resolve().ok();
    let opp_count = runner.battle_area_size(1);
    assert!(
        opp_count >= 1,
        "normal hand play is not effect-initiated -> no lowest-DP delete"
    );
}
