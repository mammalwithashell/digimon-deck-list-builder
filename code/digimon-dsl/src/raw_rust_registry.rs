//! `RawRustRegistry` — resolves raw_rust fn names referenced by cards.
//!
//! Phase 0 ships the trait and a [`StubRegistry`] for tests. The real
//! registry (populated by `register_all()` on the engine crate) lands in
//! Phase 4 per spec §6.

use std::collections::HashSet;

pub trait RawRustRegistry: Send + Sync {
    fn contains_fn(&self, name: &str) -> bool;
}

#[derive(Debug, Default)]
pub struct StubRegistry {
    names: HashSet<String>,
}

impl StubRegistry {
    pub fn empty() -> Self { Self::default() }

    pub fn with<I: IntoIterator<Item = &'static str>>(names: I) -> Self {
        Self {
            names: names.into_iter().map(String::from).collect(),
        }
    }
}

impl RawRustRegistry for StubRegistry {
    fn contains_fn(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}
