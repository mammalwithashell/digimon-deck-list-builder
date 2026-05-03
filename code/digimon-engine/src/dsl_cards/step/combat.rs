use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::Battle { attacker, defender } => {
            let Some(ResolvedBinding::Permanent(attacker)) =
                resolve_binding_ref(attacker, ctx, bindings)
            else {
                return true;
            };
            let Some(ResolvedBinding::Permanent(defender)) =
                resolve_binding_ref(defender, ctx, bindings)
            else {
                return true;
            };
            ctx.battle_digimon(attacker, defender);
            true
        }
        CompiledStep::EndAttack { enabled } => {
            if *enabled {
                ctx.cancel_pending_attack();
            }
            true
        }
        _ => false,
    }
}
