//! Round-trip integration tests for `ReplayRunner`.
//!
//! Tasks 3.11–3.14 of the `add-engine-debug-mcp` change:
//! - Round-trip parity (record a real game with `HeadlessRunner`, replay it,
//!   assert state matches).
//! - Verify-mode divergence detection.
//! - Seek-equivalence (`seek(N)` ≡ N sequential `step()` calls).
//! - Backward-seek correctness.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{
    ActionRow, GameEnd, GameStart, InitialStateRow, InitialStateSide, RecordingV1, RevealRow, Row,
};
use digimon_engine::recorder::VerificationReplayRecording;
use digimon_engine::runners::replay::{
    DivergenceKind, ReplayError, ReplayRunner, ReplaySession, StepPolicy, VerificationReplayCheck,
};
use digimon_engine::HeadlessRunner;

fn minimal_db() -> HashMap<String, CardData> {
    let json = r#"{
        "ST1-01": {
            "card_id": "ST1-01", "card_name_eng": "Botamon",
            "card_effect_class_name": "ST1_01", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "ST1-02": {
            "card_id": "ST1-02", "card_name_eng": "Punimon",
            "card_effect_class_name": "ST1_02", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "ST1-03": {
            "card_id": "ST1-03", "card_name_eng": "Koromon",
            "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn test_deck() -> Vec<String> {
    std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect()
}

/// Record a game with N steps: 2 mulligan keeps + (N-2) passes. Returns
/// the recording JSON.
fn record_game(n_post_mulligan_steps: usize) -> serde_json::Value {
    let db = minimal_db();
    let deck = test_deck();
    let mut r = HeadlessRunner::new(
        deck.clone(),
        deck,
        &db,
        false, // verbose
        true,  // record_actions
        false, // record_tensors
        Some(42),
    )
    .expect("headless runner constructs");

    // Mulligan: both players keep.
    r.step(0);
    r.step(0);
    for _ in 0..n_post_mulligan_steps {
        r.step(62); // PASS
    }
    r.get_recording().expect("recording present")
}

fn record_verification_game(n_post_mulligan_steps: usize) -> VerificationReplayRecording {
    let db = minimal_db();
    let deck = test_deck();
    let mut r = HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(42))
        .expect("headless runner constructs");

    r.step(0);
    r.step(0);
    for _ in 0..n_post_mulligan_steps {
        r.step(62);
    }

    r.get_verification_replay_recording("sha256:test-cards")
        .expect("verification recording present")
}

#[test]
fn round_trip_construct_replay_no_divergence() {
    let db = minimal_db();
    let recording = record_game(5);
    let mut replay = ReplayRunner::new(recording, &db, true).expect("constructs from recording");

    // 5 replayable steps (the 2 mulligan actions are filtered out).
    assert_eq!(replay.total_steps(), 5);

    let mut total_divergences = 0;
    while !replay.is_complete() {
        let r = replay.step();
        total_divergences += r.divergences.len();
    }
    // With identical engine on both sides and no RNG-consuming effects in
    // this minimal deck, replay should produce zero divergences.
    assert_eq!(
        total_divergences, 0,
        "round-trip replay produced divergences"
    );
}

#[test]
fn run_to_completion_advances_through_all_actions() {
    let db = minimal_db();
    let recording = record_game(3);
    let mut replay = ReplayRunner::new(recording, &db, false).expect("constructs");
    let n = replay.total_steps();
    replay.run_to_completion();
    assert_eq!(replay.current_step(), n);
    assert!(replay.is_complete());
}

#[test]
fn seek_forward_equivalent_to_sequential_steps() {
    let db = minimal_db();
    let recording = record_game(4);

    // Path A: seek directly to step 3.
    let mut r_seek = ReplayRunner::new(recording.clone(), &db, false).unwrap();
    r_seek.seek(3).expect("seek 3");

    // Path B: call step() three times.
    let mut r_step = ReplayRunner::new(recording, &db, false).unwrap();
    for _ in 0..3 {
        r_step.step();
    }

    // Compare key observable state. Internal `Game` equality is not
    // trivially comparable (closures, RNG state), so we compare the
    // serializable surfaces that matter for debugging.
    assert_eq!(r_seek.current_step(), r_step.current_step());
    assert_eq!(r_seek.game.turn_count, r_step.game.turn_count);
    assert_eq!(r_seek.game.memory, r_step.game.memory);
    assert_eq!(r_seek.game.current_phase, r_step.game.current_phase);
    assert_eq!(r_seek.game.turn_player(), r_step.game.turn_player());
    for i in 0..2 {
        assert_eq!(
            r_seek.game.players[i].hand.len(),
            r_step.game.players[i].hand.len()
        );
        assert_eq!(
            r_seek.game.players[i].deck.len(),
            r_step.game.players[i].deck.len()
        );
        assert_eq!(
            r_seek.game.players[i].battle_area.len(),
            r_step.game.players[i].battle_area.len()
        );
    }
}

#[test]
fn seek_backward_rebuilds_from_initial() {
    let db = minimal_db();
    let recording = record_game(5);

    // Path A: step to 4, then seek back to 2.
    let mut r_back = ReplayRunner::new(recording.clone(), &db, false).unwrap();
    for _ in 0..4 {
        r_back.step();
    }
    r_back.seek(2).expect("backward seek to 2");

    // Path B: build fresh and step to 2.
    let mut r_fresh = ReplayRunner::new(recording, &db, false).unwrap();
    for _ in 0..2 {
        r_fresh.step();
    }

    assert_eq!(r_back.current_step(), 2);
    assert_eq!(r_back.current_step(), r_fresh.current_step());
    assert_eq!(r_back.game.turn_count, r_fresh.game.turn_count);
    assert_eq!(r_back.game.memory, r_fresh.game.memory);
    assert_eq!(r_back.game.current_phase, r_fresh.game.current_phase);
}

#[test]
fn verify_mode_detects_injected_memory_divergence() {
    let db = minimal_db();
    let mut recording = record_game(3);

    // Inject a synthetic divergence: corrupt the recorded memory_after of
    // the first replayable action so verify mode reports it.
    let actions = recording["actions"].as_array().unwrap().clone();
    // Find the first non-mulligan action and bump memory_after.
    let mut new_actions = Vec::new();
    let mut bumped = false;
    for mut a in actions {
        if !bumped && a["phase"].as_str() != Some("Mulligan") {
            let cur = a["memory_after"].as_i64().unwrap_or(0);
            a["memory_after"] = serde_json::json!(cur + 999);
            bumped = true;
        }
        new_actions.push(a);
    }
    recording["actions"] = serde_json::Value::Array(new_actions);

    let mut replay = ReplayRunner::new(recording, &db, true).expect("constructs");
    let first = replay.step();
    let mem_div = first.divergences.iter().find(|d| d.field == "memory_after");
    assert!(
        mem_div.is_some(),
        "expected memory_after divergence in step 1, got: {:?}",
        first.divergences
    );
}

#[test]
fn unknown_card_in_recording_errors() {
    let db = minimal_db();
    let mut recording = record_game(2);
    // Replace one card_id in player1's library with something not in `db`.
    recording["initial_state"]["player1"]["library_order"][0] = serde_json::json!("UNKNOWN-XYZ");
    match ReplayRunner::new(recording, &db, false) {
        Err(ReplayError::UnknownCard(ids)) => {
            assert!(ids.contains(&"UNKNOWN-XYZ".to_string()));
        }
        other => panic!("expected UnknownCard, got {:?}", other.err()),
    }
}

// ── ReplaySession (Group 2): generic core over NativeAdapter ────────────────

/// The generic `ReplaySession` + `NativeAdapter` reproduces `ReplayRunner`
/// exactly over a real multi-action recording — proving the native path is
/// preserved by the new abstraction.
#[test]
fn replay_session_round_trip_matches_replay_runner() {
    let db = minimal_db();
    let recording = record_game(5);
    let mut runner = ReplayRunner::new(recording.clone(), &db, false).unwrap();
    let mut session = ReplaySession::new(recording, &db, false).unwrap();
    assert_eq!(session.total_steps(), runner.total_steps());

    runner.run_to_completion();
    session.run_to_completion();

    assert_eq!(session.current_step(), runner.current_step());
    assert_eq!(session.is_game_over(), runner.is_game_over());
    assert_eq!(session.winner_id(), runner.winner_id());
    assert_eq!(session.game.turn_count, runner.game.turn_count);
    assert_eq!(session.game.memory, runner.game.memory);
    assert_eq!(session.game.current_phase, runner.game.current_phase);
    for i in 0..2 {
        assert_eq!(
            session.game.players[i].hand.len(),
            runner.game.players[i].hand.len()
        );
        assert_eq!(
            session.game.players[i].deck.len(),
            runner.game.players[i].deck.len()
        );
    }
}

/// Task 2.5: forward `seek(N)` ≡ N sequential `step()` calls on `ReplaySession`.
#[test]
fn replay_session_seek_forward_equivalent_to_sequential_steps() {
    let db = minimal_db();
    let recording = record_game(4);

    let mut s_seek = ReplaySession::new(recording.clone(), &db, false).unwrap();
    s_seek.seek(3).expect("seek 3");

    let mut s_step = ReplaySession::new(recording, &db, false).unwrap();
    for _ in 0..3 {
        s_step.step();
    }

    assert_eq!(s_seek.current_step(), s_step.current_step());
    assert_eq!(s_seek.game.turn_count, s_step.game.turn_count);
    assert_eq!(s_seek.game.memory, s_step.game.memory);
    assert_eq!(s_seek.game.current_phase, s_step.game.current_phase);
    for i in 0..2 {
        assert_eq!(
            s_seek.game.players[i].hand.len(),
            s_step.game.players[i].hand.len()
        );
    }
}

/// Task 2.6: backward seek (reset-and-replay) yields state equal to a fresh
/// session stepped to the same point.
#[test]
fn replay_session_seek_backward_equals_fresh() {
    let db = minimal_db();
    let recording = record_game(5);

    let mut s_back = ReplaySession::new(recording.clone(), &db, false).unwrap();
    for _ in 0..4 {
        s_back.step();
    }
    s_back.seek(2).expect("backward seek to 2");

    let mut s_fresh = ReplaySession::new(recording, &db, false).unwrap();
    for _ in 0..2 {
        s_fresh.step();
    }

    assert_eq!(s_back.current_step(), 2);
    assert_eq!(s_back.current_step(), s_fresh.current_step());
    assert_eq!(s_back.game.turn_count, s_fresh.game.turn_count);
    assert_eq!(s_back.game.memory, s_fresh.game.memory);
    assert_eq!(s_back.game.current_phase, s_fresh.game.current_phase);
    for i in 0..2 {
        assert_eq!(
            s_back.game.players[i].hand.len(),
            s_fresh.game.players[i].hand.len()
        );
        assert_eq!(
            s_back.game.players[i].deck.len(),
            s_fresh.game.players[i].deck.len()
        );
    }
}

/// Task 2.6: `step_back()` decrements the cursor by one and matches a fresh
/// session stepped to that point.
#[test]
fn replay_session_step_back_decrements_and_matches_fresh() {
    let db = minimal_db();
    let recording = record_game(5);

    let mut s = ReplaySession::new(recording.clone(), &db, false).unwrap();
    for _ in 0..4 {
        s.step();
    }
    s.step_back().expect("step back");
    assert_eq!(s.current_step(), 3);

    let mut fresh = ReplaySession::new(recording, &db, false).unwrap();
    for _ in 0..3 {
        fresh.step();
    }
    assert_eq!(s.game.turn_count, fresh.game.turn_count);
    assert_eq!(s.game.memory, fresh.game.memory);
    assert_eq!(s.game.current_phase, fresh.game.current_phase);
}

// ── Group 3: StepPolicy + Divergence ────────────────────────────────────────

/// Corrupt the first non-mulligan action's `action_id` to one that is illegal
/// at that state (a digivolve with no matching hand/field), leaving the actor
/// intact so the divergence isolates to `MaskMiss`.
fn recording_with_illegal_first_action() -> serde_json::Value {
    let mut recording = record_game(3);
    let actions = recording["actions"].as_array().unwrap().clone();
    let mut new_actions = Vec::new();
    let mut bumped = false;
    for mut a in actions {
        if !bumped && a["phase"].as_str() != Some("Mulligan") {
            a["action_id"] = serde_json::json!(999);
            bumped = true;
        }
        new_actions.push(a);
    }
    recording["actions"] = serde_json::Value::Array(new_actions);
    recording
}

/// Task 3.4: `CheckThenApply` replays a legal recording cleanly (no
/// divergences, runs to completion).
#[test]
fn replay_session_check_then_apply_passes_legal_recording() {
    let db = minimal_db();
    let recording = record_game(4);
    let mut s = ReplaySession::new(recording, &db, false).unwrap();
    s.set_policy(StepPolicy::CheckThenApply);
    s.run_to_completion();
    assert!(
        !s.is_paused(),
        "a legal recording must not pause under CheckThenApply"
    );
    assert!(
        s.divergences().is_empty(),
        "no divergences on a legal recording"
    );
    assert_eq!(s.current_step(), s.total_steps());
}

/// Task 3.4: `CheckThenApply` pauses and records a `MaskMiss` (with a legal
/// sample) on a masked-out action, without advancing the cursor.
#[test]
fn replay_session_check_then_apply_pauses_on_masked_out_action() {
    let db = minimal_db();
    let mut s = ReplaySession::new(recording_with_illegal_first_action(), &db, false).unwrap();
    s.set_policy(StepPolicy::CheckThenApply);
    s.run_to_completion();

    assert!(s.is_paused(), "should pause on the masked-out action");
    let divs = s.divergences();
    assert_eq!(divs.len(), 1, "exactly one (pausing) divergence recorded");
    assert_eq!(divs[0].action_id, 999);
    match &divs[0].kind {
        DivergenceKind::MaskMiss { sample_legal_ids } => {
            assert!(
                !sample_legal_ids.is_empty(),
                "MaskMiss must carry a sample of legal action IDs"
            );
        }
        other => panic!("expected MaskMiss, got {:?}", other),
    }
    // Cursor halted at the divergent (first replayable) step — not applied.
    assert_eq!(s.current_step(), 0);
}

/// Task 3.4: `Trust` (native default) applies without a legality pre-check —
/// it neither pauses nor records differential divergences on an illegal id.
#[test]
fn replay_session_trust_ignores_illegality() {
    let db = minimal_db();
    let mut s = ReplaySession::new(recording_with_illegal_first_action(), &db, false).unwrap();
    assert_eq!(
        s.policy(),
        StepPolicy::Trust,
        "native default policy is Trust"
    );
    s.run_to_completion();
    assert!(!s.is_paused(), "Trust never pauses on legality");
    assert!(
        s.divergences().is_empty(),
        "Trust records no differential divergences"
    );
    assert_eq!(
        s.current_step(),
        s.total_steps(),
        "Trust advances through all steps"
    );
}

// ── Verification replay recordings ────────────────────────────────────────

#[test]
fn replay_session_verification_recording_round_trips_with_digest_checks() {
    let db = minimal_db();
    let recording = record_verification_game(3);
    let total = recording.actions.len() as u32;
    let report =
        ReplaySession::verify_verification_recording("valid-golden", recording, &db).unwrap();

    assert!(
        report.divergence.is_none(),
        "valid recording should not diverge: {:?}",
        report.divergence
    );
    assert_eq!(report.game, "valid-golden");
    assert_eq!(report.checked_steps, total);
}

#[test]
fn verification_recording_reports_first_mask_legality_divergence() {
    let db = minimal_db();
    let mut recording = record_verification_game(2);
    recording.actions[0].action_id = 999;

    let report = ReplaySession::verify_verification_recording("bad-mask", recording, &db).unwrap();
    let divergence = report.divergence.expect("expected first divergence");

    assert_eq!(divergence.game, "bad-mask");
    assert_eq!(divergence.step, 0);
    assert_eq!(divergence.check_type, VerificationReplayCheck::Legality);
    assert!(divergence.recorded.contains("999"));
    assert!(divergence.replayed.contains("legal"));
    assert_eq!(report.checked_steps, 0);
}

#[test]
fn verification_recording_reports_first_digest_divergence() {
    let db = minimal_db();
    let mut recording = record_verification_game(2);
    recording.digests[0] = recording.digests[0].wrapping_add(1);

    let report =
        ReplaySession::verify_verification_recording("bad-digest", recording, &db).unwrap();
    let divergence = report.divergence.expect("expected digest divergence");

    assert_eq!(divergence.game, "bad-digest");
    assert_eq!(divergence.step, 1);
    assert_eq!(divergence.check_type, VerificationReplayCheck::Digest);
    assert_ne!(divergence.recorded, divergence.replayed);
    assert_eq!(report.checked_steps, 1);
}

// ── Group 4: DcgoAdapter (bot + opaque) ─────────────────────────────────────

fn dcgo_bot_recording() -> RecordingV1 {
    RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "bot-test".into(),
            timestamp: "t".into(),
            my_player_id: 0,
            first_player: None,
            is_ai: true,
            my_deck_post_shuffle: test_deck(),
            opp_deck_post_shuffle: Some(test_deck()),
            opp_decklist_composition: None,
            my_egg_deck: None,
            opp_egg_deck: None,
        },
        rows: vec![
            Row::Action(ActionRow {
                step: 0,
                actor: 0,
                action_id: 0,
                phase: "Mulligan".into(),
                source: "mulligan".into(),
                board_p0: None,
                board_p1: None,
                memory: None,
                card_id: None,
                memory_before: None,
            }),
            Row::Action(ActionRow {
                step: 1,
                actor: 1,
                action_id: 0,
                phase: "Mulligan".into(),
                source: "mulligan".into(),
                board_p0: None,
                board_p1: None,
                memory: None,
                card_id: None,
                memory_before: None,
            }),
        ],
        end: GameEnd {
            winner: 0,
            reason: "win".into(),
            total_steps: 2,
        },
    }
}

