//! Judge-quiz faithfulness suite.
//!
//! Reproduces the 30 scenarios of the TCG-Judges' Discord rules quiz
//! ("ZXavier's Digi Rulings") as behavioral tests using the REAL cards the
//! judges chose. Each test asserts the official judge-correct outcome — never
//! the engine's current output. A failing test is a *discovered* faithfulness
//! gap to be logged to `qa/archetype-qa/engine-gaps.md` (discover-then-pin),
//! NOT a weakened assertion or a silent `#[ignore]`.
//!
//! See `openspec/changes/add-judge-quiz-faithfulness-suite/` —
//! `card-resolution.md` is the authoritative per-question card-ID + board-state
//! + judge-answer table.
//!
//! Modules are organized by the rule cluster each scenario exercises:
//!   A immunity scope · B deferred rules-check · C declare-then-pay cost ·
//!   D trigger activation site · E `<Partition>`/DigiXros/de-digivolve ·
//!   F token lifecycle & memory arithmetic · G zone/keyword scoping.

#![allow(dead_code, unused_imports)]

mod loader;

mod a_immunity_scope;
mod b_deferred_rules_check;
mod c_declare_then_pay;
mod d_activation_site;
mod e_partition_digixros;
mod f_token_and_memory;
mod g_zone_keyword;
