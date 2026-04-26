//! Lower `CompiledDeclarativeClause::Replacement` into a Would* engine
//! `Effect`.

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep};
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::run_steps;
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
///
/// Returns `None` for unknown trigger strings (caller silently skips).
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<&CompiledPredicate>,
    trigger: &str,
    process: &[CompiledStep],
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let label = format!("Replacement: {trigger}");
    let process: Arc<[CompiledStep]> = Arc::from(process);

    let mut builder = new_when_would_builder(card, timing)?;
    builder = builder.name(&label);

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    builder = builder.replacement_process(move |rctx| {
        let mut bindings = Bindings::new();
        rctx.effect.game.dsl_replacement_outcome = None;
        let _ = run_steps(&process, rctx.effect, &mut bindings);
        if let Some(outcome) = rctx.effect.game.dsl_replacement_outcome.take() {
            rctx.outcome = outcome;
        }
    });

    Some(builder.build())
}