/// Task 4.2/4.4: a DCGO bot-vs-bot recording reconstructs through the
/// `DcgoAdapter` (both decks ordered), defaults to `CheckThenApply`, and is
/// deterministic (two constructions yield identical initial state).
#[test]
fn dcgo_bot_recording_reconstructs_deterministically() {
    let db = minimal_db();
    let a = ReplaySession::from_dcgo(dcgo_bot_recording(), &db, false).unwrap();
    let b = ReplaySession::from_dcgo(dcgo_bot_recording(), &db, false).unwrap();

    assert_eq!(
        a.policy(),
        StepPolicy::CheckThenApply,
        "DCGO default policy"
    );
    assert_eq!(
        a.total_steps(),
        2,
        "two mulligan actions are replayable steps"
    );
    // Deterministic reconstruction (seed 0).
    assert_eq!(a.game.turn_player(), b.game.turn_player());
    for i in 0..2 {
        assert_eq!(a.game.players[i].deck.len(), b.game.players[i].deck.len());
        assert_eq!(
            a.game.players[i].digitama_deck.len(),
            b.game.players[i].digitama_deck.len()
        );
        assert_eq!(a.game.players[i].hand.len(), b.game.players[i].hand.len());
    }
    // Lands in Mulligan (DCGO replays the mulligan actions).
    assert_eq!(a.game.current_phase.py_name(), "Mulligan");
}

