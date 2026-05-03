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
        CompiledStep::DeleteBoundPermanents { binding } => {
            if let Some(mut handles) = bindings.get_permanent_list(binding) {
                handles.sort_by_key(|h| (h.player, h.index));
                handles.reverse();
                for handle in handles {
                    ctx.delete_permanent(handle);
                }
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
            include_sources,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let position = super::map_stack_position(*position);
                if *include_sources {
                    let _ = ctx.return_stack_to_deck(h, position);
                } else {
                    let _ = ctx.return_to_deck(h, position);
                }
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
        CompiledStep::TrashAllSources { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.trash_all_sources(h);
            }
            true
        }
        _ => false,
    }
}
