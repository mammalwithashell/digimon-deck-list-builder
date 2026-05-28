//! `dcgo-replay` — replay DCGO JSONL recordings through `digimon-engine` and
//! validate parity.
//!
//! Phase 1 scope:
//!  - Consumes JSONL recordings produced by the DCGO mod's `GameRecorder`
//!    (see `DCGO/Assets/Scripts/Script/Recording/GameRecorder.cs`).
//!  - Replays the action stream through `HeadlessRunner`, asserting mask
//!    legality at each step and winner-match at game-end.
//!  - Reports failures as one of three structured kinds (illegal action,
//!    winner mismatch, actor mismatch). Engine errors surface as a fourth.
//!  - Aggregates failures across a corpus into a parity report keyed by
//!    failure kind and by card identity (best-effort attribution).
//!
//! Out of scope (Phase 2/3):
//!  - Opaque-opponent-deck recordings (`opp_deck_post_shuffle == null`).
//!    These require the engine's opaque-deck mode, which is in tasks 6.x.
//!  - Per-prompt selection encoding. The DCGO recorder currently emits
//!    `encoder_failure` rows for every selection (see task 3.5 fallback);
//!    the harness halts cleanly with a `PartialPass` when it hits one,
//!    reporting the step it stopped at.

pub mod recording;
pub mod replay;
pub mod report;

pub use recording::{
    parse_jsonl, GameEnd, GameStart, RecordingV1, Row, SchemaError, SUPPORTED_SCHEMA_VERSION,
};
pub use replay::{
    replay_recording, ActorMismatch, IllegalAction, ReplayConfig, ReplayFail, ReplayOutcome,
    WinnerMismatch,
};
pub use report::{aggregate, ParityReport, PerCardEntry};
