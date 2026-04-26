//! Synchronous modifier-step lowering (Phase 2c).
//!
//! Verbs: AddDpModifier, AddModifier, GrantKeyword. All are binding-target
//! (filter-target form for AddModifier is Phase 2d). Unknown expiry strings,
//! unknown modifier names, or unknown keyword names cause the step to no-op —
//! same strictness convention as 2b (invalid references don't panic).
//!
//! Phase 2c handlers: AddDpModifier, AddModifier, GrantKeyword.

use digimon_dsl::compiled::{CompiledModifierTarget, CompiledModifierValue, CompiledStep};

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
            // Phase 2f2 Task 1: `value` is now an enum. Task 3 will replace this
            // guard with a real evaluator call for the `Formula` arm; for now
            // formula-valued modifiers silently no-op so engine compilation
            // stays green and the existing literal path is unchanged.
            let CompiledModifierValue::Literal(n) = value else { return true; };
            if let Some(ResolvedBinding::Permanent(h)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                ctx.add_dp_modifier(h, *n, expiry);
            }
            true
        }
        CompiledStep::AddModifier { target, modifier, value, expiry } => {
            let Some(expiry) = lookup_expiry(expiry) else { return true; };
            let Some(modifier_ty) = crate::dsl_cards::modifier_map::lookup_modifier_type(modifier) else {
                return true;
            };
            // Phase 2f2 Task 1: see AddDpModifier note above — Task 3 wires
            // formula evaluation; for now formula values are a silent no-op.
            let CompiledModifierValue::Literal(n) = value else { return true; };
            match target {
                CompiledModifierTarget::Binding(b) => {
                    if let Some(ResolvedBinding::Permanent(h)) =
                        resolve_binding_ref(b, ctx, bindings)
                    {
                        ctx.add_modifier(h, modifier_ty, *n, expiry);
                    }
                }
                CompiledModifierTarget::Filter(pred) => {
                    // Phase 2d Task 8: scan battle-area, apply modifier to every match.
                    let matches = crate::dsl_cards::step::permanent_scan::scan(ctx, pred);
                    for h in matches {
                        ctx.add_modifier(h, modifier_ty, *n, expiry);
                    }
                }
            }
            true
        }
        CompiledStep::GrantKeyword { target, keyword, expiry, value } => {
            let Some(expiry) = lookup_expiry(expiry) else { return true; };
            let Some(kw) = crate::dsl_cards::modifier_map::lookup_keyword(keyword, *value) else {
                return true;
            };
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
                ctx.grant_keyword(h, kw, expiry);
            }
            true
        }
        _ => false,
    }
}
