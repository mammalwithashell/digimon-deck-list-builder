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

/// Park a `BeginAttack` resume frame alongside the closure selection installed
/// by `may_attack_now_*` / `force_opponent_attack` (make-engine-cloneable
/// coexistence). `inner_tail` is empty (a single-step effect); the clause's
/// following steps ride `outer_conts`. The optional "may attack" decline is a
/// NO-OP that CONTINUES the clause, so `ResumeDecline::RunTail{empty,false}`
/// (runs outer_conts); a forced attack (`optional: false`) is mandatory →
/// `ResumeDecline::None`. No-op when the primitive installed no selection
/// (no legal target).
fn park_begin_attack_resume_frame(
    ctx: &mut EffectContext<'_>,
    attacker: PermanentHandle,
    without_suspending: bool,
    ignore_summoning_sickness: bool,
    optional: bool,
    cost_upgrade: Option<AttackCostUpgrade>,
) {
    if ctx.game.pending_selection.is_none() {
        return;
    }
    let decline = if optional {
        crate::resume::ResumeDecline::RunTail {
            tail: std::sync::Arc::new(Vec::new()),
            aborts_clause: false,
        }
    } else {
        crate::resume::ResumeDecline::None
    };
    let prov = crate::resume::ResumeProvenance {
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        controller: ctx.player,
        override_pin: ctx.override_selecting_player(),
    };
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::RunTail {
            prov,
            select_kind: crate::resume::ResumeSelectKind::BeginAttack {
                attacker,
                without_suspending,
                ignore_summoning_sickness,
                optional,
                cost_upgrade,
            },
            bind_as: None,
            inner_tail: std::sync::Arc::new(Vec::new()),
            outer_conts: Vec::new(),
            bindings: Bindings::new(),
            runtime: crate::dsl_cards::step::StepRuntime::default(),
            trigger_context,
            decline,
        }],
    });
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
            ignore_summoning_sickness,
            optional,
            windowed,
            prompt,
            cost_upgrade,
        } => {
            let Some(attacker) = resolve_permanent_ref(attacker, ctx, bindings) else {
                return true;
            };
            if *windowed {
                use crate::enums::{Expiry, ModifierType};
                use crate::modifiers::ModifierEntry;
                let owner = attacker.player;
                ctx.game.modifiers.add(
                    attacker,
                    ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, owner),
                );
                if *without_suspending {
                    ctx.game.modifiers.add(
                        attacker,
                        ModifierEntry::simple(
                            ModifierType::CanAttackUnsuspended,
                            1,
                            Expiry::EndOfTurn,
                            owner,
                        ),
                    );
                }
                return true;
            }
            let prompt = prompt.as_deref().unwrap_or("Attack with this Digimon?");
            let lowered_upgrade = lower_attack_cost_upgrade(*cost_upgrade);
            let _ = if *ignore_summoning_sickness {
                ctx.may_attack_now_ignoring_summoning_sickness_with_upgrade(
                    attacker,
                    lower_attack_target_restriction(*targets),
                    *without_suspending,
                    *optional,
                    prompt,
                    lowered_upgrade,
                )
            } else {
                ctx.may_attack_now_optional_with_upgrade(
                    attacker,
                    lower_attack_target_restriction(*targets),
                    *without_suspending,
                    *optional,
                    prompt,
                    lowered_upgrade,
                )
            };
            // Park the data frame alongside the closure (coexistence): driven by
            // run_resume's BeginAttack arm (decode → begin_attack_open).
            park_begin_attack_resume_frame(
                ctx,
                attacker,
                *without_suspending,
                *ignore_summoning_sickness,
                *optional,
                lowered_upgrade,
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
            let lowered_upgrade = lower_attack_cost_upgrade(*cost_upgrade);
            let _ = ctx.force_opponent_attack_with_upgrade(
                attacker,
                lower_attack_target_restriction(*targets),
                *without_suspending,
                prompt,
                lowered_upgrade,
            );
            // force_opponent_attack ⇒ mandatory (optional: false), summoning
            // sickness NOT ignored. Park the data frame (coexistence).
            park_begin_attack_resume_frame(
                ctx,
                attacker,
                *without_suspending,
                false,
                false,
                lowered_upgrade,
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
            // Resumable-VM (make-engine-cloneable): capture the attacker before
            // the primitive installs the closure selection (the redirect does not
            // consume `pending_attack`), so the parked data frame can decode +
            // substitute without it.
            let attacker = ctx.game.pending_attack.as_ref().map(|pa| pa.attacker);
            let trigger_context = ctx.game.current_trigger_context.clone();
            let source_card = ctx.source_card;
            let source_permanent = ctx.source_permanent;
            let source_kind = ctx.source_kind;
            let controller = ctx.player;
            let override_pin = ctx.override_selecting_player();
            let _ = ctx.select_redirect_attack_target(
                lower_attack_target_restriction(*targets),
                *optional,
                prompt,
            );
            // Park the data frame alongside the closure (coexistence): driven by
            // run_resume's AttackTarget arm (decode → validate → substitute).
            // `inner_tail` is empty (a single-step effect — the redirect IS the
            // effect); the clause's following steps ride `outer_conts`. The
            // optional "may" decline is a NO-OP that CONTINUES the clause, so
            // ResumeDecline::RunTail{ empty, false } (runs outer_conts), NOT
            // ResumeDecline::None (drops them).
            if let Some(attacker) = attacker {
                if ctx.game.pending_selection.is_some() {
                    let decline = if *optional {
                        crate::resume::ResumeDecline::RunTail {
                            tail: std::sync::Arc::new(Vec::new()),
                            aborts_clause: false,
                        }
                    } else {
                        crate::resume::ResumeDecline::None
                    };
                    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                        frames: vec![crate::resume::ResumeFrame::RunTail {
                            prov: crate::resume::ResumeProvenance {
                                source_card,
                                source_permanent,
                                source_kind,
                                controller,
                                override_pin,
                            },
                            select_kind: crate::resume::ResumeSelectKind::AttackTarget { attacker },
                            bind_as: None,
                            inner_tail: std::sync::Arc::new(Vec::new()),
                            outer_conts: Vec::new(),
                            bindings: bindings.clone(),
                            runtime: crate::dsl_cards::step::StepRuntime::default(),
                            trigger_context,
                            decline,
                        }],
                    });
                }
            }
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
