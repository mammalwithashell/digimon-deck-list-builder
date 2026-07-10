//! Runner / logger / recorder infrastructure test binary. Covers the
//! deterministic headless runner loop and the `GameLogger` /
//! `GameRecorder` trace pipelines.

mod determinism_guard;
#[path = "../support/dsl_card_data.rs"]
mod dsl_card_data;
mod headless_runner;
mod logger_recorder;
mod replay_corpus;
mod verification_digest;
mod verification_recording;
