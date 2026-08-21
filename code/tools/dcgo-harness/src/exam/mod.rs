//! The DCGO card-clause **exam**.
//!
//! The exam asks one question per card clause: *does our Rust engine resolve
//! this clause the way DCGO does?* It answers it by running the **same
//! scripted line of play** through both engines and comparing a normalized
//! per-step state projection.
//!
//! A scenario is a small YAML file naming a card, the specific clause under
//! test, a seed, stacked decks, and a symbolic line of play (`hatch`, `play`,
//! `digivolve`, `attack`, `select`, …). The same file drives both sides:
//!
//! - **Our engine** — [`scenario`] parses the YAML, [`lower`] resolves each
//!   symbolic step to a concrete action id against the live action mask,
//!   [`adapter`] feeds those ids to the shared replay core as a
//!   `RecordingSource`, and [`projection`] captures the normalized state after
//!   every step.
//! - **DCGO** — the same script is handed to the scripted-input driver in the
//!   modded client, which emits the matching projection as a sidecar next to
//!   the JSONL recording; [`projection`] parses it back.
//!
//! [`differ`] aligns the two projections and reports the **first** divergence
//! (later ones are almost always downstream of it), and [`verdict`] turns that
//! into a per-clause pass/fail record.
//!
//! The remaining modules turn a clean run into something durable:
//! [`dcgo_pool`] resolves which cards DCGO actually implements (only those can
//! be examined), [`backfill`] writes confirmed oracle state into a scenario's
//! `assert:` block so the Unity-free CI job can re-check it, and [`drafter`]
//! renders that observation as a draft `cards_behavioral` test for a human to
//! review.
//!
//! The `pub use` block below is a convenience surface only. Each module
//! remains the owner of its own names — import from the module when you want
//! to be explicit, and note that nothing may be re-exported here under a name
//! that differs from the one its module gave it.

pub mod adapter;
pub mod backfill;
pub mod dcgo_pool;
pub mod differ;
pub mod drafter;
pub mod lower;
pub mod projection;
pub mod scenario;
pub mod verdict;

#[cfg(test)]
pub mod test_support;

pub use adapter::ScenarioAdapter;
pub use backfill::{backfill, backfill_from_diff, GENERATED_MARKER};
pub use dcgo_pool::has_dcgo_script;
pub use differ::{diff, DiffReport, FieldDiff, StepDivergence};
pub use drafter::{draft_test, Provenance};
pub use lower::{lower_step, LowerError};
pub use projection::{
    parse_sidecar, PermanentProjection, SeatProjection, StateProjection, KEYWORD_PROBES,
};
pub use scenario::{
    Assertion, Expect, Scenario, ScenarioDecks, ScenarioSeat, ScenarioStep, StepAction,
};
pub use verdict::{ClauseVerdict, Verdict, VerdictStore, VerdictSummary};
