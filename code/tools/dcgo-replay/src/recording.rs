//! DCGO recording JSONL schema (version 1) and parser.
//!
//! The parser now lives in the engine (`digimon_engine::dcgo_recording`) so the
//! `dcgo-replay` batch tool and the engine's interactive `DcgoAdapter` build
//! from one code path. This module re-exports it unchanged for back-compat;
//! existing `crate::recording::*` paths keep working.

pub use digimon_engine::dcgo_recording::*;
