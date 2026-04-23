//! Semantic validator — runs after YAML parse, before lowering (Phase 2+).
//!
//! Checks:
//! - modifier / keyword / expiry strings resolve to engine enums
//! - declarative-clause body schemas deserialize per-kind via `typed_body`
//! - `raw_rust:` references resolve in the registry

use crate::clause::{ClauseSpec, DeclarativeKind, TriggeredClause};
use crate::errors::ValidationError;
use crate::raw_rust_registry::RawRustRegistry;
use crate::spec::CardSpec;
use crate::step::StepSpec;

pub struct ValidationContext<'a> {
    pub raw_rust: &'a dyn RawRustRegistry,
}

pub fn validate(spec: &CardSpec, ctx: &ValidationContext<'_>) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    for (i, clause) in spec.effects.iter().enumerate() {
        let prefix = format!("effects[{i}]");
        match clause {
            ClauseSpec::Triggered(t) => validate_triggered(t, &prefix, &spec.card, ctx, &mut errors),
            ClauseSpec::Declarative(d) => {
                match d.typed_body() {
                    Err(e) => {
                        errors.push(ValidationError {
                            card_id: spec.card.clone(),
                            path: prefix.clone(),
                            message: format!("declarative body schema: {e}"),
                        });
                        continue;
                    }
                    Ok(body) => {
                        match d.kind {
                            DeclarativeKind::RawRust => {
                                if let crate::clause::TypedDeclarativeBody::RawRust(b) = body {
                                    if !ctx.raw_rust.contains_fn(&b.fn_name) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.fn"),
                                            message: format!("unknown raw_rust fn: {}", b.fn_name),
                                        });
                                    }
                                }
                            }
                            DeclarativeKind::FloodGate => {
                                if let crate::clause::TypedDeclarativeBody::FloodGate(b) = body {
                                    if !is_known_modifier(&b.modifier) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.modifier"),
                                            message: format!("unknown modifier: {}", b.modifier),
                                        });
                                    }
                                    validate_predicate(&b.target, &format!("{prefix}.target"), &spec.card, &mut errors);
                                }
                            }
                            DeclarativeKind::Aura => {
                                if let crate::clause::TypedDeclarativeBody::Aura(b) = &body {
                                    if let Some(gk) = &b.grant_keyword {
                                        if !is_known_keyword(&gk.keyword) {
                                            errors.push(ValidationError {
                                                card_id: spec.card.clone(),
                                                path: format!("{prefix}.grant_keyword.keyword"),
                                                message: format!("unknown keyword: {}", gk.keyword),
                                            });
                                        }
                                    }
                                    validate_predicate(&b.target, &format!("{prefix}.target"), &spec.card, &mut errors);
                                }
                            }
                            DeclarativeKind::GrantKeyword => {
                                if let crate::clause::TypedDeclarativeBody::GrantKeyword(b) = body {
                                    if !is_known_keyword(&b.keyword) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.keyword"),
                                            message: format!("unknown keyword: {}", b.keyword),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn validate_triggered(
    t: &TriggeredClause,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(cond) = &t.condition {
        validate_predicate(cond, &format!("{prefix}.condition"), card_id, errors);
    }
    if let Some(aw) = &t.active_when {
        validate_predicate(aw, &format!("{prefix}.active_when"), card_id, errors);
    }
    for (i, step) in t.process.iter().enumerate() {
        let sp = format!("{prefix}.process[{i}]");
        validate_step(step, &sp, card_id, ctx, errors);
    }
}

fn validate_predicate(
    pred: &crate::predicate::PredicateSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(kw) = &pred.has_keyword {
        if !is_known_keyword(kw) {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.has_keyword"),
                message: format!("unknown keyword: {kw}"),
            });
        }
    }
    for (i, sub) in pred.all_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.all_of[{i}]"), card_id, errors);
    }
    for (i, sub) in pred.any_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.any_of[{i}]"), card_id, errors);
    }
    for (i, sub) in pred.none_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.none_of[{i}]"), card_id, errors);
    }
    if let Some(sub) = &pred.not {
        validate_predicate(sub, &format!("{prefix}.not"), card_id, errors);
    }
    if let Some(ex) = &pred.any_permanent {
        validate_predicate(&ex.predicate, &format!("{prefix}.any_permanent"), card_id, errors);
    }
    if let Some(ex) = &pred.no_permanent {
        validate_predicate(&ex.predicate, &format!("{prefix}.no_permanent"), card_id, errors);
    }
    if let Some(ex) = &pred.all_permanents {
        validate_predicate(&ex.predicate, &format!("{prefix}.all_permanents"), card_id, errors);
    }
    if let Some(agg) = &pred.count_lte {
        validate_predicate(&agg.filter, &format!("{prefix}.count_lte.filter"), card_id, errors);
    }
    if let Some(agg) = &pred.count_gte {
        validate_predicate(&agg.filter, &format!("{prefix}.count_gte.filter"), card_id, errors);
    }
    if let Some(inh) = &pred.has_inherited {
        validate_predicate(inh, &format!("{prefix}.has_inherited"), card_id, errors);
    }
}

