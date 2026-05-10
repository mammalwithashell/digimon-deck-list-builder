use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect::TimingFilter;
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

fn resolve_permanent_ref(
    target: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<PermanentHandle> {
    if matches!(
        target,
        CompiledBindingRef::SelfRef | CompiledBindingRef::Source
    ) {
        return ctx.source_permanent;
    }

    match resolve_binding_ref(target, ctx, bindings) {
        Some(ResolvedBinding::Permanent(handle)) => Some(handle),
        _ => None,
    }
}

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    let CompiledStep::RefireEffect {
        source,
        timing,
        optional,
    } = step
    else {
        return false;
    };

    if let Some(source) = resolve_permanent_ref(source, ctx, bindings) {
        if timing == "on_play_or_when_digivolving" {
            let _ = ctx.refire_target_effect(source, TimingFilter::Either, ctx.player, false);
        } else {
            let _ = ctx.refire_effect_from_permanent(source, timing, *optional);
        }
    }
    true
}
