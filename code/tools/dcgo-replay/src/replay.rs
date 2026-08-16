//! Replays a parsed [`RecordingV1`] through `digimon_engine::HeadlessRunner`
//! and validates parity at each step.
//!
//! Parity is defined as:
//!   1. The action stream is **legality-consistent**: every recorded
//!      `action_id` is set in the engine's action mask at the moment the
//!      engine expects that actor to decide, except `CONCEDE_GAME`, which
//!      remains a decoder-accepted surrender primitive even though RL masks
//!      hide it from learnable policy actions.
//!   2. The actor stream is **player-consistent**: the recording's `actor`
//!      field matches the engine's `current_decision_player()` at every step.
//!   3. The terminal state is **winner-consistent**: after consuming every
//!      decision, the engine's `winner_id()` matches the recording's
//!      `game_end.winner`.
//!
//! When the harness hits an unencoded selection (`encoder_failure` row in
//! the recording — the Phase 1 fallback), it cannot proceed — the engine
//! is still waiting for that selection to resolve and won't accept
//! subsequent main-phase actions. The harness halts cleanly with
//! [`ReplayOutcome::PartialPass`] and reports the step it stopped at.
//! Once selection encoding lands (task 3.5 follow-up), these recordings
//! will replay all the way to game-end.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::runners::replay::{Divergence, DivergenceKind, ReplaySession};

use crate::recording::{RecordingV1, Row};

/// Knobs for the replay harness. Defaults are sensible for batch runs;
/// override when debugging a specific recording.
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Step cap as a runaway guard. Real DCGO games terminate within a
    /// few hundred decisions; 5000 is a generous safety net.
    pub max_steps: u32,
    /// Print per-step trace to stderr. Off in batch mode; on for triage.
    pub verbose: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            max_steps: 5000,
            verbose: false,
        }
    }
}

/// The verdict for one recording.
#[derive(Debug, Clone)]
pub enum ReplayOutcome {
    /// Action stream fully consumed and winner matches. The parity oracle's
    /// happy path.
    Pass { steps_consumed: u32, winner: u8 },
    /// The harness halted before reaching `game_end`, but not because of an
    /// engine-vs-recording disagreement. Typical cause: an unencoded
    /// selection (`encoder_failure` row).
    ///
    /// This is *not* a parity failure — the harness is honestly admitting
    /// "I can't continue replay from here without the selection encoding."
    /// In the per-card report it bucks into a separate "stopped at" tally
    /// rather than the failure counts.
    PartialPass {
        steps_consumed: u32,
        stop_reason: String,
    },
    /// A genuine parity disagreement.
    Fail(ReplayFail),
}

/// The flavors of parity failure the harness surfaces.
#[derive(Debug, Clone)]
pub enum ReplayFail {
    /// Action ID was not in the engine's legal-action mask at this step.
    IllegalAction(IllegalAction),
    /// Engine expected a different actor to decide.
    ActorMismatch(ActorMismatch),
    /// Engine arrived at a different winner (or no winner where the
    /// recording had one, or vice versa).
    WinnerMismatch(WinnerMismatch),
    /// The Game constructor or step path returned an engine-level error.
    /// Usually a deck-data issue (card ID unknown to our pool) or a
    /// step taken past game_over.
    EngineError { step: Option<u32>, message: String },
    /// Opaque-deck mode: the supplied `RevealQueue` and the engine's
    /// reveal requests went out of alignment. Common causes:
    ///   - Recording's `reveal` row tagged `draw` but engine requested a
    ///     `Security` reveal at that point (recorder mis-tagged, or
    ///     engine resolution order diverged from DCGO).
    ///   - Recording exhausted the reveal queue mid-game (truncated
    ///     recording, or engine drew more times than DCGO did).
    ///   - Recording's reveal card ID not present in the opaque pile's
    ///     multiset (recording bug — supplier returned a card the
    ///     decklist didn't contain).
    OpaqueRevealError { step: Option<u32>, message: String },
}

