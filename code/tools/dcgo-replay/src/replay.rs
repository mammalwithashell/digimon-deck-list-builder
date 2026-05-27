//! Replays a parsed [`RecordingV1`] through `digimon_engine::HeadlessRunner`
//! and validates parity at each step.
//!
//! Parity is defined as:
//!   1. The action stream is **legality-consistent**: every recorded
//!      `action_id` is set in the engine's action mask at the moment the
//!      engine expects that actor to decide.
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

use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::enums::PlayerId;
use digimon_engine::game::Game;
use digimon_engine::opaque_deck::{RevealKind, RevealQueue};
use digimon_engine::rules::Rules;

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
    Pass {
        steps_consumed: u32,
        winner: u8,
    },
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
    EngineError {
        step: Option<u32>,
        message: String,
    },
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
    OpaqueRevealError {
        step: Option<u32>,
        message: String,
    },
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

/// Replay one recording through the engine.
///
/// Bot-vs-bot recordings (`opp_deck_post_shuffle.is_some()`) construct a
/// standard `Game::new` with both decks ordered. PvP recordings
/// (`opp_deck_post_shuffle.is_none()`) construct via
/// `Game::new_with_opaque_opponent` with a `RevealQueue` preloaded from
/// the recording's `reveal` rows.
pub fn replay_recording(
    recording: &RecordingV1,
    card_data: &HashMap<String, CardData>,
    config: &ReplayConfig,
) -> ReplayOutcome {
    let mut game = match build_game(recording, card_data) {
        Ok(g) => g,
        Err(fail) => return ReplayOutcome::Fail(fail),
    };

    let mut step_count: u32 = 0;
    for row in &recording.rows {
        // Runaway guard.
        if step_count >= config.max_steps {
            return ReplayOutcome::Fail(ReplayFail::EngineError {
                step: Some(step_count),
                message: format!(
                    "max_steps ({}) exceeded; engine may be in an infinite loop or \
                     recording is unexpectedly long.",
                    config.max_steps
                ),
            });
        }

        match row {
            Row::Action(act) => {
                // Verify actor matches what the engine expects to decide.
                let expected_actor: u8 = current_decision_player(&game);
                if expected_actor != act.actor {
                    return ReplayOutcome::Fail(ReplayFail::ActorMismatch(ActorMismatch {
                        step: act.step,
                        expected_actor,
                        recorded_actor: act.actor,
                        action_id: act.action_id,
                        phase: act.phase.clone(),
                    }));
                }

                // Verify the action is legal under the current mask.
                let mask = build_action_mask(&game, expected_actor);
                let action_idx = act.action_id as usize;
                let legal = mask.get(action_idx).copied().unwrap_or(0.0) > 0.5;
                if !legal {
                    let sample = sample_legal_ids(&mask, 10);
                    return ReplayOutcome::Fail(ReplayFail::IllegalAction(IllegalAction {
                        step: act.step,
                        actor: act.actor,
                        action_id: act.action_id,
                        phase: act.phase.clone(),
                        source: act.source.clone(),
                        sample_legal_ids: sample,
                    }));
                }

                if config.verbose {
                    eprintln!(
                        "step {} actor {} action_id {} phase {} source {}",
                        act.step, act.actor, act.action_id, act.phase, act.source
                    );
                }
                game.decode_action(act.action_id, expected_actor);
                step_count += 1;
            }
            Row::EncoderFailure(ef) => {
                // We can't fabricate an action ID for this row — the engine
                // is still waiting for an unencoded selection. Halt cleanly.
                return ReplayOutcome::PartialPass {
                    steps_consumed: step_count,
                    stop_reason: format!(
                        "hit encoder_failure row at step {} ({}; raw={}). \
                         The engine cannot advance without the corresponding \
                         selection encoded. Resolve task 3.5 (per-prompt \
                         selection encoding) to replay past this point.",
                        ef.step, ef.reason, ef.raw_value
                    ),
                };
            }
            Row::Reveal(_) => {
                // Reveals are consumed by the engine via the preloaded
                // RevealQueue, NOT advanced by the harness loop. They're
                // present in the row stream for documentation / debugging.
                // No-op here.
            }
            Row::Unknown => {
                // Tolerated for forward compat — unknown row types are
                // skipped without aborting the replay.
            }
            Row::GameStart(_) | Row::GameEnd(_) => {
                // These can't appear mid-stream per parser invariants.
                // Defensive no-op.
            }
        }
    }

    // Final winner-match check.
    let engine_winner_raw: u8 = game.winner.unwrap_or(u8::MAX);
    let engine_winner_i8: i8 = if engine_winner_raw == u8::MAX {
        -1
    } else {
        engine_winner_raw as i8
    };
    if engine_winner_i8 != recording.end.winner {
        return ReplayOutcome::Fail(ReplayFail::WinnerMismatch(WinnerMismatch {
            expected_winner: recording.end.winner,
            engine_winner: engine_winner_i8,
            steps_consumed: step_count,
        }));
    }

    ReplayOutcome::Pass {
        steps_consumed: step_count,
        winner: engine_winner_raw,
    }
}

