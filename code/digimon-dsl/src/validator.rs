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
            ClauseSpec::Triggered(t) => {
                validate_triggered(t, &prefix, &spec.card, ctx, &mut errors)
            }
            ClauseSpec::Declarative(d) => {
                if let Some(active_when) = &d.active_when {
                    validate_predicate(
                        active_when,
                        &format!("{prefix}.active_when"),
                        &spec.card,
                        ctx,
                        &mut errors,
                    );
                }
                match d.typed_body() {
                    Err(e) => {
                        errors.push(ValidationError {
                            card_id: spec.card.clone(),
                            path: prefix.clone(),
                            message: format!("declarative body schema: {e}"),
                        });
                        continue;
                    }
                    Ok(body) => match d.kind {
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
                        DeclarativeKind::CostReduction => {
                            if let crate::clause::TypedDeclarativeBody::CostReduction(b) = body {
                                if let Some(pred) = &b.when_any_ally_played {
                                    validate_predicate(
                                        pred,
                                        &format!("{prefix}.when_any_ally_played"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                                if let Some(pred) = &b.condition {
                                    validate_predicate(
                                        pred,
                                        &format!("{prefix}.condition"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                                if let Some(formula) = &b.amount_fn {
                                    validate_formula(
                                        formula,
                                        &format!("{prefix}.amount_fn"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                                for (k, step) in b.pay_cost.iter().flatten().enumerate() {
                                    validate_step(
                                        step,
                                        &format!("{prefix}.pay_cost[{k}]"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                            }
                        }
                        DeclarativeKind::Replacement => {
                            if let crate::clause::TypedDeclarativeBody::Replacement(b) = body {
                                for (k, step) in b.process.iter().enumerate() {
                                    validate_step(
                                        step,
                                        &format!("{prefix}.process[{k}]"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                            }
                        }
                        DeclarativeKind::Delay => {
                            if let crate::clause::TypedDeclarativeBody::Delay(b) = body {
                                for (k, step) in b.process.iter().enumerate() {
                                    validate_step(
                                        step,
                                        &format!("{prefix}.process[{k}]"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
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
                                validate_predicate(
                                    &b.target,
                                    &format!("{prefix}.target"),
                                    &spec.card,
                                    ctx,
                                    &mut errors,
                                );
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
                                validate_predicate(
                                    &b.target,
                                    &format!("{prefix}.target"),
                                    &spec.card,
                                    ctx,
                                    &mut errors,
                                );
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
                    },
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_triggered(
    t: &TriggeredClause,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(cond) = &t.condition {
        validate_predicate(cond, &format!("{prefix}.condition"), card_id, ctx, errors);
    }
    if let Some(aw) = &t.active_when {
        validate_predicate(aw, &format!("{prefix}.active_when"), card_id, ctx, errors);
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
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    for (field, dp) in [
        ("dp_eq", &pred.dp_eq),
        ("dp_lte", &pred.dp_lte),
        ("dp_gte", &pred.dp_gte),
    ] {
        if let Some(crate::predicate::DpConstraint::Formula(formula)) = dp {
            validate_formula(formula, &format!("{prefix}.{field}"), card_id, ctx, errors);
        }
    }
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
        validate_predicate(sub, &format!("{prefix}.all_of[{i}]"), card_id, ctx, errors);
    }
    for (i, sub) in pred.any_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.any_of[{i}]"), card_id, ctx, errors);
    }
    for (i, sub) in pred.none_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.none_of[{i}]"), card_id, ctx, errors);
    }
    if let Some(sub) = &pred.not {
        validate_predicate(sub, &format!("{prefix}.not"), card_id, ctx, errors);
    }
    if let Some(ex) = &pred.any_permanent {
        validate_predicate(
            &ex.predicate,
            &format!("{prefix}.any_permanent"),
            card_id,
            ctx,
            errors,
        );
    }
    if let Some(ex) = &pred.no_permanent {
        validate_predicate(
            &ex.predicate,
            &format!("{prefix}.no_permanent"),
            card_id,
            ctx,
            errors,
        );
    }
    if let Some(ex) = &pred.all_permanents {
        validate_predicate(
            &ex.predicate,
            &format!("{prefix}.all_permanents"),
            card_id,
            ctx,
            errors,
        );
    }
    if let Some(agg) = &pred.count_lte {
        validate_predicate(
            &agg.filter,
            &format!("{prefix}.count_lte.filter"),
            card_id,
            ctx,
            errors,
        );
    }
    if let Some(agg) = &pred.count_gte {
        validate_predicate(
            &agg.filter,
            &format!("{prefix}.count_gte.filter"),
            card_id,
            ctx,
            errors,
        );
    }
    if let Some(inh) = &pred.has_inherited {
        validate_predicate(
            inh,
            &format!("{prefix}.has_inherited"),
            card_id,
            ctx,
            errors,
        );
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
            validate_modifier_value(
                &args.value,
                &format!("{prefix}.value"),
                card_id,
                ctx,
                errors,
            );
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::AddModifier(args) => {
            if let crate::step::ModifierTarget::Filter(filter) = &args.target {
                validate_predicate(filter, &format!("{prefix}.target"), card_id, ctx, errors);
            }
            validate_modifier_value(
                &args.value,
                &format!("{prefix}.value"),
                card_id,
                ctx,
                errors,
            );
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
        StepSpec::SelectOwnPermanent(args) | StepSpec::SelectOpponentPermanent(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::SelectHand(args)
        | StepSpec::SelectTrash(args)
        | StepSpec::SelectReveal(args)
        | StepSpec::SelectSecurity(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::SelectMaterial(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::SelectUnionZone(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::SelectCountCappedMulti(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::If(i) => {
            if let Ok(condition) =
                serde_yml::from_value::<crate::predicate::PredicateSpec>(i.condition.clone())
            {
                validate_predicate(
                    &condition,
                    &format!("{prefix}.condition"),
                    card_id,
                    ctx,
                    errors,
                );
            }
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
            validate_predicate(&f.over, &format!("{prefix}.over"), card_id, ctx, errors);
            for (k, s) in f.body.iter().enumerate() {
                validate_step(s, &format!("{prefix}.body[{k}]"), card_id, ctx, errors);
            }
        }
        StepSpec::PerSelected(ps) => {
            for (k, s) in ps.body.iter().enumerate() {
                validate_step(s, &format!("{prefix}.body[{k}]"), card_id, ctx, errors);
            }
        }
        StepSpec::ScheduleDelayed(sd) => {
            for (k, s) in sd.body.iter().enumerate() {
                validate_step(s, &format!("{prefix}.body[{k}]"), card_id, ctx, errors);
            }
        }
        StepSpec::AsSelectingPlayer(args) => {
            for (k, s) in args.body.iter().enumerate() {
                validate_step(s, &format!("{prefix}.body[{k}]"), card_id, ctx, errors);
            }
        }
        StepSpec::Optional(optional) => {
            for (k, s) in optional.0.iter().enumerate() {
                validate_step(s, &format!("{prefix}.optional[{k}]"), card_id, ctx, errors);
            }
        }
        _ => {}
    }
}

fn validate_modifier_value(
    value: &crate::step::ModifierValueSpec,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    if let crate::step::ModifierValueSpec::Formula(formula) = value {
        validate_formula(&formula.formula, prefix, card_id, ctx, errors);
    }
}

fn validate_formula(
    formula: &crate::formula::FormulaSpec,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    use crate::formula::{CompoundFormula, FormulaSpec};

    match formula {
        FormulaSpec::Compound(CompoundFormula::RawRust(name)) => {
            if !ctx.raw_rust.contains_fn(name) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: prefix.into(),
                    message: format!("unknown raw_rust fn: {name}"),
                });
            }
        }
        FormulaSpec::Compound(CompoundFormula::FloorDiv(args))
        | FormulaSpec::Compound(CompoundFormula::Max(args))
        | FormulaSpec::Compound(CompoundFormula::Min(args)) => {
            for (i, arg) in args.iter().enumerate() {
                validate_formula(arg, &format!("{prefix}[{i}]"), card_id, ctx, errors);
            }
        }
        FormulaSpec::Literal(_)
        | FormulaSpec::BasePerDelta { .. }
        | FormulaSpec::Compound(CompoundFormula::Aggregate(_))
        | FormulaSpec::Compound(CompoundFormula::AggregateScoped(_)) => {}
    }
}

fn is_known_modifier(name: &str) -> bool {
    matches!(
        name,
        "ChangeDp"
            | "ChangeBaseDp"
            | "DpFloor"
            | "DontHaveDp"
            | "ChangePlayCost"
            | "ChangeDigivolveCost"
            | "CannotReduceCost"
            | "CannotBeDestroyed"
            | "CannotBeDestroyedByBattle"
            | "CannotBeDestroyedByEffect"
            | "CannotBeRemoved"
            | "CannotAttack"
            | "CannotAttackPlayer"
            | "CanAttackUnsuspended"
            | "CanAttackActivePlayer"
            | "CannotAttackTarget"
            | "CannotSuspend"
            | "CannotUnsuspend"
            | "CannotBeSelectedByEffect"
            | "CannotBeAffected"
            | "GrantBlocker"
            | "GrantRush"
            | "GrantJamming"
            | "GrantPiercing"
            | "GrantReboot"
            | "GrantBlitz"
            | "GrantAlliance"
            | "GrantRaid"
            | "GrantDecoy"
            | "GrantVortex"
            | "GrantOverclock"
            | "MayAttack"
            | "ForceAttack"
            | "SecurityAttackChange"
            | "CannotDigivolve"
            | "ChangeColor"
            | "AddColor"
            | "ChangeLevel"
            | "CannotReturnToHand"
            | "CannotTrash"
            | "CannotBlock"
            | "CannotCounter"
            | "DrawBlock"
            | "MemoryBlock"
            | "CannotPlayFromHand"
            | "CannotPlayDigimonByEffect"
            | "CannotGainMemoryByEffect"
            | "CannotGainMemoryExceptFromTamers"
            | "CannotReducePlayCost"
            | "CannotActivateMainEffects"
            | "CannotActivateWhenDigivolvingEffects"
            | "CannotActivateSecurityEffects"
            | "CannotDigivolveDigimonByEffect"
            | "CannotDrawByEffect"
            | "CannotAddSecurityByEffect"
            | "CannotTrashOpponentSecurity"
            | "CannotReduceOpponentSecurity"
            | "IgnoreColorRequirement"
    )
}

fn is_known_keyword(name: &str) -> bool {
    matches!(
        name,
        "Blocker"
            | "SecurityAttackPlus"
            | "SecurityAttackMinus"
            | "Rush"
            | "Jamming"
            | "Piercing"
            | "Reboot"
            | "DeDigivolve"
            | "DrawX"
            | "Blitz"
            | "Raid"
            | "Alliance"
            | "BlastDigivolve"
            | "Save"
            | "Fortitude"
            | "Overclock"
            | "Barrier"
            | "Decoy"
            | "Partition"
            | "Vortex"
            | "Collision"
            | "Progress"
            | "Evade"
            | "MaterialSave"
            | "Delay"
    )
}

/// Every snake_case expiry key the validator accepts. Must stay in lockstep
/// with `digimon_engine::dsl_cards::expiry_map::all_engine_expiry_keys()`;
/// the parity test in `code/digimon-engine/tests/dsl/expiry_parity.rs`
/// fails the build if the two lists diverge.
pub const KNOWN_EXPIRY_KEYS: &[&str] = &[
    "permanent",
    "end_of_turn",
    "end_of_opponents_turn",
    "end_of_attack",
    "end_of_battle",
    "until_leave_field",
];

fn is_known_expiry(name: &str) -> bool {
    KNOWN_EXPIRY_KEYS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw_rust_registry::StubRegistry;
    use crate::spec::CardSpec;

    #[test]
    fn formula_level_raw_rust_is_checked_against_registry() {
        let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: cost_reduction
    when_playing_this: true
    amount_fn:
      raw_rust: unregistered_formula_fn
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message.contains("unregistered_formula_fn")));
    }

    #[test]
    fn formula_level_raw_rust_passes_when_registered() {
        let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: cost_reduction
    when_playing_this: true
    amount_fn:
      raw_rust: registered_formula_fn
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::with(["registered_formula_fn"]);
        assert!(validate(&spec, &ValidationContext { raw_rust: &reg }).is_ok());
    }
}
