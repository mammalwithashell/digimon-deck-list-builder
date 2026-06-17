//! DSL → engine lowering. Phase 1c: declarative clauses only.
//!
//! `DslCardEffect` wraps a `digimon_dsl::CompiledCard` and emits engine
//! `Effect`s at `effects()` time. Triggered clauses, identity, alt_paths,
//! and raw_rust are skipped in Phase 1c (Phase 2 owns them).

pub mod binding_ref;
pub mod bindings;
pub mod expiry_map;
pub mod formula_eval;
pub mod formula_registry;
pub mod lower_alt_path_registration;
pub mod lower_aura;
pub mod lower_cost_reduction;
pub mod lower_delay;
pub mod lower_flood_gate;
pub mod lower_grant_keyword;
pub mod lower_partition;
pub mod lower_replacement;
pub mod lower_triggered;
pub mod modifier_map;
pub mod predicate;
pub mod raw_rust;
pub mod step;
pub mod timing_map;
pub mod trigger_map;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledCard, CompiledCardKind, CompiledPredicate};
use digimon_dsl::CardRegistry as DslCardRegistry;

use crate::card_source::CardHandle;
use crate::cards::CardEffectRegistry;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect::{CardEffect, Effect};
use crate::enums::{Expiry, ModifierType};
use crate::modifiers::ModifierPayload;

pub struct DslCardEffect {
    compiled: Arc<CompiledCard>,
    raw: Arc<EngineRawRustRegistry>,
}

impl DslCardEffect {
    pub fn new(compiled: Arc<CompiledCard>) -> Self {
        Self::new_with_raw(compiled, Arc::new(EngineRawRustRegistry::new()))
    }