/// Egg (digitama) decks recorded in `game_start` are placed at replay
/// construction: the adapter appends them to the ordered deck lists,
/// `Game`'s card-kind routing splits `DigiEgg` cards into `digitama_deck`
/// (relative order preserved), and ordered construction does NOT re-shuffle
/// them — the recorded order lands verbatim in HATCH order.
///
/// `Player::hatch()` pops from the *end* of `digitama_deck` (same
/// pop-from-end convention `Player::draw()`/`setup_security()` use for the
/// main deck — verified against real DCGO recordings: a recording's
/// `*_deck_post_shuffle[0:5]` matches the built hand exactly, and `[5:10]`
/// matches the built security stack exactly, order for order). Per
/// `dcgo_recording.rs`'s `my_egg_deck` doc ("index 0 is hatched first in
/// DCGO" — the same convention as the main deck's "drawn from index 0
/// first"), the adapter must store `digitama_deck` as the REVERSE of the
/// recorded order so the first `.pop()`/`hatch()` yields the recording's
/// first egg. So `digitama_deck.iter().rev()` — not raw `.iter()` order —
/// is what "placed verbatim" means here.
#[test]
fn dcgo_bot_recording_places_recorded_egg_decks() {
    let db = minimal_db();
    let mut rec = dcgo_bot_recording();
    // Pure main decks + explicit, distinctly-ordered egg decks.
    rec.start.my_deck_post_shuffle = vec!["ST1-03".to_string(); 50];
    rec.start.opp_deck_post_shuffle = Some(vec!["ST1-03".to_string(); 50]);
    rec.start.my_egg_deck = Some(vec![
        "ST1-01".to_string(),
        "ST1-02".to_string(),
        "ST1-01".to_string(),
        "ST1-02".to_string(),
    ]);
    rec.start.opp_egg_deck = Some(vec!["ST1-01".to_string(); 5]);

    let s = ReplaySession::from_dcgo(rec, &db, false).unwrap();
    // my_player_id = 0 in the fixture. `.rev()` converts storage order
    // (pop-from-end) to hatch order (first `.pop()` first).
    let my_eggs: Vec<&str> = s.game.players[0]
        .digitama_deck
        .iter()
        .rev()
        .map(|c| c.card_id(&s.game.card_data))
        .collect();
    assert_eq!(
        my_eggs,
        vec!["ST1-01", "ST1-02", "ST1-01", "ST1-02"],
        "recorded egg order placed verbatim in hatch order (no shuffle)"
    );
    assert_eq!(
        s.game.players[1].digitama_deck.len(),
        5,
        "opp egg deck placed"
    );
    // Main decks are unaffected by the egg append (50 = deck + dealt hand).
    for i in 0..2 {
        assert_eq!(
            s.game.players[i].deck.len() + s.game.players[i].hand.len(),
            50,
            "player {} main deck intact",
            i
        );
    }
}

