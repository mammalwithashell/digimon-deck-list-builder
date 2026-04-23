//! DSL → engine lowering. Phase 1c: declarative clauses only.
//!
//! `DslCardEffect` wraps a `digimon_dsl::CompiledCard` and emits engine
//! `Effect`s at `effects()` time. Triggered clauses, identity, alt_paths,
//! and raw_rust are skipped in Phase 1c (Phase 2 owns them).

pub mod lower_aura;
pub mod lower_grant_keyword;
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
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};

        let mut out = Vec::new();
        for clause in &self.compiled.effects {
            match clause {
                CompiledClause::Triggered(_) => {
                    // Phase 1c: triggered clauses are not lowered.
                }
                CompiledClause::Declarative(decl) => match decl {
                    CompiledDeclarativeClause::GrantKeyword {
                        keyword, value, scope, ..
                    } => {
                        if let Some(e) =
                            lower_grant_keyword::lower(card, keyword, *value, *scope)
                        {
                            out.push(e);
                        }
                    }
                    CompiledDeclarativeClause::Aura {
                        scope,
                        active_when,
                        target,
                        dp_modifier,
                        grant_keyword,
                        ..
                    } => {
                        if let Some(e) = lower_aura::lower(
                            card,
                            *scope,
                            active_when.clone(),
                            target.clone(),
                            *dp_modifier,
                            grant_keyword.clone(),
                        ) {
                            out.push(e);
                        }
                    }
                    _ => {
                        // Other declarative clauses lowered in Tasks 7-8.
                    }
                },
            }
        }
        out
    }
}
