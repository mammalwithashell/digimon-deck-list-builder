//! Rules-conformance suite: tests that pin engine behaviour to numbered
//! clauses of the Comprehensive Rules, via the fingerprinted `cite!` macro.
//!
//! The denominator is `qa/rules/cr_obligations.json` (embedded in `cite.rs`).
//! `#[macro_use]` on `cite` must stay first so sibling chapter modules can use
//! `cite!` (a test crate cannot `#[macro_export]`).

#[macro_use]
mod cite;
mod meta;

// Shared YAML fixture-card loader. It lives in `fixtures/mod.rs` (a
// subdirectory), so `meta::chapter_source_list_is_complete` does not count it.
mod fixtures;

// Shared prompt-driving / board-reading helpers (infrastructure, no tests).
mod support;

mod ch15_04_effect_states;
mod ch15_07_optional_conditions;
mod ch15_08_effect_categories;
mod ch17_rule_checks;
mod ch18_pending_overwrite;