fn validate_step(
    step: &StepSpec,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    match step {
        StepSpec::AddDpModifier(args) => {
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::AddModifier(args) => {
            if !is_known_modifier(&args.modifier) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.modifier"),
                    message: format!("unknown modifier: {}", args.modifier),
                });
            }
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::GrantKeyword(args) => {
            if !is_known_keyword(&args.keyword) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.keyword"),
                    message: format!("unknown keyword: {}", args.keyword),
                });
            }
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::RawRust(raw) => {
            if !ctx.raw_rust.contains_fn(&raw.fn_name) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.fn"),
                    message: format!("unknown raw_rust fn: {}", raw.fn_name),
                });
            }
        }
        StepSpec::If(i) => {
            for (k, s) in i.then.iter().enumerate() {
                validate_step(s, &format!("{prefix}.then[{k}]"), card_id, ctx, errors);
            }
            if let Some(else_) = &i.else_ {
                for (k, s) in else_.iter().enumerate() {
                    validate_step(s, &format!("{prefix}.else[{k}]"), card_id, ctx, errors);
                }
            }
        }
        StepSpec::ForEach(f) => {
            for (k, s) in f.body.iter().enumerate() {
                validate_step(s, &format!("{prefix}.body[{k}]"), card_id, ctx, errors);
            }
        }
        _ => {}
    }
}

fn is_known_modifier(name: &str) -> bool {
    matches!(
        name,
        "ChangeDp" | "ChangeBaseDp" | "DpFloor" | "DontHaveDp"
        | "ChangePlayCost" | "ChangeDigivolveCost" | "CannotReduceCost"
        | "CannotBeDestroyed" | "CannotBeDestroyedByBattle" | "CannotBeDestroyedByEffect" | "CannotBeRemoved"
        | "CannotAttack" | "CannotAttackPlayer" | "CanAttackUnsuspended" | "CanAttackActivePlayer" | "CannotAttackTarget"
        | "CannotSuspend" | "CannotUnsuspend"
        | "CannotBeSelectedByEffect" | "CannotBeAffected"
        | "GrantBlocker" | "GrantRush" | "GrantJamming" | "GrantPiercing" | "GrantReboot"
        | "GrantBlitz" | "GrantAlliance" | "GrantRaid" | "GrantBarrier" | "GrantArmor"
        | "GrantDecoy" | "GrantVortex" | "GrantOverclock"
        | "MayAttack" | "ForceAttack"
        | "SecurityAttackChange"
        | "CannotDigivolve" | "ChangeColor" | "AddColor" | "ChangeLevel"
        | "CannotReturnToHand" | "CannotTrash" | "CannotBlock" | "CannotCounter"
        | "DrawBlock" | "MemoryBlock" | "CannotPlayFromHand"
        | "CannotPlayDigimonByEffect" | "CannotGainMemoryByEffect" | "CannotGainMemoryExceptFromTamers"
        | "CannotReducePlayCost" | "CannotActivateMainEffects" | "CannotActivateWhenDigivolvingEffects"
        | "CannotActivateSecurityEffects" | "CannotDigivolveDigimonByEffect" | "CannotDrawByEffect"
        | "CannotAddSecurityByEffect" | "CannotTrashOpponentSecurity" | "CannotReduceOpponentSecurity"
        | "IgnoreColorRequirement"
    )
}

fn is_known_keyword(name: &str) -> bool {
    matches!(
        name,
        "Blocker" | "SecurityAttackPlus" | "SecurityAttackMinus" | "Rush" | "Jamming"
        | "Piercing" | "Reboot" | "DeDigivolve" | "DrawX" | "Blitz" | "Armor"
        | "Raid" | "Alliance" | "Blast" | "Save" | "Fortitude" | "Overclock"
        | "Barrier" | "Decoy" | "Material" | "Partition" | "Vortex" | "Collision"
        | "Progress" | "Evade" | "MaterialSave" | "Delay"
    )
}

fn is_known_expiry(name: &str) -> bool {
    matches!(
        name,
        "end_of_your_turn" | "end_of_opponents_turn" | "end_of_your_next_turn"
        | "end_of_opponents_next_turn" | "end_of_turn" | "end_of_battle"
        | "end_of_attack" | "permanent" | "until_next_unsuspend" | "while_source_exists"
    )
}
