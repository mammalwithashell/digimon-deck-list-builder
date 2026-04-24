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
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
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
        _ => false,
    }
}
