use digimon_dsl::compiled::{CompiledAttackTargetSpec, CompiledBindingRef, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::{AttackTargetRestriction, EffectContext};
use crate::permanent::PermanentHandle;

fn lower_attack_target_restriction(targets: CompiledAttackTargetSpec) -> AttackTargetRestriction {
    match targets {
        CompiledAttackTargetSpec::Any => AttackTargetRestriction::Any,
        CompiledAttackTargetSpec::Player => AttackTargetRestriction::PlayerOnly,
        CompiledAttackTargetSpec::Digimon => AttackTargetRestriction::DigimonOnly,
    }
}

fn resolve_permanent_ref(
    target: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<PermanentHandle> {
    if matches!(target, CompiledBindingRef::SelfRef) {
        return ctx.source_permanent;
    }

    match resolve_binding_ref(target, ctx, bindings) {
        Some(ResolvedBinding::Permanent(handle)) => Some(handle),
        _ => None,
    }
}

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::Battle { attacker, defender } => {
            let Some(attacker) = resolve_permanent_ref(attacker, ctx, bindings) else {
                return true;
            };
            let Some(defender) = resolve_permanent_ref(defender, ctx, bindings) else {
                return true;
            };
            ctx.battle_digimon(attacker, defender);
            true
        }
        CompiledStep::MayAttackNow {
            attacker,
            targets,
            without_suspending,
            optional,
            prompt,
        } => {
            let Some(attacker) = resolve_permanent_ref(attacker, ctx, bindings) else {
                return true;
            };
            let prompt = prompt.as_deref().unwrap_or("Attack with this Digimon?");
            let _ = ctx.may_attack_now_optional(
                attacker,
                lower_attack_target_restriction(*targets),
                *without_suspending,
                *optional,
                prompt,
            );
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
