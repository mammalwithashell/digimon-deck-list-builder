//! Action mutations on `EffectContext`, split by game mechanic.
//!
//! Each submodule holds `impl EffectContext` blocks for one mechanic.
//! `EffectContext` remains a single type; only its `impl` is split across
//! files for readability — the same technique used for `impl Game` across
//! 14 files and the `selections` sibling module.

mod combat;
mod digivolve;
mod digixros;
mod lifecycle;
mod memory;
mod modifiers;
mod play;
mod refire;
mod replacement;
mod scheduling;
mod security;
mod sources;
mod suspend;
mod trash;
mod zones;
