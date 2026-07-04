//! Game-state determinization tests (add-determinized-search Phase 1).
//!
//! Covers the three requirements in
//! `openspec/changes/add-determinized-search/specs/game-state-determinization/spec.md`:
//!
//! 1. Infoset extraction never exposes hidden information (counts + pins +
//!    deck prior only for the opponent; the viewer's own hidden model is one
//!    JOINT unseen pool — own deck ∪ own face-down security — because own
//!    security is hidden from its owner too).
//! 2. Sampled worlds are consistent and materialized (per-zone counts, copy
//!    limits, pins honored; `reveal_source = None`; fully playable).
//! 3. Round-trip law: re-extracting the infoset from a sampled world equals
//!    the source infoset for the same viewer.
//!
//! Run: RUST_MIN_STACK=268435456 cargo test --test determinization

mod support;

mod infoset_tests;
mod invariants_tests;
mod materialize_tests;
mod sampling_tests;
