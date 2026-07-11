//! Integration tests: build full synthetic JSONL recordings, feed them
//! through `parse_jsonl` + `replay_recording`, and assert the harness
//! produces the expected outcomes end-to-end.
//!
//! These tests exercise the full pipeline (parser → replay → aggregator)
//! against fixture JSONL text — no engine fixture state is hand-built,
//! so they catch wiring bugs the unit tests in `lib.rs` miss.

use std::collections::HashMap;

use dcgo_replay::{
    aggregate, parse_jsonl, replay_recording, ReplayConfig, ReplayFail, ReplayOutcome,
};
use digimon_engine::card_data::CardData;

fn card_db() -> HashMap<String, CardData> {
    // Minimum viable card pool: a DigiEgg + a Rookie + a Champion. Enough
    // to make a legal 50-card deck and to let mulligan/main-phase actions
    // be legal in the engine.
    let json = r#"{
        "BT1-001": {
            "card_id": "BT1-001", "card_name_eng": "Koromon",
            "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-010": {
            "card_id": "BT1-010", "card_name_eng": "Agumon",
            "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn deck_jsonl() -> String {
    // 50 cards: 4 DigiEggs (BT1-001) + 46 Agumon (BT1-010). The engine
    // validates: 50 main + 5 max DigiEgg, etc. — this layout passes.
    let mut cards = Vec::new();
    for _ in 0..4 {
        cards.push("\"BT1-001\"".to_string());
    }
    for _ in 0..46 {
        cards.push("\"BT1-010\"".to_string());
    }
    format!("[{}]", cards.join(","))
}

/// Build a minimal valid recording where both players mulligan-keep on
/// turn 1 then immediately surrender (CONCEDE action ID 93), producing
/// a deterministic winner. Used to exercise the happy path.
fn surrender_recording() -> String {
    let deck = deck_jsonl();
    format!(
        r#"{{"v":1,"type":"game_start","game_id":"g_surrender","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":{deck}}}
{{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":1,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":2,"actor":0,"action_id":93,"phase":"Main","source":"main_phase"}}
{{"type":"game_end","winner":1,"reason":"concede","total_steps":3}}
"#,
        deck = deck
    )
}

#[test]
fn integration_surrender_recording_passes() {
    let text = surrender_recording();
    let recording = parse_jsonl(&text).expect("parse");
    let db = card_db();
    let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
    match outcome {
        ReplayOutcome::Pass {
            steps_consumed,
            winner,
        } => {
            // 3 actions consumed; the conceder (player 0) loses so winner = 1.
            assert_eq!(steps_consumed, 3);
            assert_eq!(winner, 1);
        }
        other => panic!("expected Pass, got {:?}", other),
    }
}

/// A recording where step 2 records the wrong winner (concede was by
/// player 0 → player 1 should win, but recording says player 0 won).
/// The replay should detect this as a WinnerMismatch failure.
#[test]
fn integration_wrong_winner_yields_winner_mismatch() {
    let deck = deck_jsonl();
    let text = format!(
        r#"{{"v":1,"type":"game_start","game_id":"g_wrong_winner","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":{deck}}}
{{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":1,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":2,"actor":0,"action_id":93,"phase":"Main","source":"main_phase"}}
{{"type":"game_end","winner":0,"reason":"concede","total_steps":3}}
"#,
        deck = deck
    );
    let recording = parse_jsonl(&text).expect("parse");
    let db = card_db();
    let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
    match outcome {
        ReplayOutcome::Fail(ReplayFail::WinnerMismatch(wm)) => {
            assert_eq!(wm.expected_winner, 0);
            assert_eq!(wm.engine_winner, 1);
        }
        other => panic!("expected WinnerMismatch, got {:?}", other),
    }
}

/// A recording with an illegal action id (999) mid-stream — should
/// surface IllegalAction with sample mask entries for diagnosis.
#[test]
fn integration_illegal_action_mid_stream() {
    let deck = deck_jsonl();
    let text = format!(
        r#"{{"v":1,"type":"game_start","game_id":"g_illegal","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":{deck}}}
{{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":1,"actor":1,"action_id":999,"phase":"Mulligan","source":"mulligan"}}
{{"type":"game_end","winner":0,"reason":"win","total_steps":2}}
"#,
        deck = deck
    );
    let recording = parse_jsonl(&text).expect("parse");
    let db = card_db();
    let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
    match outcome {
        ReplayOutcome::Fail(ReplayFail::IllegalAction(ia)) => {
            assert_eq!(ia.action_id, 999);
            assert_eq!(ia.actor, 1);
            assert_eq!(ia.step, 1);
            // The engine should at least have the mulligan-keep action legal.
            assert!(ia.sample_legal_ids.contains(&0));
        }
        other => panic!("expected IllegalAction, got {:?}", other),
    }
}

/// Phase 3 (opaque PvP) replay path. A recording where
/// `opp_deck_post_shuffle == null` triggers opaque-deck construction;
/// the harness preloads a `RevealQueue` from the recording's reveal rows
/// and lets the engine consume them as it draws / sets up security.
///
/// Recording shape:
///   - my_deck: 50 cards, fully ordered (as if we were the local client).
///   - opp_deck: null — the opponent's deck order is opaque.
///   - reveal rows: enough to satisfy initial-hand + security setup for
///     the opaque opponent.
///   - actions: mulligan-keep for both players, then concede by player 0.
///   - game_end: winner = 1 (since player 0 conceded).
#[test]
fn integration_opaque_pvp_recording_passes() {
    let deck = deck_jsonl();
    // We need: starting_hand=5 + security_count=5 = 10 reveals for the
    // opaque opponent's initial setup. Serve a mix of BT1-010 and BT1-001
    // so the multiset has both card types represented.
    let mut reveal_lines = String::new();
    for i in 0..10 {
        let kind = if i < 5 { "draw" } else { "security" };
        let card = "BT1-010";
        reveal_lines.push_str(&format!(
            r#"{{"type":"reveal","step":{},"actor":1,"card_id":"{}","source":"{}"}}
"#,
            i, card, kind
        ));
    }

    let text = format!(
        r#"{{"v":1,"type":"game_start","game_id":"g_opaque","timestamp":"t","my_player_id":0,"is_ai":false,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":null,"opp_decklist_composition":{deck}}}
{reveal_lines}{{"type":"action","step":10,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":11,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":12,"actor":0,"action_id":93,"phase":"Main","source":"main_phase"}}
{{"type":"game_end","winner":1,"reason":"concede","total_steps":13}}
"#,
        deck = deck,
        reveal_lines = reveal_lines
    );

    let recording = parse_jsonl(&text).expect("parse opaque recording");

    // Sanity-check the parser saw the reveals as explicit Reveal rows.
    let reveal_count = recording
        .rows
        .iter()
        .filter(|r| matches!(r, dcgo_replay::Row::Reveal(_)))
        .count();
    assert_eq!(reveal_count, 10, "expected 10 reveal rows to be parsed");

    let db = card_db();
    let outcome = replay_recording(&recording, &db, &ReplayConfig::default());

    match outcome {
        ReplayOutcome::Pass { winner, .. } => {
            // Player 0 conceded, so winner is player 1.
            assert_eq!(
                winner, 1,
                "opaque replay: player 1 should win after p0 concedes"
            );
        }
        other => panic!("expected Pass for opaque replay, got {:?}", other),
    }
}

/// An opaque recording with a reveal stream tagged inconsistently
/// (e.g., first reveal tagged `security` when the engine needs a Draw
/// first for the initial hand) surfaces as OpaqueRevealError, not as a
/// silent pass.
#[test]
fn integration_opaque_pvp_kind_mismatch_surfaces_opaque_error() {
    let deck = deck_jsonl();
    // Build a reveal stream where the first reveal is tagged "security"
    // — but the engine will request a Draw reveal first (for initial hand).
    // RevealQueue's strict_kind_check should reject this.
    let mut reveal_lines = String::new();
    for i in 0..10 {
        reveal_lines.push_str(&format!(
            r#"{{"type":"reveal","step":{},"actor":1,"card_id":"BT1-010","source":"security"}}
"#,
            i
        ));
    }

    let text = format!(
        r#"{{"v":1,"type":"game_start","game_id":"g_kind_mismatch","timestamp":"t","my_player_id":0,"is_ai":false,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":null,"opp_decklist_composition":{deck}}}
{reveal_lines}{{"type":"game_end","winner":1,"reason":"concede","total_steps":0}}
"#,
        deck = deck,
        reveal_lines = reveal_lines
    );

    let recording = parse_jsonl(&text).expect("parse");
    let db = card_db();
    let outcome = replay_recording(&recording, &db, &ReplayConfig::default());

    match outcome {
        ReplayOutcome::Fail(ReplayFail::OpaqueRevealError { message, .. }) => {
            // The engine's RevealQueue rejected the mismatch; the
            // construction wrapper surfaced it as OpaqueRevealError.
            assert!(
                message.contains("security")
                    || message.contains("draw")
                    || message.contains("Game::new_with_opaque_opponent"),
                "expected message to reference the kind mismatch, got: {}",
                message
            );
        }
        other => panic!("expected OpaqueRevealError, got {:?}", other),
    }
}

/// End-to-end aggregation: 3 synthetic recordings (one pass, one fail
/// with illegal action, one wrong winner) produce a report with correct
/// counts and per-kind breakdown.
#[test]
fn integration_aggregate_mixed_corpus() {
    let pass = surrender_recording();
    let deck = deck_jsonl();
    let illegal = format!(
        r#"{{"v":1,"type":"game_start","game_id":"illegal","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":{deck}}}
{{"type":"action","step":0,"actor":0,"action_id":999,"phase":"Mulligan","source":"mulligan"}}
{{"type":"game_end","winner":0,"reason":"win","total_steps":1}}
"#,
        deck = deck
    );
    let wrong = format!(
        r#"{{"v":1,"type":"game_start","game_id":"wrong","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":{deck},"opp_deck_post_shuffle":{deck}}}
{{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"action","step":1,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}}
{{"type":"game_end","winner":0,"reason":"win","total_steps":2}}
"#,
        deck = deck
    );

    let r_pass = parse_jsonl(&pass).expect("parse pass");
    let r_illegal = parse_jsonl(&illegal).expect("parse illegal");
    let r_wrong = parse_jsonl(&wrong).expect("parse wrong");

    let db = card_db();
    let cfg = ReplayConfig::default();
    let o_pass = replay_recording(&r_pass, &db, &cfg);
    let o_illegal = replay_recording(&r_illegal, &db, &cfg);
    let o_wrong = replay_recording(&r_wrong, &db, &cfg);

    let report = aggregate(&[
        (&r_pass, &o_pass),
        (&r_illegal, &o_illegal),
        (&r_wrong, &o_wrong),
    ]);
    assert_eq!(report.total_recordings, 3);
    assert_eq!(report.pass, 1);
    assert_eq!(report.fail, 2);
    assert_eq!(report.by_kind.get("illegal_action").copied(), Some(1));
    assert_eq!(report.by_kind.get("winner_mismatch").copied(), Some(1));
    assert_eq!(report.failed_games, vec!["illegal", "wrong"]);
}
