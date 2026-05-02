//! Lower `CompiledDeclarativeClause::Delay` into an engine `Effect` with
//! `timing == DelayEffect`.
//!
//! Phase 1 scope: map `CompiledTiming::EndOfYourTurn` →
//! `DelayTrigger::EndOfThisTurn`; `CompiledTiming::StartOfYourTurn` maps to
//! `DelayTrigger::StartOfYourNextTurn`; event timings map to
//! `DelayTrigger::OnEvent`. Body `active_when` predicates are evaluated when
//! the delayed effect fires.
//! Body steps run through `run_step` (Phase 2a dispatcher).

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
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
        CompiledTiming::StartOfYourTurn => DelayTrigger::StartOfYourNextTurn,
        CompiledTiming::EndOfYourNextTurn => DelayTrigger::EndOfYourNextTurn,
        CompiledTiming::OnSuspend | CompiledTiming::OnUnsuspend => {
            compiled_timing_to_engine(trigger)
                .map(DelayTrigger::OnEvent)
                .unwrap_or(DelayTrigger::EndOfYourNextTurn)
        }
        _ => DelayTrigger::EndOfYourNextTurn,
    };
    let process_arc: Arc<[CompiledStep]> = Arc::from(process_steps);
    let runtime = StepRuntime::new(raw);
    let active_when = active_when.cloned();
    let mut builder = EffectBuilder::new(card, EffectTiming::DelayEffect)
        .delay(delay_trigger)
        .process(move |ctx| {
            let mut bindings = Bindings::new();
            let _ = run_steps_with_runtime(&process_arc, ctx, &mut bindings, &runtime);
        });
    if let Some(predicate) = active_when {
        builder =
            builder.condition(move |ctx| eval_predicate(&predicate, ctx, PredicateSubject::None));
    }
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}
