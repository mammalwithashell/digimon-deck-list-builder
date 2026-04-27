//! Lower `CompiledDeclarativeClause::Replacement` into a Would* engine
//! `Effect` whose replacement process runs the compiled DSL step body.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::trigger_map::lookup_replacement_trigger;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;
use crate::replacement::{ParkedReplacement, ReplacementOutcome};

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

/// Lower a `Replacement` declarative clause with a default empty raw-rust
/// registry.
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    process: &[CompiledStep],
) -> Option<Effect> {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        process,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    process: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let label = format!("Replacement: {trigger}");
    let active_when = active_when.cloned().map(Arc::new);
    let process = Arc::new(process.to_vec());
    let runtime = StepRuntime::new(raw);

    let mut builder = new_when_would_builder(card, timing)?.name(&label);
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    if let Some(predicate) = active_when.clone() {
        builder = builder.replacement_condition(move |ctx, _cause| {
            eval_predicate(&predicate, ctx, PredicateSubject::None)
        });
    }

    builder = builder.replacement_process(move |rctx| {
        let previous = rctx
            .effect
            .game
            .parked_replacement
            .replace(ParkedReplacement {
                subject: rctx.subject,
                cause: rctx.cause,
                original_destination: rctx.original_destination,
                source_card: rctx.effect.source_card,
                source_permanent: rctx.effect.source_permanent,
                controller: rctx.effect.player,
                outcome: ReplacementOutcome::None,
            });
        debug_assert!(
            previous.is_none(),
            "DSL replacement process started while another replacement was parked"
        );

        let mut bindings = crate::dsl_cards::bindings::Bindings::new();
        let _ = run_steps_with_runtime(&process, rctx.effect, &mut bindings, &runtime);

        if let Some(parked) = rctx.effect.game.parked_replacement.take() {
            rctx.outcome = parked.outcome;
        }

        if let Some(previous) = previous {
            rctx.effect.game.parked_replacement = Some(previous);
        }
    });

    Some(builder.build())
}