/// Pre-egg-capture recordings (no egg fields in `game_start`) still
/// construct — with empty digitama decks, matching prior behavior.
#[test]
fn dcgo_bot_recording_without_egg_decks_still_constructs() {
    let db = minimal_db();
    let mut rec = dcgo_bot_recording();
    rec.start.my_deck_post_shuffle = vec!["ST1-03".to_string(); 50];
    rec.start.opp_deck_post_shuffle = Some(vec!["ST1-03".to_string(); 50]);

    let s = ReplaySession::from_dcgo(rec, &db, false).unwrap();
    assert!(s.game.players[0].digitama_deck.is_empty());
    assert!(s.game.players[1].digitama_deck.is_empty());
}

/// Task 4.2: the DCGO mulligan actions replay under `Trust` (Trust avoids
/// coupling the test to the engine's seed-0 mulligan order).
#[test]
fn dcgo_bot_replays_mulligan_under_trust() {
    let db = minimal_db();
    let mut s = ReplaySession::from_dcgo(dcgo_bot_recording(), &db, false).unwrap();
    s.set_policy(StepPolicy::Trust);
    s.run_to_completion();
    assert_eq!(s.current_step(), s.total_steps());
    assert!(!s.is_paused());
}

/// Like [`dcgo_bot_recording`], but the local player (actor 0) actually
/// **redraws** (`action_id: 1`) instead of keeping. DCGO's recorder only
/// ever logs the keep/redraw *decision* for a mulligan
/// (`Digimon.Recording.GameRecorder.LogMulligan(playerID, isRedraw)` in
/// `TurnStateMachine.SetRedraw`) — it never captures the resulting
/// redrawn hand, so a DCGO recording can never carry a genuine
/// post-mulligan `initial_state` snapshot the way a native `GameRecorder`
/// recording does. Uses an all-identical-card deck so the redrawn hand's
/// *content* is deterministic (mod shuffle order) regardless of the
/// engine's own RNG stream, letting the regression assert exact
/// post-mulligan hand content without coupling to a specific seed or to
/// DCGO's (unrecoverable) actual redraw.
fn dcgo_bot_recording_with_redraw() -> RecordingV1 {
    let uniform_deck: Vec<String> = std::iter::repeat("ST1-03".to_string()).take(50).collect();
    RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "redraw-test".into(),
            timestamp: "t".into(),
            my_player_id: 0,
            first_player: Some(0),
            is_ai: true,
            my_deck_post_shuffle: uniform_deck.clone(),
            opp_deck_post_shuffle: Some(uniform_deck),
            opp_decklist_composition: None,
            my_egg_deck: None,
            opp_egg_deck: None,
        },
        rows: vec![
            Row::Action(ActionRow {
                step: 0,
                actor: 0,
                action_id: 1, // redraw, not keep
                phase: "Mulligan".into(),
                source: "mulligan".into(),
                board_p0: None,
                board_p1: None,
                memory: None,
                card_id: None,
                memory_before: None,
            }),
            Row::Action(ActionRow {
                step: 1,
                actor: 1,
                action_id: 0, // keep
                phase: "Mulligan".into(),
                source: "mulligan".into(),
                board_p0: None,
                board_p1: None,
                memory: None,
                card_id: None,
                memory_before: None,
            }),
        ],
        end: GameEnd {
            winner: 0,
            reason: "win".into(),
            total_steps: 2,
        },
    }
}

