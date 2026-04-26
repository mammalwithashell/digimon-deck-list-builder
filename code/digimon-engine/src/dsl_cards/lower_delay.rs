//! Lower `CompiledDeclarativeClause::Delay` into an engine `Effect` with
//! `timing == DelayEffect`.
//!
//! Phase 1 scope: map `CompiledTiming::EndOfYourTurn` →
//! `DelayTrigger::EndOfThisTurn`; all other timings default to
//! `DelayTrigger::EndOfYourNextTurn`. `active_when` gating is deferred.
//! Body steps run through `run_step` (Phase 2a dispatcher).

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::run_steps;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{DelayTrigger, EffectTiming};

/// Lower a `Delay` declarative clause.
///
/// Returns an `Effect` with `timing == DelayEffect` and a `delay_trigger`
/// mapped from `compiled_trigger`. The `process` closure iterates the
/// `process_steps` via `run_step`.
///
/// `_active_when` gating is deferred to a future phase.
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
) -> Effect {
    let delay_trigger = match trigger {
        CompiledTiming::EndOfYourTurn => DelayTrigger::EndOfThisTurn,
        _ => DelayTrigger::EndOfYourNextTurn,
    };
    let process_arc: Arc<[CompiledStep]> = Arc::from(process_steps);
    let mut builder = EffectBuilder::new(card, EffectTiming::DelayEffect)
        .delay(delay_trigger)
        .process(move |ctx| {
            let mut bindings = Bindings::new();
            let _ = run_steps(&process_arc, ctx, &mut bindings);
        });
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}