    pub fn new_with_raw(compiled: Arc<CompiledCard>, raw: Arc<EngineRawRustRegistry>) -> Self {
        Self { compiled, raw }
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
        use digimon_dsl::compiled::{
            CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
        };

        let mut out = Vec::new();
        if let Some(identity) = &self.compiled.identity {
            for source_alias in &identity.source_name_aliases {
                let level_lte = source_alias.level_lte;
                out.push(
                    Effect::declarative(card)
                        .name("Dynamic source name aliases")
                        .materializes_declarative_state()
                        .process(move |ctx| {
                            let Some(handle) = ctx.source_permanent else {
                                return;
                            };
                            ctx.add_declarative_modifier_with_payload(
                                handle,
                                ModifierType::SourceNameAliases,
                                0,
                                Expiry::Permanent,
                                ModifierPayload::SourceNameAliases { level_lte },
                            );
                        })
                        .build(),
                );
            }
        }
        for path in &self.compiled.alt_paths {
            if matches!(
                path.kind,
                CompiledAltPathKind::BurstDigivolve | CompiledAltPathKind::BlastDnaDigivolve
            ) {
                out.push(
                    Effect::declarative(card)
                        .name("Blast digivolve marker")
                        .blast_digivolve()
                        .build(),
                );
            }
        }
        let option_use_requirement = option_use_requirement_for_card(&self.compiled);
        'clause: for (clause_index, clause) in self.compiled.effects.iter().enumerate() {
            match clause {
                CompiledClause::Triggered(clause) => {
                    out.extend(lower_triggered::lower_for_kind_with_clause_index(
                        card,
                        clause,
                        self.raw.clone(),
                        option_use_requirement.clone(),
                        Some(self.compiled.kind),
                        clause_index,
                    ));
                }
                CompiledClause::Declarative(decl) => match decl {
                    CompiledDeclarativeClause::GrantKeyword {
                        keyword,
                        value,
                        scope,
                        active_when,
                        overclock_cost_filter,
                        ..
                    } => {
                        if let Some(e) = lower_grant_keyword::lower(
                            card,
                            keyword,
                            *value,
                            *scope,
                            active_when.clone(),
                            overclock_cost_filter.clone(),
                        ) {
                            out.push(e);
                        }
                    }
                    CompiledDeclarativeClause::Aura {
                        scope,
                        active_when,
                        target,
                        target_player,
                        dp_modifier,
                        dp_modifier_fn,
                        security_attack,
                        security_attack_fn,
                        grant_keyword,
                        modifier,
                        modifier_value,
                        modifier_name,
                        while_condition,
                        applies_to_opponent_security_dp,
                        applies_to_own_security_dp,
                        effect_immunity,
                        ..
                    } => {
                        for e in lower_aura::lower_all(
                            card,
                            *scope,
                            active_when.clone(),
                            target.clone(),
                            *target_player,
                            *dp_modifier,
                            dp_modifier_fn.clone(),
                            *security_attack,
                            security_attack_fn.clone(),
                            grant_keyword.clone(),
                            modifier.clone(),
                            *modifier_value,
                            modifier_name.clone(),
                            while_condition.clone(),
                            *applies_to_opponent_security_dp,
                            *applies_to_own_security_dp,
                            *effect_immunity,
                            self.raw.clone(),
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
                        when_any_ally_digivolves_into,
                        condition,
                        optional,
                        once_per_turn,
                        amount,
                        amount_fn,
                        pay_cost,
                        ..
                    } => {
                        let timing_ok =
                            matches!(reduction_timing.as_deref(), None | Some("before_pay_cost"));
                        if !timing_ok {
                            continue 'clause;
                        }
                        // Originally this short-circuit required either
                        // `when_playing_this` or `when_any_ally_played` so authors
                        // could not register an unscoped global reducer by accident.
                        // Phase 2 Track H expands the supported trigger surface to
                        // include "when this permanent is being digivolved into"
                        // via `cost_target: { ... }` + `source_is_cost_target_permanent`
                        // inside `active_when`. Authors using that pattern set
                        // neither flag, so the short-circuit must defer to
                        // `active_when` instead of dropping the clause silently.
                        // (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET.)
                        if !*when_playing_this
                            && when_any_ally_played.is_none()
                            && when_any_ally_digivolves_into.is_none()
                            && active_when.is_none()
                            && condition.is_none()
                        {
                            continue 'clause;
                        }
                        let amount_formula = amount_fn.clone().or_else(|| {
                            (*amount).map(digimon_dsl::compiled::CompiledFormula::Literal)
                        });
                        if let Some(amount_formula) = amount_formula {
                            out.push(lower_cost_reduction::lower_with_formula(
                                card,
                                *scope,
                                active_when.clone(),
                                condition.clone(),
                                *once_per_turn,
                                Some(amount_formula),
                                pay_cost.clone(),
                                self.raw.clone(),
                                *optional,
                                *when_playing_this,
                                when_any_ally_played.clone(),
                                when_any_ally_digivolves_into.clone(),
                            ));
                        }
                    }
                    CompiledDeclarativeClause::FloodGate {
                        scope,
                        active_when,
                        modifier,
                        target,
                        target_player,
                        expiry,
                        ..
                    } => {
                        // `scope: both` installs the flood gate from BOTH the
                        // active-top (face_up) and digivolution-source
                        // (inherited) positions — required for `target: self`
                        // locks that must hold whether the carrier is the top
                        // card or a source beneath a later digivolution
                        // (BT24-062 attack-target lock).
                        let scopes: &[CompiledScope] = match scope {
                            CompiledScope::Both => {
                                &[CompiledScope::FaceUp, CompiledScope::Inherited]
                            }
                            other => std::slice::from_ref(other),
                        };
                        for sc in scopes {
                            if let Some(e) = lower_flood_gate::lower(
                                card,
                                *sc,
                                active_when.clone(),
                                modifier,
                                target.clone(),
                                *target_player,
                                expiry.as_deref(),
                            ) {
                                out.push(e);
                            }
                        }
                    }
                    CompiledDeclarativeClause::Replacement {
                        scope,
                        active_when,
                        trigger,
                        optional,
                        once_per_turn,
                        process,
                        ..
                    } => {
                        if let Some(e) = lower_replacement::lower_with_raw(
                            card,
                            *scope,
                            active_when.as_ref(),
                            trigger,
                            *optional,
                            *once_per_turn,
                            process,
                            self.raw.clone(),
                        ) {
                            out.push(e);
                        }
                    }
                    CompiledDeclarativeClause::Partition {
                        scope,
                        active_when,
                        sources,
                        exclude_cause,
                        process,
                        ..
                    } => {
                        out.extend(lower_partition::lower_with_raw(
                            card,
                            *scope,
                            active_when.as_ref(),
                            sources,
                            exclude_cause,
                            process,
                            self.raw.clone(),
                        ));
                    }
                    CompiledDeclarativeClause::Delay {
                        scope,
                        active_when,
                        trigger,
                        process,
                        ..
                    } => {
                        out.push(lower_delay::lower_with_raw(
                            card,
                            *scope,
                            active_when.as_ref(),
                            *trigger,
                            process,
                            self.raw.clone(),
                        ));
                    }
                    CompiledDeclarativeClause::LinkRequirement {
                        cost,
                        filter,
                        summary,
                        ..
                    } => {
                        let filter = filter.clone();
                        let mut builder = Effect::on_play(card).link(*cost, move |ctx, host| {
                            eval_predicate(&filter, ctx, PredicateSubject::Permanent(host))
                        });
                        if let Some(summary) = summary {
                            builder = builder.name(summary);
                        }
                        out.push(builder.build());
                    }
                    CompiledDeclarativeClause::LinkCondition {
                        cost,
                        filter,
                        summary,
                        ..
                    } => {
                        // Shape-B Digimon self link-condition: static metadata at
                        // `EffectTiming::LinkCondition` carrying cost + host
                        // filter, read by `digimon_link_condition_targets`.
                        let filter = filter.clone();
                        let mut builder =
                            Effect::link_condition(card).link_host(*cost, move |ctx, host| {
                                eval_predicate(&filter, ctx, PredicateSubject::Permanent(host))
                            });
                        if let Some(summary) = summary {
                            builder = builder.name(summary);
                        }
                        out.push(builder.build());
                    }
                    CompiledDeclarativeClause::AltPathRegistration {
                        scope,
                        active_when,
                        trigger,
                        applies_to,
                        registers,
                        ..
                    } => {
                        if let Some(e) = lower_alt_path_registration::lower(
                            card,
                            *scope,
                            active_when.clone(),
                            *trigger,
                            applies_to.clone(),
                            registers.clone(),
                        ) {
                            out.push(e);
                        }
                    }
                    CompiledDeclarativeClause::RawRust { fn_name, .. } => {
                        if let Some(f) = self.raw.declarative_fn(fn_name) {
                            out.extend(f(card));
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

fn option_use_requirement_for_card(card: &CompiledCard) -> Option<Arc<CompiledPredicate>> {
    match card.kind {
        CompiledCardKind::Option => card
            .use_requirement
            .as_ref()
            .map(|predicate| Arc::new(predicate.clone())),
        CompiledCardKind::Dual => card.dual.as_ref().and_then(|dual| {
            dual.option
                .use_requirement
                .as_ref()
                .map(|predicate| Arc::new((**predicate).clone()))
        }),
        _ => None,
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
    register_dsl_cards_with_raw(
        effect_registry,
        dsl_registry,
        Arc::new(EngineRawRustRegistry::new()),
    );
}

pub fn register_dsl_cards_with_raw(
    effect_registry: &mut CardEffectRegistry,
    dsl_registry: &DslCardRegistry,
    raw: Arc<EngineRawRustRegistry>,
) {
    for (card_id, compiled) in dsl_registry.iter() {
        let dsl_effect = Arc::new(DslCardEffect::new_with_raw(
            Arc::new(compiled.clone()),
            raw.clone(),
        ));
        effect_registry.insert(card_id, dsl_effect);
    }
}
