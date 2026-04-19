//! Game-state-driven opponent policies.
//!
//! The ONNX inference path lives in `crate::inference` (loaded on demand
//! via `rust_load_model`). This module carries the deterministic
//! heuristic(s) that drive `PlayerKind::Greedy` and similar local-play
//! bots — no external model files, no `ort` dependency.

pub mod greedy;

pub use greedy::greedy_action;