/// Dispatch on `opp_deck_post_shuffle` to construct either a standard
/// game or an opaque-opponent game. Pulled into a helper to keep
/// `replay_recording` focused on the loop.
fn build_game(
    recording: &RecordingV1,
    card_data: &HashMap<String, CardData>,
) -> Result<Game, ReplayFail> {
    let my_deck = recording.start.my_deck_post_shuffle.clone();
    let my_pid = recording.start.my_player_id;

    match &recording.start.opp_deck_post_shuffle {
        Some(opp_deck) => {
            // Standard mode — both decks fully ordered.
            let (deck1, deck2) = if my_pid == 0 {
                (my_deck, opp_deck.clone())
            } else {
                (opp_deck.clone(), my_deck)
            };
            // Seed = 0 is fine: the post-shuffle deck order is already
            // baked in; RNG only affects card-internal random effects.
            Game::new(&[deck1, deck2], card_data, Rules::standard(), Some(0)).map_err(|e| {
                ReplayFail::EngineError {
                    step: None,
                    message: format!("Game::new failed: {}", e),
                }
            })
        }
        None => {
            // Opaque mode — opponent's deck composition is known but order
            // isn't. Preload the RevealQueue from the recording's `reveal`
            // rows in stream order, then construct via the opaque path.
            let reveal_pairs = collect_reveal_pairs(&recording.rows).map_err(|msg| {
                ReplayFail::OpaqueRevealError {
                    step: None,
                    message: msg,
                }
            })?;
            let queue = RevealQueue::from_pairs(reveal_pairs);

            // The opaque opponent's decklist is the same shape as my_deck
            // but with composition known from the recording. We don't
            // have it explicitly in the schema — the recording's
            // `opp_deck_post_shuffle: null` means "I don't know the order,
            // but I know the composition". The composition comes from the
            // reveal stream's TOTAL set of cards, supplemented by any
            // cards the opp could be inferred to hold.
            //
            // Pragmatic Phase 3 approach: until the recorder is updated
            // to include an explicit `opp_decklist_composition` header
            // field, we error out. Capturing this is task 7.x.
            //
            // For Phase 1 integration testing we work around by allowing
            // recordings to optionally include a synthetic `opp_decklist`
            // field — but that's beyond the current schema. Surface a
            // clear error so the user knows what's missing.
            //
            // Fallback: if the reveal stream contains at least `deck_size`
            // distinct entries (e.g. a test fixture with hand-written
            // reveals for the entire deck), use them as the multiset.
            // This lets integration tests proceed without a schema bump.
            let opp_decklist =
                derive_opp_decklist_from_recording(recording).map_err(|msg| {
                    ReplayFail::OpaqueRevealError {
                        step: None,
                        message: msg,
                    }
                })?;

            Game::new_with_opaque_opponent(
                my_pid,
                my_deck,
                opp_decklist,
                Box::new(queue),
                card_data,
                Rules::standard(),
                Some(0),
            )
            .map_err(|e| ReplayFail::OpaqueRevealError {
                step: None,
                message: format!("Game::new_with_opaque_opponent failed: {}", e),
            })
        }
    }
}

/// Walk the recording's row stream collecting reveal rows in order,
/// mapping each `source` string to the corresponding [`RevealKind`].
/// Returns an error if any `source` value is unrecognized.
fn collect_reveal_pairs(rows: &[Row]) -> Result<Vec<(RevealKind, String)>, String> {
    let mut out = Vec::new();
    for row in rows {
        if let Row::Reveal(rv) = row {
            let kind = match rv.source.as_str() {
                "draw" => RevealKind::Draw,
                "security" => RevealKind::Security,
                "mill" => RevealKind::Mill,
                "effect" => RevealKind::Effect,
                other => {
                    return Err(format!(
                        "reveal row at step {} has unknown source `{}` \
                         (expected draw|security|mill|effect)",
                        rv.step, other
                    ));
                }
            };
            out.push((kind, rv.card_id.clone()));
        }
    }
    Ok(out)
}

/// Determine the opaque opponent's decklist composition.
///
/// Two sources, tried in order:
///
/// 1. **Explicit `opp_decklist_composition` header field** — added to
///    the schema in anticipation of task 7.x's recorder update. When the
///    DCGO mod knows the opponent's decklist (from the Photon
///    matchmaking handshake), it emits the full decklist here even
///    though `opp_deck_post_shuffle` stays `null`. This is the path real
///    recordings will use.
///
/// 2. **Reveal-stream fallback** — when the explicit field is absent
///    (older recordings, or hand-crafted ones). Uses the reveal stream's
///    card IDs as the multiset. Requires the reveal stream to provide
///    at least `expected_size` entries — bot tests can pad, real
///    recordings without explicit composition cannot.
fn derive_opp_decklist_from_recording(recording: &RecordingV1) -> Result<Vec<String>, String> {
    let expected_size = recording.start.my_deck_post_shuffle.len();

    // Preferred: explicit field.
    if let Some(comp) = &recording.start.opp_decklist_composition {
        if comp.len() != expected_size {
            return Err(format!(
                "opp_decklist_composition has {} cards but my_deck_post_shuffle \
                 has {}; both must match the rules' deck size",
                comp.len(),
                expected_size
            ));
        }
        return Ok(comp.clone());
    }

    // Fallback: derive from reveal stream.
    let reveal_cards: Vec<String> = recording
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Reveal(rv) => Some(rv.card_id.clone()),
            _ => None,
        })
        .collect();

    if reveal_cards.len() < expected_size {
        return Err(format!(
            "opaque recording is missing an explicit `opp_decklist_composition` \
             header field, and its reveal stream has only {} entries (fewer than \
             the deck size of {}). The recorder must supply either an explicit \
             composition or enough reveals to derive it.",
            reveal_cards.len(),
            expected_size
        ));
    }

    Ok(reveal_cards.into_iter().take(expected_size).collect())
}

/// Replicates `HeadlessRunner::current_decision_player` logic without
/// requiring the runner wrapper.
fn current_decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Return up to `max` indices of `mask` whose float value > 0.5. Used to
/// surface "what the engine WOULD have accepted" in IllegalAction reports.
fn sample_legal_ids(mask: &[f32], max: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(max);
    for (i, &v) in mask.iter().enumerate() {
        if v > 0.5 {
            out.push(i as u16);
            if out.len() >= max {
                break;
            }
        }
    }
    out
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
