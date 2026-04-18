//! Turn-lifecycle + phase-transition test binary — covers engine
//! initialization, mulligan, first-turn draw, memory seesaw, digivolve
//! action decoding, and end-of-turn transitions.

mod digivolve_action;
mod end_turn_transition;
mod engine_core;
mod first_turn_draw;
mod memory_semantics;
mod mulligan;
