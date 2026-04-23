//! DSL → engine lowering. Phase 1c: declarative clauses only.
//!
//! `DslCardEffect` wraps a `digimon_dsl::CompiledCard` and emits engine
//! `Effect`s at `effects()` time. Triggered clauses, identity, alt_paths,
//! and raw_rust are skipped in Phase 1c (Phase 2 owns them).

pub mod modifier_map;
pub mod predicate;

use std::sync::Arc;

use digimon_dsl::compiled::CompiledCard;

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct DslCardEffect {
    compiled: Arc<CompiledCard>,
}

impl DslCardEffect {
    pub fn new(compiled: Arc<CompiledCard>) -> Self {
        Self { compiled }
    }

    pub fn compiled(&self) -> &CompiledCard {
        &self.compiled
    }
}

impl CardEffect for DslCardEffect {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        // Phase 1c: declarative clauses only. Empty for now; per-clause
        // lowering lands in Tasks 4-8.
        Vec::new()
    }
}
