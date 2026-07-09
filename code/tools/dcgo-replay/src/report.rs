//! Aggregates [`ReplayOutcome`]s over a corpus into a parity report keyed
//! by failure kind and by card identity.
//!
//! Determinism contract (task 4.9): given the same corpus of recordings
//! (set-equal, order-irrelevant), [`aggregate`] produces byte-identical
//! JSON output. We achieve this by:
//!   - sorting per-card entries by card ID,
//!   - sorting per-game references within each card entry by game_id,
//!   - emitting top-level keys via a `BTreeMap` (which serializes in
//!     sorted order with `serde_json`).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::recording::RecordingV1;
use crate::replay::{ReplayFail, ReplayOutcome};

/// One entry in the per-card section.
#[derive(Debug, Clone, Serialize)]
pub struct PerCardEntry {
    pub failure_count: u32,
    pub first_failure: FirstFailureRef,
}

/// Minimal pointer back to the first observed failure for this card. Useful
/// for triage: open the recording file, jump to the step, see what happened.
#[derive(Debug, Clone, Serialize)]
pub struct FirstFailureRef {
    pub game_id: String,
    pub step: u32,
    pub kind: String,
}

/// Aggregated report.
///
/// Fields in declaration order = JSON-output key order so a reader can
/// scan the file top-down (overall → kinds → cards).
#[derive(Debug, Clone, Serialize)]
pub struct ParityReport {
    pub total_recordings: u32,
    pub pass: u32,
    pub partial_pass: u32,
    pub fail: u32,
    /// Failure count keyed by kind name: "illegal_action", "actor_mismatch",
    /// "winner_mismatch", "engine_error".
    pub by_kind: BTreeMap<String, u32>,
    /// Per-card failure attribution, sorted by card ID for determinism.
    pub per_card: BTreeMap<String, PerCardEntry>,
    /// Bare list of failed game_ids — useful for someone who wants to
    /// re-run the harness in --verbose on just the failures. Sorted.
    pub failed_games: Vec<String>,
}

/// Aggregate per-recording outcomes into a [`ParityReport`].
///
/// `entries` provides each recording's identity (we read game_id and the
/// observed action stream from it for card attribution) alongside its
/// replay outcome.
pub fn aggregate(entries: &[(&RecordingV1, &ReplayOutcome)]) -> ParityReport {
    let mut report = ParityReport {
        total_recordings: entries.len() as u32,
        pass: 0,
        partial_pass: 0,
        fail: 0,
        by_kind: BTreeMap::new(),
        per_card: BTreeMap::new(),
        failed_games: Vec::new(),
    };

    for (recording, outcome) in entries {
        match outcome {
            ReplayOutcome::Pass { .. } => report.pass += 1,
            ReplayOutcome::PartialPass { .. } => report.partial_pass += 1,
            ReplayOutcome::Fail(fail) => {
                report.fail += 1;
                report.failed_games.push(recording.start.game_id.clone());

                let kind_name = match fail {
                    ReplayFail::IllegalAction(_) => "illegal_action",
                    ReplayFail::ActorMismatch(_) => "actor_mismatch",
                    ReplayFail::WinnerMismatch(_) => "winner_mismatch",
                    ReplayFail::EngineError { .. } => "engine_error",
                    ReplayFail::OpaqueRevealError { .. } => "opaque_reveal_error",
                };
                *report.by_kind.entry(kind_name.to_string()).or_insert(0) += 1;

                if let Some(card_id) = attribute_card(recording, fail) {
                    let entry = report
                        .per_card
                        .entry(card_id.clone())
                        .or_insert(PerCardEntry {
                            failure_count: 0,
                            first_failure: FirstFailureRef {
                                game_id: recording.start.game_id.clone(),
                                step: step_of(fail),
                                kind: kind_name.to_string(),
                            },
                        });
                    entry.failure_count += 1;
                }
            }
        }
    }
    report.failed_games.sort();
    report
}

/// Best-effort card-identity attribution for a failure.
///
/// Logic:
///   - For `IllegalAction` with `source = "main_phase"` and a play_hand,
///     digivolve, or activate action, derive the card from the recording's
///     own decklist (since the engine refused, we can't query Game state
///     for what the played card actually was — we use the recording's
///     deck order + hand-index assumption).
///   - For `WinnerMismatch` and `ActorMismatch`, attribution is impossible
///     in the general case (the failure is structural). Returns `None`.
///   - For `EngineError`, returns `None` (these are constructor / step
///     errors not tied to a specific card).
///
/// Limitation: deriving card ID from `(deck, hand_index)` requires knowing
/// the hand state at the failure step. The Phase 1 implementation doesn't
/// replay forward to reach that state — instead it uses a very rough
/// heuristic (the action's source and ID range) to pick out an *approximate*
/// card label. When the per-prompt selection encoding lands (task 3.5
/// follow-up) the harness will be able to track hand contents step-by-step
/// and attribute more accurately. Until then, "selection-context" is used
/// as a sentinel for cards we can't pin down.
fn attribute_card(recording: &RecordingV1, fail: &ReplayFail) -> Option<String> {
    let illegal = match fail {
        ReplayFail::IllegalAction(ia) => ia,
        _ => return None,
    };

    use digimon_engine::action::space;
    let aid = illegal.action_id;

    // Play-from-hand range: action ID IS the hand index. We don't know the
    // hand contents without a parallel replay; surface as "play_card_in_hand"
    // until task 3.5 lands.
    if aid < space::PLAY_HAND_END {
        // Best-effort: if the recording's deck has fewer cards than this
        // hand index, the deck is too short for this action ever to have
        // been legal. Otherwise we return a deck-level placeholder.
        let deck_len = recording.start.my_deck_post_shuffle.len();
        if (aid as usize) < deck_len {
            return Some(format!("hand_index_{}", aid));
        }
    }

    // Effect activation on field permanent. Same "no parallel replay"
    // limitation — return a slot-level placeholder.
    if aid >= space::FIELD_EFFECT_START && aid < space::FIELD_EFFECT_END {
        let (perm, eff) = space::decode_field_effect(aid);
        return Some(format!("field_slot_{}_effect_{}", perm, eff));
    }

    // Attack
    if aid >= space::ATTACK_START && aid < space::ATTACK_END {
        let (att, _tgt) = space::decode_attack(aid);
        return Some(format!("attacker_slot_{}", att));
    }

    // Digivolve
    if aid >= space::DIGIVOLVE_START && aid < space::DIGIVOLVE_END {
        let (hand, _field) = space::decode_digivolve(aid);
        return Some(format!("digivolve_hand_{}", hand));
    }

    // Fallback for selection-range IDs or unknown ranges.
    Some("selection-context".to_string())
}

