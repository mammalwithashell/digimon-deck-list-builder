//! Lower `CompiledDeclarativeClause::Replacement` into a Would* engine
//! `Effect`.

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep};
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::trigger_map::lookup_replacement_trigger;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;
use crate::replacement::ReplacementSubject;

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
    let process: Arc<[CompiledStep]> = Arc::from(process);
    let runtime = StepRuntime::new(raw);

    let mut builder = new_when_would_builder(card, timing)?;
    builder = builder.name(&label);

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    if let Some(active_when) = active_when {
        let active_when = active_when.clone();
        builder = builder.replacement_condition(move |ctx, subject| {
            let subject = match *subject {
                ReplacementSubject::Permanent(handle) => PredicateSubject::Permanent(handle),
                ReplacementSubject::Card(handle, _) => PredicateSubject::Card(handle),
                ReplacementSubject::Player(_) => PredicateSubject::None,
            };
            eval_predicate(&active_when, ctx, subject)
        });
    }

    builder = builder.replacement_process(move |rctx| {
        // DSL replacements scope to the carrier permanent by default:
        // only fire when the SUBJECT is the same permanent that carries
        // this effect. This mirrors the "When THIS Digimon would leave"
        // semantics on all printed self-replacement clauses. Inherited
        // clauses (scope == Inherited) can propagate from any card in the
        // digivolution stack; for now they still bind to the top-card perm.
        let subject_matches = match rctx.subject {
            ReplacementSubject::Permanent(subject_h) => {
                rctx.effect.source_permanent == Some(subject_h)
            }
            _ => false,
        };
        if !subject_matches {
            return;
        }

        let mut bindings = Bindings::new();
        rctx.effect.game.dsl_replacement_outcome = None;
        let _ = run_steps_with_runtime(&process, rctx.effect, &mut bindings, &runtime);
        if let Some(outcome) = rctx.effect.game.dsl_replacement_outcome.take() {
            rctx.outcome = outcome;
        }
    });

    Some(builder.build())
}
