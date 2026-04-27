//! Control-flow step lowering (Phase 2c: Optional + If; Phase 2d: returns
//! `Option<RunOutcome>` so a parked inner body suspends the outer slice).
//!
//! These live on the `run_steps` path (not `run_step`) because their inner
//! bodies may contain selection steps that need to park the continuation.
//! The dispatcher re-enters `run_steps` for each branch.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
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
            // Phase 2c: always run the body. Opt-out UX lands in 2e.
            Some(run_steps_with_runtime(body, ctx, bindings, runtime))
        }
        CompiledStep::If {
            condition,
            then,
            else_branch,
        } => {
            let cond_holds = {
                let rctx = ctx.as_read();
                eval_predicate(condition, &rctx, PredicateSubject::None)
            };
            let body = if cond_holds { then } else { else_branch };
            Some(run_steps_with_runtime(body, ctx, bindings, runtime))
        }
        _ => None,
    }
}
