//! Engine-side raw_rust dispatch registry. Holds two maps of Arc-wrapped
//! closures: step-level (`fn(&mut EffectContext, &mut Bindings)`) and
//! whole-clause declarative (`fn(CardHandle) -> Vec<Effect>`). Card scripts
//! reference entries by string name; unregistered names become no-ops.
//!
//! Step fns receive the same `&mut Bindings` the DSL step dispatcher uses,
//! so raw_rust implementations can read/write the named-binding environment
//! built up by preceding DSL steps (e.g. chosen-target permanents). This
//! keeps raw_rust feature-parallel with declarative steps.
//!
//! Registering the same `name` twice overwrites the prior entry and emits a
//! `eprintln!` warning so accidental clashes between archetypes surface at
//! load time instead of silently swallowing one of the implementations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

pub type RawStepFn =
    Arc<dyn for<'a> Fn(&mut EffectContext<'a>, &mut Bindings) + Send + Sync + 'static>;
pub type RawDeclarativeFn = Arc<dyn Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static>;

#[derive(Default)]
pub struct EngineRawRustRegistry {
    steps: HashMap<String, RawStepFn>,
    declaratives: HashMap<String, RawDeclarativeFn>,
}

impl EngineRawRustRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a step-level raw_rust fn. Warns to stderr if `name` was
    /// already registered — the prior entry is dropped and the new one wins,
    /// but the clash is surfaced so configuration mistakes (e.g. two
    /// archetypes accidentally sharing a fn_name) don't silently hide one
    /// implementation.
    pub fn register_step<F>(&mut self, name: &str, f: F)
    where
        F: for<'a> Fn(&mut EffectContext<'a>, &mut Bindings) + Send + Sync + 'static,
    {
        if self.steps.insert(name.to_string(), Arc::new(f)).is_some() {
            eprintln!(
                "WARNING: raw_rust step fn '{name}' re-registered; prior entry overwritten"
            );
        }
    }

    /// Register a whole-clause raw_rust fn. Warns to stderr on duplicate
    /// registration — see [`register_step`] for rationale.
    pub fn register_declarative<F>(&mut self, name: &str, f: F)
    where
        F: Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static,
    {
        if self
            .declaratives
            .insert(name.to_string(), Arc::new(f))
            .is_some()
        {
            eprintln!(
                "WARNING: raw_rust declarative fn '{name}' re-registered; prior entry overwritten"
            );
        }
    }

    pub fn step_fn(&self, name: &str) -> Option<RawStepFn> {
        self.steps.get(name).cloned()
    }

    pub fn declarative_fn(&self, name: &str) -> Option<RawDeclarativeFn> {
        self.declaratives.get(name).cloned()
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn declarative_count(&self) -> usize {
        self.declaratives.len()
    }
}

impl std::fmt::Debug for EngineRawRustRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineRawRustRegistry")
            .field("steps", &self.steps.len())
            .field("declaratives", &self.declaratives.len())
            .finish()
    }
}

/// Returns true when the raw_rust budget (step + declarative fn count) exceeds
/// 3% of the card pool. `card_count == 0` returns false (nothing to ratio against).
pub fn exceeds_budget(raw_fn_count: usize, card_count: usize) -> bool {
    if card_count == 0 {
        return false;
    }
    (raw_fn_count as f32) / (card_count as f32) > 0.03
}
