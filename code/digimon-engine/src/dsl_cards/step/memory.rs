//! Memory-mutation step lowering.

use digimon_dsl::compiled::CompiledStep;

use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
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
        _ => false,
    }
}
