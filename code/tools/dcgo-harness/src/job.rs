//! Job spec and result types, plus the directory protocol.
//!
//! A job is one JSON file whose *state is its directory*:
//! `jobs/` → (atomic rename) `claimed/` → `done/` or `failed/`.
//! Atomic rename is what makes crash recovery legible — a job sitting in
//! `claimed/` past its timeout is a hung Unity run, not a lost one.

use serde::{Deserialize, Serialize};

/// Card ID lists for both seats. Deliberately NOT DCGO deck codes: the code is
/// a base-n encoding over DCGO's internal `CEntity_Base.CardIndex`, so DCGO
/// owns the encoding and we send identities it resolves itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDecks {
    pub p0: Vec<String>,
    pub p1: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobLimits {
    /// Abandon the game past this many turns. Guards against a bot that loops
    /// forever; the partial recording is still filed and still useful.
    pub max_turns: u32,
    /// Wall-clock budget. The CLI (not Unity) enforces this by age of the
    /// claimed file, so a hung Unity process is still detected.
    pub timeout_seconds: u64,
}

/// One unattended DCGO game. Phase 1 only emits `policy: "ai"`; the field
/// exists so phase-2 scripted jobs share the reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub job_id: String,
    pub policy: String,
    pub decks: JobDecks,
    pub first_player: u8,
    /// Seeds `UnityEngine.Random.InitState` so the game is reproducible. A
    /// divergence found in game 137 of an overnight batch is worthless if it
    /// cannot be re-run.
    pub seed: u64,
    pub limits: JobLimits,
}

impl JobSpec {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOutcome {
    /// Game reached its natural end and the recording has a `game_end` row.
    Completed,
    /// Game was abandoned (turn cap); the recording is usable but truncated.
    Partial,
    /// DCGO could not run the job (bad deck, crash).
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub outcome: JobOutcome,
    pub recording_path: String,
    pub steps: u32,
    pub duration_seconds: f64,
    /// Failure detail. Empty on success.
    #[serde(default)]
    pub message: String,
}

impl JobResult {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

/// The four job directories, relative to the harness root.
pub const DIR_JOBS: &str = "jobs";
pub const DIR_CLAIMED: &str = "claimed";
pub const DIR_DONE: &str = "done";
pub const DIR_FAILED: &str = "failed";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_spec_round_trips_through_json() {
        let spec = JobSpec {
            job_id: "vol-0042".to_string(),
            policy: "ai".to_string(),
            decks: JobDecks {
                p0: vec!["EX12-035".to_string(), "EX12-001".to_string()],
                p1: vec!["BT16-082".to_string()],
            },
            first_player: 0,
            seed: 12345,
            limits: JobLimits {
                max_turns: 40,
                timeout_seconds: 180,
            },
        };
        let text = spec.to_json().expect("serialize");
        let back = JobSpec::from_json(&text).expect("deserialize");
        assert_eq!(back.job_id, "vol-0042");
        assert_eq!(back.seed, 12345);
        assert_eq!(back.decks.p0.len(), 2);
        assert_eq!(back.limits.max_turns, 40);
    }

    #[test]
    fn job_spec_tolerates_unknown_fields() {
        // Forward compatibility: a phase-2 job carrying `deck_order` must not
        // break a phase-1 reader.
        let text = r#"{
            "job_id": "vol-1",
            "policy": "ai",
            "decks": {"p0": ["EX12-035"], "p1": ["BT16-082"]},
            "first_player": 1,
            "seed": 7,
            "limits": {"max_turns": 40, "timeout_seconds": 180},
            "deck_order": {"p0": ["EX12-035"]},
            "dump_state": true
        }"#;
        let spec = JobSpec::from_json(text).expect("unknown fields must be ignored");
        assert_eq!(spec.first_player, 1);
        assert_eq!(spec.seed, 7);
    }

    #[test]
    fn job_result_parses_outcomes() {
        let text = r#"{
            "job_id": "vol-1",
            "outcome": "completed",
            "recording_path": "recordings/x.jsonl",
            "steps": 61,
            "duration_seconds": 12.5,
            "message": ""
        }"#;
        let result = JobResult::from_json(text).expect("parse");
        assert_eq!(result.outcome, JobOutcome::Completed);
        assert_eq!(result.steps, 61);
    }
}
