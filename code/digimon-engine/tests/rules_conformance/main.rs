//! Rules-conformance suite: tests that pin engine behaviour to numbered
//! clauses of the Comprehensive Rules, via the fingerprinted `cite!` macro.
//!
//! The denominator is `qa/rules/cr_obligations.json` (embedded in `cite.rs`).
//! `#[macro_use]` on `cite` must stay first so sibling chapter modules can use
//! `cite!` (a test crate cannot `#[macro_export]`).

/// mimalloc as this test binary's global allocator, exactly as
/// `tests/cards_behavioral/main.rs` does. With the Windows system heap the
/// target crawls at the default thread count under heap-lock contention
/// (CLAUDE.md rule 33): measured 741 s without mimalloc vs 15 s with it.
/// `#[global_allocator]` must live in the binary crate, and an integration-test
/// binary is its own crate, so it goes here.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[macro_use]
mod cite;
mod meta;

// Shared YAML fixture-card loader. It lives in `fixtures/mod.rs` (a
// subdirectory), so `meta::chapter_source_list_is_complete` does not count it.
mod fixtures;

// Shared prompt-driving / board-reading helpers (infrastructure, no tests).
mod support;

mod ch03_areas;
mod ch11_14_attack_procedure;
mod ch15_04_effect_states;
mod ch15_07_optional_conditions;
mod ch15_08_effect_categories;
mod ch15_15_unaffected;
mod ch16_attack_keywords;
mod ch16_new_keywords;
mod ch16_processing_keywords;
mod ch16_replacement_keywords;
mod ch17_rule_checks;
mod ch18_pending_overwrite;
