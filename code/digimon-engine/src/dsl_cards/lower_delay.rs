//! Lower `CompiledDeclarativeClause::Delay` into an engine `Effect` with
//! `timing == DelayEffect`.
//!
//! Phase 1 scope: map `CompiledTiming::EndOfYourTurn` →
//! `DelayTrigger::EndOfThisTurn`; all other timings default to
//! `DelayTrigger::EndOfYourNextTurn`.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{DelayTrigger, EffectTiming};

/// Lower a `Delay` declarative clause.
///
/// Returns an `Effect` with `timing == DelayEffect` and a `delay_trigger`
/// mapped from `compiled_trigger`. The `process` closure iterates the
/// `process_steps` via `run_step`.
///
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
) -> Effect {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        process_steps,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Effect {
    let delay_trigger = match trigger {
        CompiledTiming::EndOfYourTurn => DelayTrigger::EndOfThisTurn,
        _ => DelayTrigger::EndOfYourNextTurn,
    };
    let active_when = active_when.cloned().map(Arc::new);
    let process_arc: Arc<[CompiledStep]> = Arc::from(process_steps);
    let runtime = StepRuntime::new(raw);
    let mut builder = EffectBuilder::new(card, EffectTiming::DelayEffect)
        .delay(delay_trigger)
        .process(move |ctx| {
            if let Some(aw) = &active_when {
                let read = ctx.as_read();
                if !eval_predicate(aw, &read, PredicateSubject::None) {
                    return;
                }
            }
            let mut bindings = Bindings::new();
            run_steps_with_runtime(process_arc.as_ref(), ctx, &mut bindings, &runtime);
        });
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}
