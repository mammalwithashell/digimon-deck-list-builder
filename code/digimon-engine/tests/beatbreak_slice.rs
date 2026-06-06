//! Isolated test binary for the BT25 "beatbreak" slice.
//!
//! The shared `cards_behavioral` binary is under concurrent authoring by
//! sibling sessions whose WIP files do not always compile; this standalone
//! harness lets the beatbreak slice be TDD'd in isolation. It `#[path]`-includes
//! the same per-card test files the orchestrator wires into
//! `tests/cards_behavioral/bt25/`, plus the shared `dsl_card_data` support
//! module. Once the slice merges, these modules are exercised by the canonical
//! `cards_behavioral` binary too; this file is a transient development aid.

#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;

#[path = "cards_behavioral/bt25/bt25_088.rs"]
mod bt25_088;
#[path = "cards_behavioral/bt25/bt25_090.rs"]
mod bt25_090;
#[path = "cards_behavioral/bt25/bt25_035.rs"]
mod bt25_035;
#[path = "cards_behavioral/bt25/bt25_049.rs"]
mod bt25_049;
#[path = "cards_behavioral/bt25/bt25_081.rs"]
mod bt25_081;
#[path = "cards_behavioral/bt25/bt25_041.rs"]
mod bt25_041;
#[path = "cards_behavioral/bt25/bt25_057.rs"]
mod bt25_057;
