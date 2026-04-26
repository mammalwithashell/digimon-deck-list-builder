//! Phase 2f3 — `AsSelectingPlayer` step lowering.
//!
//! Sets `ctx.override_selecting_player = Some(resolve_player(of))` for the
//! duration of `body`, runs `body` via `run_steps`, and restores the
//! previous override on synchronous return. On park, the override persists
//! into the captured callback's reconstructed `EffectContext` via Task 1's
//! `new_with_override` mechanism — so do NOT restore on park.
//!
//! Outer-tail steps (post-`AsSelectingPlayer` siblings parked by
//! `dsl_outer_tail`) must NOT see the override. The selection callbacks
//! reconstruct `cb_ctx` with `override_pin` (= the override at install
//! time) preserved; this would leak into outer-tail steps drained from
//! that same `cb_ctx`. Clearing the override happens in
//! `drain_dsl_outer_tail` (see `step::mod`) so any AsSelectingPlayer's
//! body-parked select cannot smuggle the override past the body boundary.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::{resolve_player, run_steps, RunOutcome};
use crate::effect_context::EffectContext;

/// Returns `Some(outcome)` if `step` is `AsSelectingPlayer`. Returns
/// `None` otherwise so the dispatcher falls through to the next handler.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::AsSelectingPlayer { of, body } => {
            let p = resolve_player(ctx, *of);
            let prev = ctx.override_selecting_player();
            ctx.set_override_selecting_player(Some(p));
            let outcome = run_steps(body, ctx, bindings);
            if matches!(outcome, RunOutcome::Synchronous) {
                ctx.set_override_selecting_player(prev);
            }
            // On Parked: do NOT restore. Task 1's `new_with_override`
            // captured the override into the parked callback's
            // reconstructed `EffectContext`. Restoration of the outer
            // (controller-only) view happens in `drain_dsl_outer_tail`,
            // which clears the override before draining sibling steps.
            Some(outcome)
        }
        _ => None,
    }
}
