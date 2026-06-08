//! Tests for the FFI-facing surfaces (PendingSelectionView, GameEvent,
//! to_ui_json, GameRecorder). Exercised by the PyO3 crate via
//! `digimon-engine-py`, but the Rust half is validated here so failures
//! don't have to travel through Python to be diagnosed.

mod events;
mod perm_inspector;
mod recorder;
mod selection_view;
mod ui_json;
