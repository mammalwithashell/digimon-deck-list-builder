//! Engine-side raw_rust runtime registry for DSL cards.
//!
//! The `digimon-dsl` crate validates that referenced function names exist in
//! a registry. This module is the engine runtime counterpart: it stores typed
//! function pointers used when a compiled DSL card reaches a `raw_rust`
//! clause, process step, or formula.

use std::collections::HashMap;
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::effect::Effect;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::permanent::PermanentHandle;

#[cfg(test)]
#[path = "raw_declarative_guard_tests.rs"]
mod declarative_guard_tests;

pub type RawStepFn = dyn Fn(&mut EffectContext<'_>, &mut Bindings) + Send + Sync + 'static;
pub type RawDeclarativeFn = dyn Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static;
pub type RawFormulaFn =
    dyn Fn(&EffectReadContext<'_>, PermanentHandle) -> i32 + Send + Sync + 'static;

#[derive(Clone, Default)]
pub struct EngineRawRustRegistry {
    steps: HashMap<String, Arc<RawStepFn>>,
    declaratives: HashMap<String, Arc<RawDeclarativeFn>>,
    formulas: HashMap<String, Arc<RawFormulaFn>>,
}

impl std::fmt::Debug for EngineRawRustRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineRawRustRegistry")
            .field("steps", &self.steps.len())
            .field("declaratives", &self.declaratives.len())
            .field("formulas", &self.formulas.len())
            .finish()
    }
}

impl EngineRawRustRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_step<F>(&mut self, name: impl Into<String>, f: F)
    where
        F: Fn(&mut EffectContext<'_>, &mut Bindings) + Send + Sync + 'static,
    {
        // Guard the stored callback rather than lookup: a caller can retain it
        // before a projected context is restored or the registry is cloned.
        self.steps.insert(name.into(), Arc::new(move |ctx, bindings| {
            crate::determinize::context_guard::check(ctx.game, "raw_dsl_step");
            f(ctx, bindings)
        }));
    }

    pub fn register_declarative<F>(&mut self, name: impl Into<String>, f: F)
    where
        F: Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static,
    {
        // Keep guards on returned callbacks even for prefetched factories,
        // cloned registries and direct DslCardEffect users. The factory itself
        // has no Game parameter: its execution/captures need a separate proof.
        self.declaratives.insert(name.into(), Arc::new(move |card| {
            let mut effects = f(card);
            for effect in &mut effects {
                crate::determinize::context_guard::guard_effect(effect);
            }
            effects
        }));
    }

    pub fn register_formula<F>(&mut self, name: impl Into<String>, f: F)
    where
        F: Fn(&EffectReadContext<'_>, PermanentHandle) -> i32 + Send + Sync + 'static,
    {
        self.formulas.insert(name.into(), Arc::new(move |ctx, target| {
            crate::determinize::context_guard::check(ctx.game, "raw_dsl_formula");
            f(ctx, target)
        }));
    }

    pub fn step_fn(&self, name: &str) -> Option<Arc<RawStepFn>> {
        self.steps.get(name).cloned()
    }

    pub fn declarative_fn(&self, name: &str) -> Option<Arc<RawDeclarativeFn>> {
        self.declaratives.get(name).cloned()
    }

    pub fn formula_fn(&self, name: &str) -> Option<Arc<RawFormulaFn>> {
        self.formulas.get(name).cloned()
    }

    pub fn contains_fn(&self, name: &str) -> bool {
        self.steps.contains_key(name)
            || self.declaratives.contains_key(name)
            || self.formulas.contains_key(name)
    }

    pub fn registered_fn_count(&self) -> usize {
        self.steps.len() + self.declaratives.len() + self.formulas.len()
    }
}

impl digimon_dsl::raw_rust_registry::RawRustRegistry for EngineRawRustRegistry {
    fn contains_fn(&self, name: &str) -> bool {
        self.contains_fn(name)
    }
}
