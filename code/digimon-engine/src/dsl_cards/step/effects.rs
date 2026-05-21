use digimon_dsl::compiled::{CompiledBindingRef, CompiledColor, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect::TimingFilter;
use crate::effect_context::EffectContext;
use crate::enums::CardColor;
use crate::permanent::PermanentHandle;

/// Map a DSL `CompiledColor` to the engine's `CardColor`.
fn compiled_color_to_card_color(c: CompiledColor) -> CardColor {
    match c {
        CompiledColor::Red => CardColor::Red,
        CompiledColor::Blue => CardColor::Blue,
        CompiledColor::Yellow => CardColor::Yellow,
        CompiledColor::Green => CardColor::Green,
        CompiledColor::Black => CardColor::Black,
        CompiledColor::Purple => CardColor::Purple,
        CompiledColor::White => CardColor::White,
    }
}

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
    match step {
        CompiledStep::RefireEffect {
            source,
            timing,
            optional,
        } => {
            if let Some(source) = resolve_permanent_ref(source, ctx, bindings) {
                if timing == "on_play_or_when_digivolving" {
                    let _ =
                        ctx.refire_target_effect(source, TimingFilter::Either, ctx.player, false);
                } else {
                    let _ = ctx.refire_effect_from_permanent(source, timing, *optional);
                }
            }
            true
        }
        // G-COST-REDUCE-ALLY-DIGIVOLVE — install a player-scoped one-shot
        // future-digivolve cost reducer (BT3-103 Hidden Potential Discovered!).
        CompiledStep::ArmDigivolveCostReducer {
            amount,
            single_fire,
            target_color,
            suspend_cost,
        } => {
            ctx.arm_player_digivolve_cost_reducer(
                *amount,
                *single_fire,
                target_color.map(compiled_color_to_card_color),
                *suspend_cost,
            );
            true
        }
        _ => false,
    }
}
