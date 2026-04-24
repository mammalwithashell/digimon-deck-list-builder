//! Synchronous modifier-step lowering (Phase 2c).
//!
//! Verbs: AddDpModifier, AddModifier, GrantKeyword. All are binding-target
//! (filter-target form for AddModifier is Phase 2d). Unknown expiry strings,
//! unknown modifier names, or unknown keyword names cause the step to no-op —
//! same strictness convention as 2b (invalid references don't panic).

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::expiry_map::lookup_expiry;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` is a modifier family handled here.
/// Unknown steps fall through (the caller may try other families).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::AddDpModifier { target, value, expiry } => {
            let Some(expiry) = lookup_expiry(expiry) else { return true; };
            if let Some(ResolvedBinding::Permanent(h)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                ctx.add_dp_modifier(h, *value, expiry);
            }
            true
        }
        _ => false,
    }
}
