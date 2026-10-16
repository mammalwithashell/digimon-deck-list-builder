//! Rules-conformance suite: tests that pin engine behaviour to numbered
//! clauses of the Comprehensive Rules, via the fingerprinted `cite!` macro.
//!
//! The denominator is `qa/rules/cr_obligations.json` (embedded in `cite.rs`).
//! `#[macro_use]` on `cite` must stay first so sibling chapter modules can use
//! `cite!` (a test crate cannot `#[macro_export]`).

#[macro_use]
mod cite;
mod meta;