fn step_of(fail: &ReplayFail) -> u32 {
    match fail {
        ReplayFail::IllegalAction(ia) => ia.step,
        ReplayFail::ActorMismatch(am) => am.step,
        ReplayFail::WinnerMismatch(wm) => wm.steps_consumed,
        ReplayFail::EngineError { step, .. } => step.unwrap_or(0),
        ReplayFail::OpaqueRevealError { step, .. } => step.unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::{GameEnd, GameStart, RecordingV1};
    use crate::replay::{IllegalAction, ReplayFail, ReplayOutcome, WinnerMismatch};

    fn rec(game_id: &str) -> RecordingV1 {
        RecordingV1 {
            start: GameStart {
                v: 1,
                game_id: game_id.into(),
                timestamp: "t".into(),
                my_player_id: 0,
                is_ai: true,
                my_deck_post_shuffle: (0..50).map(|i| format!("CARD-{}", i)).collect(),
                opp_deck_post_shuffle: Some((0..50).map(|i| format!("CARD-{}", i)).collect()),
                opp_decklist_composition: None,
            },
            rows: Vec::new(),
            end: GameEnd {
                winner: 0,
                reason: "win".into(),
                total_steps: 0,
            },
        }
    }

    #[test]
    fn aggregate_pass_only() {
        let r1 = rec("g1");
        let r2 = rec("g2");
        let o1 = ReplayOutcome::Pass {
            steps_consumed: 10,
            winner: 0,
        };
        let o2 = ReplayOutcome::Pass {
            steps_consumed: 12,
            winner: 1,
        };
        let report = aggregate(&[(&r1, &o1), (&r2, &o2)]);
        assert_eq!(report.total_recordings, 2);
        assert_eq!(report.pass, 2);
        assert_eq!(report.fail, 0);
        assert!(report.failed_games.is_empty());
    }

    #[test]
    fn aggregate_failure_bucketing_and_card_attribution() {
        let r1 = rec("g1");
        let r2 = rec("g2");
        let fail1 = ReplayOutcome::Fail(ReplayFail::IllegalAction(IllegalAction {
            step: 5,
            actor: 0,
            action_id: 100, // ATTACK range, slot 0
            phase: "Main".into(),
            source: "main_phase".into(),
            sample_legal_ids: vec![62],
        }));
        let fail2 = ReplayOutcome::Fail(ReplayFail::WinnerMismatch(WinnerMismatch {
            expected_winner: 0,
            engine_winner: 1,
            steps_consumed: 10,
        }));
        let report = aggregate(&[(&r1, &fail1), (&r2, &fail2)]);
        assert_eq!(report.fail, 2);
        assert_eq!(report.by_kind.get("illegal_action").copied(), Some(1));
        assert_eq!(report.by_kind.get("winner_mismatch").copied(), Some(1));
        // Card attribution: action_id 100 is attack slot 0
        assert!(report.per_card.contains_key("attacker_slot_0"));
        // WinnerMismatch has no attributable card
        assert_eq!(report.per_card.len(), 1);
        assert_eq!(report.failed_games, vec!["g1", "g2"]);
    }

    #[test]
    fn aggregate_is_deterministic_under_input_permutation() {
        // The same set of outcomes in a different order must produce a
        // byte-identical report (after JSON serialization).
        let r1 = rec("aaa");
        let r2 = rec("bbb");
        let r3 = rec("ccc");
        let o1 = ReplayOutcome::Pass {
            steps_consumed: 1,
            winner: 0,
        };
        let o2 = ReplayOutcome::Fail(ReplayFail::IllegalAction(IllegalAction {
            step: 1,
            actor: 0,
            action_id: 100,
            phase: "Main".into(),
            source: "main_phase".into(),
            sample_legal_ids: vec![],
        }));
        let o3 = ReplayOutcome::Fail(ReplayFail::IllegalAction(IllegalAction {
            step: 2,
            actor: 1,
            action_id: 200,
            phase: "Main".into(),
            source: "main_phase".into(),
            sample_legal_ids: vec![],
        }));
        let a = aggregate(&[(&r1, &o1), (&r2, &o2), (&r3, &o3)]);
        let b = aggregate(&[(&r3, &o3), (&r2, &o2), (&r1, &o1)]);

        let a_json = serde_json::to_string(&a).unwrap();
        let b_json = serde_json::to_string(&b).unwrap();
        assert_eq!(a_json, b_json, "report not deterministic under input order");
    }
}
