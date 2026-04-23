//! DSL → engine lowering. Phase 1c: declarative clauses only.
//!
//! `DslCardEffect` wraps a `digimon_dsl::CompiledCard` and emits engine
//! `Effect`s at `effects()` time. Triggered clauses, identity, alt_paths,
//! and raw_rust are skipped in Phase 1c (Phase 2 owns them).

pub mod lower_aura;
pub mod lower_cost_reduction;
pub mod lower_flood_gate;
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

    /// Return the ACE overflow threshold for this card, if any.
    /// Engine ACE integration reads this value to check whether a
    /// Digimon can overflow memory during an attack.
    pub fn ace_overflow(&self) -> Option<i32> {
        self.compiled.ace_overflow
    }
}

impl CardEffect for DslCardEffect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};

        let mut out = Vec::new();
        'clause: for clause in &self.compiled.effects {
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
                    CompiledDeclarativeClause::CostReduction {
                        scope,
                        active_when,
                        reduction_timing,
                        when_playing_this,
                        when_any_ally_played,
                        condition,
                        once_per_turn,
                        amount,
                        amount_fn,
                        pay_cost,
                        ..
                    } => {
                        // Phase 1c scope: only when_playing_this + literal amount
                        // + no pay_cost + no ally-played hook + before_pay_cost timing.
                        let timing_ok = matches!(
                            reduction_timing.as_deref(),
                            None | Some("before_pay_cost")
                        );
                        if !timing_ok {
                            continue 'clause;
                        }
                        if !*when_playing_this {
                            continue 'clause;
                        }
                        if when_any_ally_played.is_some() {
                            continue 'clause;
                        }
                        if amount_fn.is_some() {
                            continue 'clause;
                        }
                        if !pay_cost.is_empty() {
                            continue 'clause;
                        }
                        if let Some(a) = *amount {
                            out.push(lower_cost_reduction::lower(
                                card,
                                *scope,
                                active_when.clone(),
                                condition.clone(),
                                *once_per_turn,
                                a,
                            ));
                        }
                    }
                    CompiledDeclarativeClause::FloodGate {
                        scope,
                        active_when,
                        modifier,
                        target,
                        ..
                    } => {
                        if let Some(e) = lower_flood_gate::lower(
                            card,
                            *scope,
                            active_when.clone(),
                            modifier,
                            target.clone(),
                        ) {
                            out.push(e);
                        }
                    }
                    _ => {
                        // Other declarative clauses lowered in Tasks 7-8+.
                    }
                },
            }
        }
        out
    }
}