/// Regression (see module docs "Mulligan handling"): a DCGO-shaped
/// recording with no `initial_state` and a real redraw (not just a keep)
/// must replay the mulligan decisions — under the DCGO adapter's real
/// default policy, `CheckThenApply`, not just under `Trust` — without a
/// spurious divergence, and must land on a legitimate, fully-drawn
/// post-mulligan hand rather than silently keeping the pre-mulligan deal.
#[test]
fn dcgo_bot_redraw_replays_under_check_then_apply_without_divergence() {
    let db = minimal_db();
    let mut s = ReplaySession::from_dcgo(dcgo_bot_recording_with_redraw(), &db, false).unwrap();
    assert_eq!(
        s.policy(),
        StepPolicy::CheckThenApply,
        "DCGO's real default policy — this must hold under production settings, not just Trust"
    );

    s.run_to_completion();

    assert_eq!(
        s.current_step(),
        s.total_steps(),
        "both mulligan decisions (including the redraw) were consumed as replayable steps"
    );
    assert!(
        !s.is_paused(),
        "no pausing divergence — the redraw must be legal under the engine's own mask/actor checks"
    );
    assert!(
        s.divergences().is_empty(),
        "no divergence recorded while replaying the mulligan decisions: {:?}",
        s.divergences()
    );
    // Mulligan is complete for both players — phase advanced past Mulligan.
    assert_ne!(s.game.current_phase.py_name(), "Mulligan");

    // The redraw actually happened: a fresh 5-card hand, not the stale
    // pre-mulligan deal silently left in place by a dropped/filtered action.
    assert_eq!(s.game.players[0].hand.len(), 5, "redrawn hand is full size");
    for card in &s.game.players[0].hand {
        assert_eq!(
            card.card_id(&s.game.card_data),
            "ST1-03",
            "uniform deck: every redrawn card is deterministically ST1-03"
        );
    }
    // The keeping opponent is unaffected and drew directly from the ordered
    // deck (sanity check on the fixture, not the regression itself).
    assert_eq!(s.game.players[1].hand.len(), 5);
}

