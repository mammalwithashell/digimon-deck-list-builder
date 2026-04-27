//! Synchronous permanent-mutation step lowering (Phase 2c).
//!
//! Verbs: DeletePermanent, ReturnToHand, ReturnToDeck, Suspend, Unsuspend,
//! DeDigivolve. All are binding-consuming: the `target` field resolves to
//! `ResolvedBinding::Permanent`; any other variant is silently skipped (same
//! convention as the 2b zone-move handlers).

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` is a permanent-mutation family handled here.
/// Unknown steps fall through (the caller may try other families).
pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
        CompiledStep::DeletePermanent { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.delete_permanent(h);
            }
            true
        }
        CompiledStep::ReturnToHand { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let _ = ctx.return_to_hand(h);
            }
            true
        }
        CompiledStep::Suspend { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.suspend(h);
            }
            true
        }
        CompiledStep::Unsuspend { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.unsuspend(h);
            }
            true
        }
        CompiledStep::ReturnToDeck {
            target,
            position,
            include_sources: _,
        } => {
            // Phase 2c: `include_sources=true` is modelled in CompiledStep but the
            // engine currently trashes lower sources regardless — there is no
            // stack-return API yet. Phase 2d must add one (faithful full-stack
            // return) before this arm can honor `include_sources=true`.
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let _ = ctx.return_to_deck(h, super::map_stack_position(*position));
            }
            true
        }
        CompiledStep::DeDigivolve {
            target,
            amount,
            stop_at_level,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                // Engine signature is (target, stop_at_level, amount) — note stop_at_level
                // precedes amount here, opposite to CompiledStep field order.
                ctx.de_digivolve(h, *stop_at_level, *amount);
            }
            true
        }
        _ => false,
    }
}
