//! Lower `CompiledDeclarativeClause::Replacement` into a Would* engine
//! `Effect` with an empty `replacement_process` closure.
//!
//! Phase 1 scope: the process body is a no-op. Full replacement process
//! bodies (cancel / redirect / substitute helpers wired into
//! `ReplacementContext`) are a separate plan.

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::trigger_map::lookup_replacement_trigger;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;

/// Build the base `EffectBuilder` for a "Would*" replacement timing.
/// Returns `None` only if `timing` is not one of the nine `WhenWould*`
/// variants (guard against future callers passing the wrong timing).
fn new_when_would_builder(card: CardHandle, timing: EffectTiming) -> Option<EffectBuilder> {
    match timing {
        EffectTiming::WhenWouldBeDeleted
        | EffectTiming::WhenWouldLeaveBattleArea
        | EffectTiming::WhenWouldBeReturnedToHand
        | EffectTiming::WhenWouldBeReturnedToDeck
        | EffectTiming::WhenWouldBeTrashed
        | EffectTiming::WhenWouldBeDeDigivolved
        | EffectTiming::WhenWouldLoseSecurity
        | EffectTiming::WhenWouldDraw
        | EffectTiming::WhenWouldPlaceInSecurity => Some(EffectBuilder::new(card, timing)),
        _ => None,
    }
}

/// Lower a `Replacement` declarative clause.
///
/// `_active_when` is accepted but ignored — Phase 2 gating.
/// `_process` is accepted but ignored — full process bodies are a
/// separate plan; Phase 1 installs an empty no-op closure.
///
/// Returns `None` for unknown trigger strings (caller silently skips).
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<&CompiledPredicate>,
    trigger: &str,
    _process: &[CompiledStep],
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let label = format!("Replacement: {trigger}");

    let mut builder = new_when_would_builder(card, timing)?;
    builder = builder.name(&label);

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    builder = builder.replacement_process(|_rctx| {
        // Phase 1: no-op body.
        // Full process lowering (cancel / redirect / substitute) is a
        // separate plan once run_step is lifted into ReplacementContext.
    });

    Some(builder.build())
}