/// A 50-card deck (4 DigiEggs + 46 main) over the micro DB — the shape the
/// opaque constructor validates against.
fn opaque_micro_db() -> HashMap<String, CardData> {
    minimal_db()
}

fn opaque_micro_deck() -> Vec<String> {
    std::iter::repeat("ST1-01".to_string())
        .take(4)
        .chain(std::iter::repeat("ST1-03".to_string()).take(46))
        .collect()
}

fn dcgo_opaque_recording() -> RecordingV1 {
    // Enough Draw reveals to satisfy the opaque opponent's initial-hand draw
    // at construction. card_ids must be present in the composition multiset.
    let reveals: Vec<Row> = (0..7)
        .map(|i| {
            Row::Reveal(RevealRow {
                step: i,
                actor: 1,
                card_id: "ST1-03".into(),
                source: "draw".into(),
            })
        })
        .collect();
    RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "opaque-test".into(),
            timestamp: "t".into(),
            my_player_id: 0,
            first_player: None,
            is_ai: false,
            my_deck_post_shuffle: opaque_micro_deck(),
            opp_deck_post_shuffle: None,
            opp_decklist_composition: Some(opaque_micro_deck()),
            my_egg_deck: None,
            opp_egg_deck: None,
        },
        rows: reveals,
        end: GameEnd {
            winner: 0,
            reason: "win".into(),
            total_steps: 0,
        },
    }
}

/// Task 4.2/4.3/4.4: an opaque PvP recording reconstructs via
/// `Game::new_with_opaque_opponent` + a `RevealQueue` built from the reveal
/// stream, defaults to `CheckThenApply`, and is deterministic (a fresh reveal
/// queue per build → identical reconstruction, the property backward-seek
/// relies on).
#[test]
fn dcgo_opaque_recording_reconstructs_via_opaque_path() {
    let db = opaque_micro_db();
    let a = ReplaySession::from_dcgo(dcgo_opaque_recording(), &db, false)
        .expect("opaque recording reconstructs via the opaque path");
    let b = ReplaySession::from_dcgo(dcgo_opaque_recording(), &db, false).unwrap();

    assert_eq!(a.policy(), StepPolicy::CheckThenApply);
    // Deterministic opaque reconstruction (fresh RevealQueue at cursor 0).
    assert_eq!(a.game.turn_player(), b.game.turn_player());
    for i in 0..2 {
        assert_eq!(a.game.players[i].hand.len(), b.game.players[i].hand.len());
    }
}

// ── Group 5: partial-observability (opaque placeholders) ────────────────────

/// Task 5.1/5.2: an unrevealed opaque-deck security card surfaces as the
/// `OPAQUE_PLACEHOLDER_CARD_ID` sentinel (with its `placeholders` flag set) in
/// the god view, and reads concretely once materialized by a reveal.
#[test]
fn opaque_security_placeholder_renders_hidden_then_concrete() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::game::Game;
    use digimon_engine::rules::Rules;
    use digimon_engine::view::{Perspective, SecurityView, OPAQUE_PLACEHOLDER_CARD_ID};

    let db = minimal_db();
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(7)).unwrap();
    let real_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == "ST1-03")
        .unwrap();

    // Inject: one unrevealed opaque placeholder + one concrete card into p1's
    // security (Game::new lays no security pre-mulligan, so we control it).
    game.players[1].security.clear();
    game.players[1]
        .security
        .push(CardSource::new_opaque_security_placeholder(1, 9000));
    game.players[1]
        .security
        .push(CardSource::new(real_idx, 1, 9001));

    let sec = SecurityView::from_game(&game, 1, Perspective::God);
    let ids = sec.card_ids.clone().expect("god view exposes card_ids");
    let phs = sec
        .placeholders
        .clone()
        .expect("god view exposes placeholders");
    assert_eq!(ids.len(), 2);
    assert_eq!(phs, vec![true, false], "slot 0 hidden, slot 1 concrete");
    assert_eq!(
        ids[0], OPAQUE_PLACEHOLDER_CARD_ID,
        "unrevealed card must not expose a concrete identity"
    );
    assert_eq!(ids[1], "ST1-03", "concrete card reads normally");

    // Materialize the placeholder (a reveal replaces the slot with a real card).
    game.players[1].security[0] = CardSource::new(real_idx, 1, 9002);
    let sec2 = SecurityView::from_game(&game, 1, Perspective::God);
    assert_eq!(
        sec2.placeholders.unwrap(),
        vec![false, false],
        "after materialization nothing is a placeholder"
    );
    assert_eq!(
        sec2.card_ids.unwrap()[0],
        "ST1-03",
        "materialized card reads concretely"
    );
}

