use digimon_dsl::compiled::{
    CompiledAttackCostUpgrade, CompiledAttackTargetSpec, CompiledBindingRef, CompiledStep,
};

use crate::combat::AttackCostUpgrade;
use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::{AttackTargetRestriction, EffectContext};
use crate::permanent::PermanentHandle;
use crate::selection::AttackTarget;

fn lower_attack_target_restriction(targets: CompiledAttackTargetSpec) -> AttackTargetRestriction {
    match targets {
        CompiledAttackTargetSpec::Any => AttackTargetRestriction::Any,
        CompiledAttackTargetSpec::Player => AttackTargetRestriction::PlayerOnly,
        CompiledAttackTargetSpec::Digimon => AttackTargetRestriction::DigimonOnly,
    }
}

fn lower_attack_cost_upgrade(
    upgrade: Option<CompiledAttackCostUpgrade>,
) -> Option<AttackCostUpgrade> {
    upgrade.map(|u| AttackCostUpgrade {
        dp: u.dp,
        security_attack: u.security_attack,
    })
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
            cost_upgrade,
        } => {
            let Some(attacker) = resolve_permanent_ref(attacker, ctx, bindings) else {
                return true;
            };
            let prompt = prompt.as_deref().unwrap_or("Attack with this Digimon?");
            let _ = ctx.may_attack_now_optional_with_upgrade(
                attacker,
                lower_attack_target_restriction(*targets),
                *without_suspending,
                *optional,
                prompt,
                lower_attack_cost_upgrade(*cost_upgrade),
            );
            true
        }
        CompiledStep::ForceAttack {
            attacker,
            targets,
            without_suspending,
            prompt,
            cost_upgrade,
        } => {
            let Some(attacker) = resolve_permanent_ref(attacker, ctx, bindings) else {
                return true;
            };
            let prompt = prompt.as_deref().unwrap_or("Attack with this Digimon");
            let _ = ctx.force_opponent_attack_with_upgrade(
                attacker,
                lower_attack_target_restriction(*targets),
                *without_suspending,
                prompt,
                lower_attack_cost_upgrade(*cost_upgrade),
            );
            true
        }
        CompiledStep::RedirectAttackTarget {
            new_target,
            player,
            targets,
            optional,
            prompt,
        } => {
            if let Some(player) = player {
                let _ = ctx.redirect_attack(AttackTarget::Player(resolve_player(ctx, *player)));
                return true;
            }
            if let Some(new_target) = new_target {
                let Some(handle) = resolve_permanent_ref(new_target, ctx, bindings) else {
                    return true;
                };
                let _ = ctx.redirect_attack(AttackTarget::Digimon(handle));
                return true;
            }
            let prompt = prompt.as_deref().unwrap_or("Change the attack target?");
            let _ = ctx.select_redirect_attack_target(
                lower_attack_target_restriction(*targets),
                *optional,
                prompt,
            );
            true
        }
        CompiledStep::CancelAttack => {
            ctx.cancel_pending_attack();
            true
        }
        CompiledStep::OpenCounterWindow => {
            let _ = ctx.open_counter_window();
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
