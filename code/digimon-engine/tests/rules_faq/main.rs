//! Rules-FAQ faithfulness suite.
//!
//! Reproduces the official **General Rules/FAQ**
//! (digimoncardgame.fandom.com/wiki/General_Rules/FAQ — sourced from official
//! Carddass/Bandai Q&A) as behavioral tests against the Rust engine, using
//! REAL cards already implemented in the DSL. Each test asserts the
//! FAQ-correct outcome — never the engine's current output. A failing test is
//! a *discovered* faithfulness gap to be logged to
//! `qa/archetype-qa/engine-gaps.md` (discover-then-pin), NOT a weakened
//! assertion or a silent `#[ignore]`.
//!
//! Source priority for any rules dispute (CLAUDE.md): `general_rule.pdf`
//! (canonical) > base-repo `DCGO/` C# > the FAQ text itself.
//!
//! Authoritative per-item ledger (surface triage, reused-card vehicles,
//! verdicts): `qa/qa-reports/rules-faq.md`. OpenSpec change:
//! `openspec/changes/add-rules-faq-faithfulness-suite/`.
//!
//! Modules mirror the FAQ's own section taxonomy:
//!   deck_creation · phases (unsuspend/draw/breeding) · main_phase ·
//!   effect_resolution · keyword_identity · multicolor · no_level_no_value ·
//!   security_digimon · in_its_text.

#![allow(dead_code, unused_imports)]

mod loader;

mod deck_creation;
mod effect_resolution;
mod in_its_text;
mod keyword_identity;
mod main_phase;
mod multicolor;
mod no_level_no_value;
mod phases;
mod security_digimon;
