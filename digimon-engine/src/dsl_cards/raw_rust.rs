//! Engine-side raw_rust dispatch registry. Holds two maps of Arc-wrapped
//! closures: step-level (`fn(&mut EffectContext)`) and whole-clause
//! declarative (`fn(CardHandle) -> Vec<Effect>`). Card scripts reference
//! entries by string name; unregistered names become no-ops.

use std::collections::HashMap;
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

pub type RawStepFn = Arc<dyn for<'a> Fn(&mut EffectContext<'a>) + Send + Sync + 'static>;
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

    pub fn register_step<F>(&mut self, name: &str, f: F)
    where
        F: for<'a> Fn(&mut EffectContext<'a>) + Send + Sync + 'static,
    {
        self.steps.insert(name.to_string(), Arc::new(f));
    }

    pub fn register_declarative<F>(&mut self, name: &str, f: F)
    where
        F: Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static,
    {
        self.declaratives.insert(name.to_string(), Arc::new(f));
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
