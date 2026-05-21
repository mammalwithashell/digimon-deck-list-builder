//! Memory-mutation step lowering.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::formula_eval;
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> bool {
    match step {
        CompiledStep::GainMemory(n) => {
            ctx.gain_memory(*n as i16);
            true
        }
        CompiledStep::LoseMemory(n) => {
            ctx.lose_memory(*n as i16);
            true
        }
        CompiledStep::SetMemory(n) => {
            ctx.set_memory(*n as i16);
            true
        }
        CompiledStep::GainMemoryFn { formula } => {
            let target = formula_target(ctx);
            let n = formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings));
            if n != 0 {
                ctx.gain_memory(n.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
            }
            true
        }
        CompiledStep::LoseMemoryFn { formula } => {
            let target = formula_target(ctx);
            let n = formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings));
            if n != 0 {
                ctx.lose_memory(n.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
            }
            true
        }
        _ => false,
    }
}

/// Default-target permanent passed to `formula_eval` for memory-fn steps.
/// Most formula variants (e.g. `card_count_in_zone`, `distinct_colors_count`)
/// ignore the target; the source-permanent-dependent ones (e.g.
/// `source_stack_dp_sum`, `binding_dp`) read it through the same threaded
/// bindings. Source permanent if known; else fall through to a placeholder
/// that the formula evaluator tolerates.
fn formula_target(ctx: &EffectContext<'_>) -> PermanentHandle {
    if let Some(h) = ctx.source_permanent {
        return h;
    }
    PermanentHandle {
        player: ctx.player,
        index: 0,
    }
}
