use std::collections::HashMap;
use std::sync::Arc;

use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub type FormulaExtensionFn = fn(&EffectContext<'_>, PermanentHandle) -> i32;

// digimon-dsl validates that a raw_rust name is allowed; the engine registry
// resolves allowed names into executable formula callbacks at runtime.
#[derive(Clone, Default, Debug)]
pub struct FormulaExtensionRegistry {
    entries: Arc<HashMap<String, FormulaExtensionFn>>,
}

impl FormulaExtensionRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_entries(
        entries: impl IntoIterator<Item = (&'static str, FormulaExtensionFn)>,
    ) -> Self {
        let entries = entries
            .into_iter()
            .map(|(name, f)| (name.to_string(), f))
            .collect();
        Self {
            entries: Arc::new(entries),
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn evaluate(
        &self,
        name: &str,
        ctx: &EffectContext<'_>,
        target: PermanentHandle,
    ) -> Option<i32> {
        self.entries.get(name).map(|f| f(ctx, target))
    }
}
