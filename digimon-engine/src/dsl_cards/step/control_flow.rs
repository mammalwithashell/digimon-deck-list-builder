//! Control-flow step lowering (Phase 2c: Optional + If).
//!
//! These live on the `run_steps` path (not `run_step`) because their inner
//! bodies may contain selection steps that need to park the continuation.
//! The dispatcher re-enters `run_steps` for each branch.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::run_steps;
use crate::effect_context::EffectContext;

/// Returns `true` if the step is a control-flow verb whose body has been
/// dispatched. The caller (`run_steps`) should continue with the next step
/// at the outer level after this returns.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::Optional(body) => {
            // Phase 2c: always run the body. Opt-out UX lands in 2d.
            run_steps(body, ctx, bindings);
            true
        }
        _ => false,
    }
}
