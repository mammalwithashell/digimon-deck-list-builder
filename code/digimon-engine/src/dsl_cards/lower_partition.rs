//! Lower `CompiledDeclarativeClause::Partition` into a declarative engine
//! `Effect` that grants `Keyword::Partition` to the source permanent.
//!
//! Phase 1 scope granted the `Keyword::Partition` keyword and installed a
//! declarative marker. Phase 3 reducer closeout applies the declarative
//! gates at grant time so inactive or cause/source-excluded Partition clauses
//! do not install the keyword.

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::Effect;
use crate::effect_context::EffectContext;
use crate::enums::{Expiry, Keyword};
use crate::replacement::ReplacementCause;

/// Lower a `Partition` declarative clause.
///
/// Returns an `Effect` that grants `Keyword::Partition` to the source
/// permanent when its declarative process fires.
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    sources: &[CompiledPredicate],
    exclude_cause: &[String],
) -> Effect {
    let active_when = active_when.cloned();
    let sources = sources.to_vec();
    let exclude_cause = exclude_cause
        .iter()
        .map(|cause| normalize_cause_key(cause))
        .collect::<Vec<_>>();

    let mut builder = Effect::declarative(card)
        .name("Partition")
        .process(move |ctx| {
            if !active_when_matches(active_when.as_ref(), ctx)
                || !source_predicates_match(&sources, ctx)
                || cause_is_excluded(&exclude_cause, ctx.deletion_cause())
            {
                return;
            }

            let Some(handle) = ctx.source_permanent else {
                return;
            };
            ctx.grant_keyword(handle, Keyword::Partition, Expiry::Permanent);
        });
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}

fn active_when_matches(active_when: Option<&CompiledPredicate>, ctx: &EffectContext<'_>) -> bool {
    let Some(predicate) = active_when else {
        return true;
    };
    let read = ctx.as_read();
    eval_predicate(predicate, &read, PredicateSubject::None)
}

fn source_predicates_match(sources: &[CompiledPredicate], ctx: &EffectContext<'_>) -> bool {
    if sources.is_empty() {
        return true;
    }

    let read = ctx.as_read();
    let Some(handle) = read.source_permanent else {
        return false;
    };
    let Some(permanent) = read
        .game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
    else {
        return false;
    };

    let material_count = permanent.card_sources.len().saturating_sub(1);
    sources.iter().all(|predicate| {
        permanent
            .card_sources
            .iter()
            .take(material_count)
            .any(|source| eval_predicate(predicate, &read, PredicateSubject::Card(source.handle())))
    })
}

fn cause_is_excluded(exclude_cause: &[String], cause: Option<ReplacementCause>) -> bool {
    let Some(cause) = cause else {
        return false;
    };
    let cause_key = normalize_cause_key(&format!("{cause:?}"));
    exclude_cause.iter().any(|excluded| excluded == &cause_key)
}

fn normalize_cause_key(raw: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;

    for (index, ch) in raw.trim().chars().enumerate() {
        if ch == '-' || ch == ' ' {
            if !normalized.is_empty() && !previous_was_separator {
                normalized.push('_');
                previous_was_separator = true;
            }
            continue;
        }

        if ch == '_' {
            if !normalized.is_empty() && !previous_was_separator {
                normalized.push('_');
                previous_was_separator = true;
            }
            continue;
        }

        if ch.is_ascii_uppercase() {
            if index > 0 && !previous_was_separator {
                normalized.push('_');
            }
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(ch.to_ascii_lowercase());
        }
        previous_was_separator = false;
    }

    normalized.trim_matches('_').to_string()
}
