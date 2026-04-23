//! DSL → engine lowering. Phase 1c: declarative clauses only.
//!
//! `DslCardEffect` wraps a `digimon_dsl::CompiledCard` and emits engine
//! `Effect`s at `effects()` time. Triggered clauses, identity, alt_paths,
//! and raw_rust are skipped in Phase 1c (Phase 2 owns them).

pub mod bindings;
pub mod lower_aura;
pub mod lower_cost_reduction;
pub mod lower_flood_gate;
pub mod lower_grant_keyword;
pub mod lower_replacement;
pub mod lower_triggered;
pub mod modifier_map;
pub mod predicate;
pub mod step;
pub mod timing_map;
pub mod trigger_map;

use std::sync::Arc;

use digimon_dsl::compiled::CompiledCard;
use digimon_dsl::CardRegistry as DslCardRegistry;

use crate::card_source::CardHandle;
use crate::cards::CardEffectRegistry;
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
                CompiledClause::Triggered(clause) => {
                    out.extend(lower_triggered::lower(card, clause));
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
                    CompiledDeclarativeClause::Replacement {
                        scope,
                        active_when,
                        trigger,
                        process,
                        ..
                    } => {
                        if let Some(e) = lower_replacement::lower(
                            card,
                            *scope,
                            active_when.clone(),
                            trigger,
                            process,
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

/// Register every card in `dsl_registry` into `effect_registry` as a
/// `DslCardEffect`. Existing entries (e.g. hand-written TEST-* cards)
/// with the same `card_id` are replaced — DSL is authoritative once a
/// card migrates (CLAUDE.md rule 21: cards migrate one direction only,
/// Python → Rust → DSL).
pub fn register_dsl_cards(
    effect_registry: &mut CardEffectRegistry,
    dsl_registry: &DslCardRegistry,
) {
    for (card_id, compiled) in dsl_registry.iter() {
        let dsl_effect = Arc::new(DslCardEffect::new(Arc::new(compiled.clone())));
        effect_registry.insert(card_id, dsl_effect);
    }
}