/// Task 4.3: backward seek on a DCGO game rebuilds deterministically — the
/// reconstructed state equals a fresh construction.
#[test]
fn dcgo_bot_backward_seek_rebuilds_to_fresh_state() {
    let db = minimal_db();
    let mut s = ReplaySession::from_dcgo(dcgo_bot_recording(), &db, false).unwrap();
    s.set_policy(StepPolicy::Trust);
    s.run_to_completion();
    s.seek(0).expect("backward seek rebuilds");
    assert_eq!(s.current_step(), 0);

    let fresh = ReplaySession::from_dcgo(dcgo_bot_recording(), &db, false).unwrap();
    assert_eq!(s.game.turn_player(), fresh.game.turn_player());
    for i in 0..2 {
        assert_eq!(
            s.game.players[i].deck.len(),
            fresh.game.players[i].deck.len()
        );
        assert_eq!(
            s.game.players[i].hand.len(),
            fresh.game.players[i].hand.len()
        );
    }
}

// ── Group 6: DCGO memory-gauge parity check ─────────────────────────────

/// A DCGO row whose recorded `memory` agrees with the engine's own value
/// (converted to `my_player_id`'s perspective) produces no divergence.
/// Fresh construction leaves `game.memory == 0` and `memory_pair.0 ==
/// my_player_id == 0` here, so the perspective-converted value is 0.
#[test]
fn dcgo_row_memory_agreeing_produces_no_divergence() {
    let db = minimal_db();
    let mut rec = dcgo_bot_recording();
    if let Row::Action(a) = &mut rec.rows[0] {
        a.memory = Some(0);
    }
    let mut s = ReplaySession::from_dcgo(rec, &db, false).unwrap();
    let _ = s.step();
    assert!(
        s.divergences().is_empty(),
        "agreeing memory must not produce a divergence: {:?}",
        s.divergences()
    );
}

/// A DCGO row whose recorded `memory` disagrees with the engine's value
/// surfaces a distinct `DivergenceKind::Memory` naming both values, the
/// perspective player, and the step — and does NOT pause the cursor
/// (informational, unlike `Actor`/`MaskMiss`), so a batch replay keeps
/// going and the drift is visible at the exact step it happened.
#[test]
fn dcgo_row_memory_disagreeing_produces_memory_divergence() {
    let db = minimal_db();
    let mut rec = dcgo_bot_recording();
    if let Row::Action(a) = &mut rec.rows[0] {
        a.memory = Some(7); // engine memory is 0 here -> mismatch
    }
    let mut s = ReplaySession::from_dcgo(rec, &db, false).unwrap();
    let _ = s.step();
    assert_eq!(s.divergences().len(), 1);
    match &s.divergences()[0].kind {
        DivergenceKind::Memory {
            recorded,
            replayed,
            my_player_id,
        } => {
            assert_eq!(*recorded, 7);
            assert_eq!(*replayed, 0);
            assert_eq!(*my_player_id, 0);
        }
        other => panic!("expected DivergenceKind::Memory, got {:?}", other),
    }
    assert_eq!(s.divergences()[0].step, 0);
    assert!(
        !s.is_paused(),
        "memory divergence is informational and must not pause the cursor"
    );
}

/// A DCGO row with no recorded `memory` (today's corpus shape, predating
/// the field) triggers no check at all — absence is neither agreement
/// nor a failure. `dcgo_bot_recording()`'s fixture rows carry no memory.
#[test]
fn dcgo_row_without_memory_field_produces_no_divergence() {
    let db = minimal_db();
    let rec = dcgo_bot_recording();
    let mut s = ReplaySession::from_dcgo(rec, &db, false).unwrap();
    let _ = s.step();
    assert!(s.divergences().is_empty());
}

// ── Group 7: DCGO post-mulligan `initial_state` snapshot ────────────────

/// A recording WITHOUT an `initial_state` row keeps today's behavior:
/// mulligan actions stay in the replayable stream and the constructed
/// game starts in `Mulligan` phase. Explicit contrast for
/// `dcgo_bot_recording_with_initial_state_snapshot_reconstructs_exact_zones`
/// below.
#[test]
fn dcgo_recording_without_initial_state_row_keeps_mulligan_in_stream() {
    let db = minimal_db();
    let s = ReplaySession::from_dcgo(dcgo_bot_recording(), &db, false).unwrap();
    assert_eq!(
        s.total_steps(),
        2,
        "both mulligan actions remain replayable steps"
    );
    assert_eq!(s.game.current_phase.py_name(), "Mulligan");
}