#[derive(Debug, Clone)]
pub struct IllegalAction {
    pub step: u32,
    pub actor: u8,
    pub action_id: u16,
    pub phase: String,
    pub source: String,
    /// Up to ~10 action IDs the engine WAS willing to accept here. Helpful
    /// for diagnosing "did the recorder emit the wrong target index" vs.
    /// "is the engine refusing actions it should accept".
    pub sample_legal_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct ActorMismatch {
    pub step: u32,
    pub expected_actor: u8,
    pub recorded_actor: u8,
    pub action_id: u16,
    pub phase: String,
}

#[derive(Debug, Clone)]
pub struct WinnerMismatch {
    pub expected_winner: i8, // signed so -1 (no winner) round-trips
    pub engine_winner: i8,
    pub steps_consumed: u32,
}

/// Replay one recording through the engine via the shared
/// [`ReplaySession`] + `DcgoAdapter` (the same construction + step path the
/// interactive bug-hunter uses — one code path for batch and interactive).
///
/// Bot-vs-bot recordings (`opp_deck_post_shuffle.is_some()`) reconstruct via
/// `Game::new`; opaque PvP recordings build via `Game::new_with_opaque_opponent`
/// with a `RevealQueue` from the reveal stream. The session runs under the
/// adapter's default `CheckThenApply` policy, so a recorded action the engine
/// masks out pauses and records a divergence, except the decoder-supported
/// `CONCEDE_GAME` surrender primitive. This driver maps divergences back to
/// the batch [`ReplayOutcome`] taxonomy.
pub fn replay_recording(
    recording: &RecordingV1,
    card_data: &HashMap<String, CardData>,
    config: &ReplayConfig,
) -> ReplayOutcome {
    let is_opaque = recording.start.opp_deck_post_shuffle.is_none();

    let mut session = match ReplaySession::from_dcgo(recording.clone(), card_data, false) {
        Ok(s) => s,
        Err(e) => {
            // Opaque construction failures (missing/short decklist, bad reveal
            // tag) keep the prior OpaqueRevealError contract; otherwise it's a
            // deck-data / engine construction error.
            let message = e.to_string();
            return if is_opaque {
                ReplayOutcome::Fail(ReplayFail::OpaqueRevealError {
                    step: None,
                    message,
                })
            } else {
                ReplayOutcome::Fail(ReplayFail::EngineError {
                    step: None,
                    message,
                })
            };
        }
    };

    session.run_to_completion();

    if config.verbose {
        eprintln!(
            "replayed {} / {} steps; divergences={}",
            session.current_step(),
            session.total_steps(),
            session.divergences().len()
        );
    }

    // A pausing differential divergence → the matching batch failure.
    if let Some(div) = session.divergences().first() {
        return ReplayOutcome::Fail(map_divergence(div));
    }

    // An encoder_failure truncated the replayable prefix — the engine is
    // waiting on an unencoded selection. Halt cleanly (not a parity failure).
    if let Some(ef) = recording.rows.iter().find_map(|r| match r {
        Row::EncoderFailure(ef) => Some(ef),
        _ => None,
    }) {
        return ReplayOutcome::PartialPass {
            steps_consumed: session.current_step(),
            stop_reason: format!(
                "hit encoder_failure row at step {} ({}; raw={}). The engine cannot \
                 advance without the corresponding selection encoded.",
                ef.step, ef.reason, ef.raw_value
            ),
        };
    }

    // Terminal winner check.
    let engine_winner_raw: u8 = session.winner_id().unwrap_or(u8::MAX);
    let engine_winner_i8: i8 = if engine_winner_raw == u8::MAX {
        -1
    } else {
        engine_winner_raw as i8
    };
    if engine_winner_i8 != recording.end.winner {
        return ReplayOutcome::Fail(ReplayFail::WinnerMismatch(WinnerMismatch {
            expected_winner: recording.end.winner,
            engine_winner: engine_winner_i8,
            steps_consumed: session.current_step(),
        }));
    }

    ReplayOutcome::Pass {
        steps_consumed: session.current_step(),
        winner: engine_winner_raw,
    }
}

/// Map a `ReplaySession` differential [`Divergence`] to the batch
/// [`ReplayFail`] taxonomy (task 4.5).
fn map_divergence(div: &Divergence) -> ReplayFail {
    match &div.kind {
        DivergenceKind::MaskMiss { sample_legal_ids } => ReplayFail::IllegalAction(IllegalAction {
            step: div.step,
            actor: div.actor,
            action_id: div.action_id,
            phase: div.phase.clone(),
            // The DCGO source tag isn't carried on the divergence; phase is the
            // closest stable descriptor for triage clustering.
            source: div.phase.clone(),
            sample_legal_ids: sample_legal_ids.clone(),
        }),
        DivergenceKind::Actor { expected, recorded } => ReplayFail::ActorMismatch(ActorMismatch {
            step: div.step,
            expected_actor: *expected,
            recorded_actor: *recorded,
            action_id: div.action_id,
            phase: div.phase.clone(),
        }),
        DivergenceKind::Winner { recorded, replayed } => {
            let to_i8 = |w: &Option<u8>| w.map(|x| x as i8).unwrap_or(-1);
            ReplayFail::WinnerMismatch(WinnerMismatch {
                expected_winner: to_i8(recorded),
                engine_winner: to_i8(replayed),
                steps_consumed: div.step,
            })
        }
        DivergenceKind::RevealKind { message } | DivergenceKind::RevealExhausted { message } => {
            ReplayFail::OpaqueRevealError {
                step: Some(div.step),
                message: message.clone(),
            }
        }
        DivergenceKind::SelectionResolution { reason } => ReplayFail::EngineError {
            step: Some(div.step),
            message: format!("selection resolution: {reason}"),
        },
        DivergenceKind::Memory { recorded, replayed } => ReplayFail::EngineError {
            step: Some(div.step),
            message: format!(
                "memory divergence: recorded={} replayed={}",
                recorded, replayed
            ),
        },
        DivergenceKind::Phase { recorded, replayed } => ReplayFail::EngineError {
            step: Some(div.step),
            message: format!(
                "phase divergence: recorded={} replayed={}",
                recorded, replayed
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal card pool that supports trivial parity tests — enough for a
    /// game to play out a few mulligan + pass actions.
    fn micro_card_db() -> HashMap<String, CardData> {
        // Same shape as policies_headless.rs::test_card_db. Three cards
        // are the minimum needed to construct a legal 50-card standard
        // deck (4 DigiEggs + 46 main cards).
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
            },
            "BT1-025": {
                "card_id": "BT1-025", "card_name_eng": "Greymon",
                "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
                "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": ["Vaccine"],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    fn micro_deck() -> Vec<String> {
        let mut d = Vec::new();
        for _ in 0..4 {
            d.push("BT1-001".to_string());
        }
        for _ in 0..30 {
            d.push("BT1-010".to_string());
        }
        for _ in 0..16 {
            d.push("BT1-025".to_string());
        }
        d
    }

    /// An opaque PvP recording without enough reveal rows to derive the
    /// opponent's decklist composition surfaces as OpaqueRevealError.
    /// (When task 7.x adds an explicit `opp_decklist_composition` field
    /// to the recording schema, this test will update to verify the
    /// happy path uses the explicit field instead of the reveal-stream
    /// derivation.)
    #[test]
    fn opaque_recording_without_reveals_surfaces_opaque_error() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":false,"my_deck_post_shuffle":["BT1-010"],"opp_deck_post_shuffle":null}
{"type":"game_end","winner":0,"reason":"win","total_steps":0}
"#;
        let recording = crate::recording::parse_jsonl(txt).expect("parse");
        let db = micro_card_db();
        let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
        match outcome {
            ReplayOutcome::Fail(ReplayFail::OpaqueRevealError { message, .. }) => {
                assert!(
                    message.contains("opp_decklist_composition")
                        || message.contains("reveal stream"),
                    "unexpected error message: {}",
                    message
                );
            }
            other => panic!("expected OpaqueRevealError for opaque PvP, got {:?}", other),
        }
    }

    /// Hitting an encoder_failure row halts replay cleanly with
    /// PartialPass, not a Fail.
    #[test]
    fn encoder_failure_row_yields_partial_pass() {
        // Construct a recording with one main-phase action then an
        // encoder_failure for a selection.
        let mut recording = crate::recording::RecordingV1 {
            start: crate::recording::GameStart {
                v: 1,
                game_id: "test".into(),
                timestamp: "t".into(),
                my_player_id: 0,
                first_player: None,
                is_ai: true,
                my_deck_post_shuffle: micro_deck(),
                opp_deck_post_shuffle: Some(micro_deck()),
                opp_decklist_composition: None,
            },
            rows: vec![Row::EncoderFailure(crate::recording::EncoderFailureRow {
                step: 0,
                actor: 0,
                phase: "Mulligan".into(),
                source: "selection_int".into(),
                reason: "selection_prompt_kind_unknown".into(),
                raw_value: "int_value=5".into(),
            })],
            end: crate::recording::GameEnd {
                winner: 0,
                reason: "win".into(),
                total_steps: 1,
            },
        };
        let _ = &mut recording; // suppress "does not need mut" warning if compiler ever gets smarter
        let db = micro_card_db();
        let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
        match outcome {
            ReplayOutcome::PartialPass { stop_reason, .. } => {
                assert!(stop_reason.contains("encoder_failure"));
            }
            other => panic!("expected PartialPass, got {:?}", other),
        }
    }

    /// A recording with mulligan-keep for both players, then immediately
    /// game_end with winner=0, should fail because the engine is not yet
    /// over after just two mulligan-keep actions. This is the
    /// WinnerMismatch / step-mismatch path.
    #[test]
    fn premature_game_end_yields_winner_mismatch() {
        let recording = crate::recording::RecordingV1 {
            start: crate::recording::GameStart {
                v: 1,
                game_id: "test".into(),
                timestamp: "t".into(),
                my_player_id: 0,
                first_player: None,
                is_ai: true,
                my_deck_post_shuffle: micro_deck(),
                opp_deck_post_shuffle: Some(micro_deck()),
                opp_decklist_composition: None,
            },
            rows: vec![
                Row::Action(crate::recording::ActionRow {
                    step: 0,
                    actor: 0,
                    action_id: 0, // mulligan keep
                    phase: "Mulligan".into(),
                    source: "mulligan".into(),
                }),
                Row::Action(crate::recording::ActionRow {
                    step: 1,
                    actor: 1,
                    action_id: 0,
                    phase: "Mulligan".into(),
                    source: "mulligan".into(),
                }),
            ],
            end: crate::recording::GameEnd {
                winner: 0,
                reason: "win".into(),
                total_steps: 2,
            },
        };
        let db = micro_card_db();
        let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
        // The engine has no winner after just mulligan-keep×2 — so winner
        // mismatch is the expected outcome (recording says 0, engine says
        // -1 = no winner).
        match outcome {
            ReplayOutcome::Fail(ReplayFail::WinnerMismatch(wm)) => {
                assert_eq!(wm.expected_winner, 0);
                assert_eq!(wm.engine_winner, -1);
            }
            other => panic!("expected WinnerMismatch, got {:?}", other),
        }
    }

    /// An illegal action (e.g. trying to play an out-of-range card) is
    /// surfaced as IllegalAction with the actual mask sampled for diagnosis.
    #[test]
    fn illegal_action_yields_illegal_action_failure() {
        let recording = crate::recording::RecordingV1 {
            start: crate::recording::GameStart {
                v: 1,
                game_id: "test".into(),
                timestamp: "t".into(),
                my_player_id: 0,
                first_player: None,
                is_ai: true,
                my_deck_post_shuffle: micro_deck(),
                opp_deck_post_shuffle: Some(micro_deck()),
                opp_decklist_composition: None,
            },
            rows: vec![
                // First mulligan decision: action_id 999 is wildly out of
                // range for the Mulligan phase (legal IDs are 0 and 1).
                Row::Action(crate::recording::ActionRow {
                    step: 0,
                    actor: 0,
                    action_id: 999,
                    phase: "Mulligan".into(),
                    source: "mulligan".into(),
                }),
            ],
            end: crate::recording::GameEnd {
                winner: 0,
                reason: "win".into(),
                total_steps: 1,
            },
        };
        let db = micro_card_db();
        let outcome = replay_recording(&recording, &db, &ReplayConfig::default());
        match outcome {
            ReplayOutcome::Fail(ReplayFail::IllegalAction(ia)) => {
                assert_eq!(ia.action_id, 999);
                assert_eq!(ia.actor, 0);
                // The engine should at least have action 0 (mulligan keep) legal.
                assert!(
                    ia.sample_legal_ids.contains(&0),
                    "expected 0 (mulligan keep) in sampled legal ids, got {:?}",
                    ia.sample_legal_ids
                );
            }
            other => panic!("expected IllegalAction, got {:?}", other),
        }
    }
}
