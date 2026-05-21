//! Control-flow step lowering (Phase 2c: Optional + If; Phase 2d: returns
//! `Option<RunOutcome>` so a parked inner body suspends the outer slice).
//!
//! These live on the `run_steps` path (not `run_step`) because their inner
//! bodies may contain selection steps that need to park the continuation.
//! The dispatcher re-enters `run_steps` for each branch.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::selections::select_hand_candidate_count;
use crate::dsl_cards::step::{run_steps_with_runtime, RunOutcome, StepRuntime};
use crate::effect_context::EffectContext;

/// Returns `Some(outcome)` if `step` is a control-flow verb whose body
/// has been dispatched. The outer `run_steps` propagates a `Parked`
/// upward (Task 7 captures the outer tail).
///
/// Returns `None` for any non-control-flow step, letting the caller
/// fall through to the next dispatcher.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::Optional(body) => {
            // G-SELECT-EMPTY-OUTER-TAIL — when an `- optional: [...]` substep
            // leads with a `select_hand` that has zero matching candidates,
            // the raw `EffectContext::select_hand` would early-return without
            // installing a `PendingSelection`, and the dispatcher would then
            // run the rest of the optional body (e.g. a mandatory
            // `select_dna_pair`) even though the prerequisite pick is
            // impossible. Treat that as a declined optional substep: skip the
            // whole body. This keeps an empty-hand leading `select_hand` from
            // forcing subsequent mandatory steps.
            if let Some(CompiledStep::SelectHand { of, filter, .. }) = body.first() {
                if select_hand_candidate_count(ctx, *of, filter, bindings) == 0 {
                    return Some(RunOutcome::Synchronous);
                }
            }
            Some(run_steps_with_runtime(body, ctx, bindings, runtime))
        }
        CompiledStep::If {
            condition,
            then,
            else_branch,
        } => {
            let cond_holds = {
                let rctx = ctx.as_read();
                eval_predicate_with_bindings(
                    condition,
                    &rctx,
                    PredicateSubject::None,
                    Some(bindings),
                )
            };
            let body = if cond_holds { then } else { else_branch };
            Some(run_steps_with_runtime(body, ctx, bindings, runtime))
        }
        _ => None,
    }
}
