//! Iteration steps (Phase 2d): ForEach + PerSelected.
//!
//! These live on the `run_steps` path (not `run_step`) because their
//! per-iteration bodies may park selections. The dispatcher re-enters
//! `run_steps` once per iteration and propagates a `RunOutcome::Parked`
//! upward via Task 7's continuation plumbing.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::permanent_scan::scan;
use crate::dsl_cards::step::{run_steps_with_runtime, RunOutcome, StepRuntime};
use crate::effect_context::EffectContext;

/// Returns `Some(RunOutcome)` if `step` is an iteration verb. Returns
/// `None` if `step` is not an iteration verb (the caller continues
/// dispatching).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::ForEach {
            over,
            bind_as,
            body,
        } => {
            let matches = scan(ctx, over);
            for handle in matches {
                let mut iter_bindings = bindings.clone();
                iter_bindings.insert_permanent(bind_as, handle);
                let outcome = run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                if matches!(outcome, RunOutcome::Parked) {
                    // v1 semantics: a parked iteration aborts remaining
                    // iterations. Faithful per-iteration resumption is
                    // a future-phase enhancement.
                    return Some(RunOutcome::Parked);
                }
            }
            Some(RunOutcome::Synchronous)
        }
        CompiledStep::PerSelected {
            selection,
            bind_as,
            body,
        } => {
            let bref = CompiledBindingRef::Named(selection.clone());
            match resolve_binding_ref(&bref, ctx, bindings) {
                Some(ResolvedBinding::PermanentList(v)) => {
                    for h in v {
                        let mut iter_bindings = bindings.clone();
                        iter_bindings.insert_permanent(bind_as, h);
                        if matches!(
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime),
                            RunOutcome::Parked
                        ) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                Some(ResolvedBinding::CardList(v)) => {
                    for c in v {
                        let mut iter_bindings = bindings.clone();
                        iter_bindings.insert_card(bind_as, c);
                        if matches!(
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime),
                            RunOutcome::Parked
                        ) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                _ => {} // Missing or wrong-typed binding → silent no-op (2b/2c convention).
            }
            Some(RunOutcome::Synchronous)
        }
        _ => None,
    }
}
