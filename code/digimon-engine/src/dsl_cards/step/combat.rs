use digimon_dsl::compiled::CompiledStep;

use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
    match step {
        CompiledStep::EndAttack { enabled } => {
            if *enabled {
                ctx.cancel_pending_attack();
            }
            true
        }
        _ => false,
    }
}