/// A bot-vs-bot recording WITH a post-mulligan `initial_state` snapshot
/// filters the mulligan actions out of the replayable stream (baked into
/// the snapshot, same contract as a native recording) and reconstructs
/// EXACT zone contents from it rather than re-simulating the mulligan
/// through the engine's own RNG. Uses distinguishable card IDs so
/// ORDERING (not just presence) is pinned: `library_order`/
/// `security_order` use DCGO's "index 0 = top" convention, and the
/// engine's zones pop from the end — `.iter().rev()` must read back in
/// the SAME order as recorded, or the reversal direction is backwards.
#[test]
fn dcgo_bot_recording_with_initial_state_snapshot_reconstructs_exact_zones() {
    let db = minimal_db();
    let my_lib = vec![
        "ST1-03".to_string(),
        "ST1-01".to_string(),
        "ST1-02".to_string(),
    ];
    let my_sec = vec!["ST1-01".to_string(), "ST1-03".to_string()];
    let my_hand = vec!["ST1-02".to_string(), "ST1-02".to_string()];
    let opp_lib = vec!["ST1-01".to_string()];
    let opp_sec = vec!["ST1-03".to_string()];
    let opp_hand = vec!["ST1-01".to_string()];

    let mut rec = dcgo_bot_recording();
    rec.rows.push(Row::InitialState(InitialStateRow {
        first_player_id: 0,
        my: InitialStateSide {
            library_order: my_lib.clone(),
            digitama_library_order: vec![],
            security_order: my_sec.clone(),
            initial_hand: my_hand.clone(),
        },
        opp: Some(InitialStateSide {
            library_order: opp_lib.clone(),
            digitama_library_order: vec![],
            security_order: opp_sec.clone(),
            initial_hand: opp_hand.clone(),
        }),
        memory: Some(0),
    }));

    let s = ReplaySession::from_dcgo(rec, &db, false)
        .expect("recording with initial_state reconstructs");

    assert_eq!(
        s.total_steps(),
        0,
        "mulligan rows filtered; nothing else recorded"
    );
    assert_ne!(
        s.game.current_phase.py_name(),
        "Mulligan",
        "already past mulligan"
    );

    let my_lib_draw_order: Vec<&str> = s.game.players[0]
        .deck
        .iter()
        .rev()
        .map(|c| c.card_id(&s.game.card_data))
        .collect();
    assert_eq!(my_lib_draw_order, my_lib, "my library in recorded draw order");

    let my_sec_draw_order: Vec<&str> = s.game.players[0]
        .security
        .iter()
        .rev()
        .map(|c| c.card_id(&s.game.card_data))
        .collect();
    assert_eq!(
        my_sec_draw_order, my_sec,
        "my security in recorded top-first order"
    );

    let mut my_hand_ids: Vec<&str> = s.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&s.game.card_data))
        .collect();
    my_hand_ids.sort_unstable();
    let mut expected_my_hand: Vec<&str> = my_hand.iter().map(|s| s.as_str()).collect();
    expected_my_hand.sort_unstable();
    assert_eq!(my_hand_ids, expected_my_hand, "my exact hand content");

    let opp_lib_draw_order: Vec<&str> = s.game.players[1]
        .deck
        .iter()
        .rev()
        .map(|c| c.card_id(&s.game.card_data))
        .collect();
    assert_eq!(
        opp_lib_draw_order, opp_lib,
        "opp library in recorded draw order (bot-vs-bot: fully observable)"
    );
}

/// A PvP-opaque recording's `initial_state` row carries only `my` (the
/// opponent's post-mulligan hand isn't observable — same visibility
/// split as `opp_deck_post_shuffle`). Only MY zones get overwritten from
/// the snapshot; the opponent's opaque/`RevealQueue`-backed
/// representation from the pre-snapshot construction is untouched and
/// still functions.
#[test]
fn dcgo_opaque_recording_with_initial_state_overwrites_only_my_side() {
    let db = opaque_micro_db();
    let mut rec = dcgo_opaque_recording();
    let my_lib = vec!["ST1-03".to_string(), "ST1-01".to_string()];
    let my_sec = vec!["ST1-01".to_string()];
    let my_hand = vec!["ST1-03".to_string(), "ST1-03".to_string()];
    rec.rows.push(Row::InitialState(InitialStateRow {
        first_player_id: 0,
        my: InitialStateSide {
            library_order: my_lib.clone(),
            digitama_library_order: vec![],
            security_order: my_sec.clone(),
            initial_hand: my_hand.clone(),
        },
        opp: None,
        memory: Some(0),
    }));

    let s = ReplaySession::from_dcgo(rec, &db, false)
        .expect("opaque recording with initial_state still reconstructs");

    let my_lib_draw_order: Vec<&str> = s.game.players[0]
        .deck
        .iter()
        .rev()
        .map(|c| c.card_id(&s.game.card_data))
        .collect();
    assert_eq!(my_lib_draw_order, my_lib);
    assert_eq!(s.game.players[0].hand.len(), my_hand.len());
    // Opponent side is still the opaque/reveal-queue construction from
    // `build_initial_game_inner` -- unaffected by (and un-overwritten by)
    // the snapshot, since `opp` was `None`.
    assert!(
        !s.game.players[1].hand.is_empty(),
        "opponent's opaque hand from the pre-snapshot construction is intact"
    );
}
