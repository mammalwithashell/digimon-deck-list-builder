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
use crate::step::{BindingRef, StepSpec, StructuredBindingRef};
use std::collections::BTreeSet;

pub struct ValidationContext<'a> {
    pub raw_rust: &'a dyn RawRustRegistry,
}

pub fn validate(spec: &CardSpec, ctx: &ValidationContext<'_>) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Validate the card's top-level clauses plus the per-face clauses of a
    // DUAL card (`dual.digimon.effects` / `dual.option.effects`,
    // `G-DSL-DUAL-PER-FACE-EFFECTS`). All three lists share the exact same
    // per-clause semantic checks; only the error-path prefix differs.
    let mut all_clauses: Vec<(String, &ClauseSpec)> = spec
        .effects
        .iter()
        .enumerate()
        .map(|(i, c)| (format!("effects[{i}]"), c))
        .collect();
    if let Some(dual) = &spec.dual {
        all_clauses.extend(
            dual.digimon
                .effects
                .iter()
                .enumerate()
                .map(|(i, c)| (format!("dual.digimon.effects[{i}]"), c)),
        );
        all_clauses.extend(
            dual.option
                .effects
                .iter()
                .enumerate()
                .map(|(i, c)| (format!("dual.option.effects[{i}]"), c)),
        );
    }

    for (prefix, clause) in &all_clauses {
        let prefix = prefix.clone();
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
                                if let Some(formula) = &b.amount {
                                    validate_formula(
                                        formula,
                                        &format!("{prefix}.amount"),
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
                                if let Some(pay_cost) = &b.pay_cost {
                                    let mut scope = BTreeSet::new();
                                    validate_steps_binding_scope(
                                        pay_cost,
                                        &format!("{prefix}.pay_cost"),
                                        &spec.card,
                                        &mut scope,
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
                                let mut scope = BTreeSet::new();
                                validate_steps_binding_scope(
                                    &b.process,
                                    &format!("{prefix}.process"),
                                    &spec.card,
                                    &mut scope,
                                    &mut errors,
                                );
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
                                let mut scope = BTreeSet::new();
                                validate_steps_binding_scope(
                                    &b.process,
                                    &format!("{prefix}.process"),
                                    &spec.card,
                                    &mut scope,
                                    &mut errors,
                                );
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
                                if b.target.is_none() && b.target_player.is_none() {
                                    errors.push(ValidationError {
                                        card_id: spec.card.clone(),
                                        path: prefix.clone(),
                                        message: "flood_gate requires target or target_player"
                                            .into(),
                                    });
                                }
                                if b.target_player.is_some()
                                    && is_permanent_activation_modifier(&b.modifier)
                                {
                                    errors.push(ValidationError {
                                        card_id: spec.card.clone(),
                                        path: format!("{prefix}.target_player"),
                                        message: format!(
                                            "{} requires a permanent target, not target_player",
                                            b.modifier
                                        ),
                                    });
                                }
                                if let Some(target) = &b.target {
                                    validate_predicate(
                                        target,
                                        &format!("{prefix}.target"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                                if let Some(expiry) = &b.expiry {
                                    if !is_known_expiry(expiry) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.expiry"),
                                            message: format!("unknown expiry: {expiry}"),
                                        });
                                    }
                                }
                            }
                        }
                        DeclarativeKind::Aura => {
                            if let crate::clause::TypedDeclarativeBody::Aura(b) = &body {
                                if b.target.is_none() && b.target_player.is_none() {
                                    errors.push(ValidationError {
                                        card_id: spec.card.clone(),
                                        path: format!("{prefix}.target"),
                                        message: "aura requires target or target_player"
                                            .to_string(),
                                    });
                                }
                                if b.dp_modifier.is_none()
                                    && b.security_attack.is_none()
                                    && b.grant_keyword.is_none()
                                    && b.modifier.is_none()
                                    && b.effect_immunity.is_none()
                                {
                                    errors.push(ValidationError {
                                        card_id: spec.card.clone(),
                                        path: prefix.clone(),
                                        message: "aura requires a payload: dp_modifier, security_attack, grant_keyword, modifier, or effect_immunity"
                                            .to_string(),
                                    });
                                }
                                // `effect_immunity` only lowers on the
                                // self-aura declarative-tick path
                                // (G-DSL-AURA-EFFECT-IMMUNITY).
                                if b.effect_immunity.is_some() {
                                    let is_self_target = b
                                        .target
                                        .as_ref()
                                        .map(|t| t.is_empty())
                                        .unwrap_or(true);
                                    if !is_self_target || b.target_player.is_some() {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.effect_immunity"),
                                            message:
                                                "effect_immunity is a self-aura payload: use target: {} and no target_player"
                                                    .to_string(),
                                        });
                                    }
                                    if b.while_condition.is_some() {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.effect_immunity"),
                                            message:
                                                "effect_immunity does not support while_condition; gate it with active_when (re-evaluated each tick)"
                                                    .to_string(),
                                        });
                                    }
                                }
                                // unify-dsl-scalar-and-comparators: `dp_modifier`
                                // is now a `FormulaSpec`. A bare `Literal` is a
                                // static grant and needs no validation; the
                                // dynamic-DP-aura restrictions apply only when
                                // the value is a real (non-literal) formula.
                                if let Some(formula) = dynamic_aura_formula(&b.dp_modifier) {
                                    validate_formula(
                                        formula,
                                        &format!("{prefix}.dp_modifier"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                    if let Some(target) = &b.target {
                                        if predicate_depends_on_dp(target) {
                                            errors.push(ValidationError {
                                                card_id: spec.card.clone(),
                                                path: format!("{prefix}.dp_modifier"),
                                                message: "dynamic DP aura cannot use a DP-dependent target predicate"
                                                    .to_string(),
                                            });
                                        }
                                    }
                                    if let Some(active_when) = &d.active_when {
                                        if predicate_depends_on_dp(active_when) {
                                            errors.push(ValidationError {
                                                card_id: spec.card.clone(),
                                                path: format!("{prefix}.dp_modifier"),
                                                message: "dynamic DP aura cannot use a DP-dependent active_when predicate"
                                                    .to_string(),
                                            });
                                        }
                                    }
                                    if formula_uses_dp_aggregate(formula) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.dp_modifier"),
                                            message:
                                                "dynamic DP aura cannot use a DP aggregate formula"
                                                    .to_string(),
                                        });
                                    }
                                }
                                if let Some(formula) = dynamic_aura_formula(&b.security_attack) {
                                    validate_formula(
                                        formula,
                                        &format!("{prefix}.security_attack"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
                                if let Some(modifier) = &b.modifier {
                                    if !is_known_modifier(modifier) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.modifier"),
                                            message: format!("unknown modifier: {modifier}"),
                                        });
                                    }
                                }
                                if let Some(gk) = &b.grant_keyword {
                                    if !is_known_keyword(&gk.keyword) {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.grant_keyword.keyword"),
                                            message: format!("unknown keyword: {}", gk.keyword),
                                        });
                                    }
                                }
                                if let Some(target) = &b.target {
                                    validate_predicate(
                                        target,
                                        &format!("{prefix}.target"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
                                }
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
                                if let Some(filter) = &b.overclock_cost_filter {
                                    if !b.keyword.eq_ignore_ascii_case("Overclock") {
                                        errors.push(ValidationError {
                                            card_id: spec.card.clone(),
                                            path: format!("{prefix}.overclock_cost_filter"),
                                            message: "overclock_cost_filter is only valid for keyword: Overclock"
                                                .to_string(),
                                        });
                                    }
                                    validate_predicate(
                                        filter,
                                        &format!("{prefix}.overclock_cost_filter"),
                                        &spec.card,
                                        ctx,
                                        &mut errors,
                                    );
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

/// Extract the dynamic (non-literal) formula from a unified magnitude field
/// (unify-dsl-scalar-and-comparators). Returns `None` for an absent value or a
/// bare `Literal` (a static grant), so the dynamic-aura validation rules fire
/// only for true runtime formulas.
fn dynamic_aura_formula(
    f: &Option<crate::formula::FormulaSpec>,
) -> Option<&crate::formula::FormulaSpec> {
    match f {
        Some(spec) if !matches!(spec, crate::formula::FormulaSpec::Literal(_)) => Some(spec),
        _ => None,
    }
}

fn predicate_depends_on_dp(pred: &crate::predicate::PredicateSpec) -> bool {
    pred.dp_eq.is_some()
        || pred.dp_lte.is_some()
        || pred.dp_gte.is_some()
        || pred.all_of.iter().any(predicate_depends_on_dp)
        || pred.any_of.iter().any(predicate_depends_on_dp)
        || pred.none_of.iter().any(predicate_depends_on_dp)
        || pred.not.as_deref().is_some_and(predicate_depends_on_dp)
        || pred
            .any_permanent
            .as_deref()
            .is_some_and(|ex| predicate_depends_on_dp(&ex.predicate))
        || pred
            .any_field_permanent
            .as_deref()
            .is_some_and(|ex| predicate_depends_on_dp(&ex.predicate))
        || pred
            .no_permanent
            .as_deref()
            .is_some_and(|ex| predicate_depends_on_dp(&ex.predicate))
        || pred
            .all_permanents
            .as_deref()
            .is_some_and(|ex| predicate_depends_on_dp(&ex.predicate))
        || pred
            .count_lte
            .as_ref()
            .is_some_and(|agg| predicate_depends_on_dp(&agg.filter))
        || pred
            .count_gte
            .as_ref()
            .is_some_and(|agg| predicate_depends_on_dp(&agg.filter))
        || pred
            .has_inherited
            .as_deref()
            .is_some_and(predicate_depends_on_dp)
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
    let mut scope = BTreeSet::new();
    validate_steps_binding_scope(
        &t.process,
        &format!("{prefix}.process"),
        card_id,
        &mut scope,
        errors,
    );
}

fn validate_predicate(
    pred: &crate::predicate::PredicateSpec,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    for (field, dp) in [
        ("level_lte", &pred.level_lte),
        ("level_gte", &pred.level_gte),
        ("play_cost_lte", &pred.play_cost_lte),
        ("play_or_use_cost_lte", &pred.play_or_use_cost_lte),
        ("dp_eq", &pred.dp_eq),
        ("dp_lte", &pred.dp_lte),
        ("dp_gte", &pred.dp_gte),
        ("stack_size_lte", &pred.stack_size_lte),
        ("stack_size_gte", &pred.stack_size_gte),
        ("materials_count_lte", &pred.materials_count_lte),
        ("materials_count_gte", &pred.materials_count_gte),
        ("memory_lte", &pred.memory_lte),
        ("memory_gte", &pred.memory_gte),
        ("own_memory_lte", &pred.own_memory_lte),
        ("own_memory_gte", &pred.own_memory_gte),
        ("security_count_lte", &pred.security_count_lte),
        ("security_count_gte", &pred.security_count_gte),
        ("face_up_security_count_lte", &pred.face_up_security_count_lte),
        ("face_up_security_count_gte", &pred.face_up_security_count_gte),
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
    if let Some(level_aggregate) = pred.level_matches_aggregate {
        if !matches!(
            level_aggregate.selector,
            crate::formula::AggregateSelector::LowestLevel
                | crate::formula::AggregateSelector::HighestLevel
        ) {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.level_matches_aggregate.selector"),
                message: "level_matches_aggregate selector must be lowest_level or highest_level"
                    .into(),
            });
        }
    }
    if let Some(materials_aggregate) = pred.materials_count_matches_aggregate {
        if !matches!(
            materials_aggregate.selector,
            crate::formula::AggregateSelector::FewestMaterials
        ) {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.materials_count_matches_aggregate.selector"),
                message: "materials_count_matches_aggregate selector must be fewest_materials"
                    .into(),
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
    if let Some(sub) = &pred.returned_card_matching {
        validate_predicate(
            sub,
            &format!("{prefix}.returned_card_matching"),
            card_id,
            ctx,
            errors,
        );
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
    if let Some(ex) = &pred.any_field_permanent {
        validate_predicate(
            &ex.predicate,
            &format!("{prefix}.any_field_permanent"),
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
        if let crate::predicate::DpConstraint::Formula(formula) = &agg.n {
            validate_formula(
                formula,
                &format!("{prefix}.count_lte.n"),
                card_id,
                ctx,
                errors,
            );
        }
        validate_predicate(
            &agg.filter,
            &format!("{prefix}.count_lte.filter"),
            card_id,
            ctx,
            errors,
        );
    }
    if let Some(agg) = &pred.count_gte {
        if let crate::predicate::DpConstraint::Formula(formula) = &agg.n {
            validate_formula(
                formula,
                &format!("{prefix}.count_gte.n"),
                card_id,
                ctx,
                errors,
            );
        }
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
    if let Some(sc) = &pred.source_count {
        validate_predicate(
            &sc.filter,
            &format!("{prefix}.source_count.filter"),
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
        StepSpec::Battle(args) => {
            validate_binding_ref(
                &args.attacker,
                &format!("{prefix}.attacker"),
                card_id,
                errors,
            );
            validate_binding_ref(
                &args.defender,
                &format!("{prefix}.defender"),
                card_id,
                errors,
            );
        }
        StepSpec::RefireEffect(args) => {
            validate_binding_ref(&args.source, &format!("{prefix}.source"), card_id, errors);
            if !matches!(
                args.timing.as_str(),
                "on_play" | "when_digivolving" | "on_play_or_when_digivolving"
            ) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.timing"),
                    message: format!(
                        "refire_effect only supports timing: on_play, when_digivolving, or on_play_or_when_digivolving, got {}",
                        args.timing
                    ),
                });
            }
            if args.timing == "on_play_or_when_digivolving" && args.optional {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.optional"),
                    message: "refire_effect optional: true is not supported with timing: on_play_or_when_digivolving; put optionality on the target selection or containing clause".to_string(),
                });
            }
        }
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
            // `synth_identity` is the structured payload for `TreatAsDigimon`
            // and is meaningless on any other modifier. Require it for
            // TreatAsDigimon (without it the modifier would install with an
            // empty payload and silently no-op) and forbid it elsewhere.
            match (args.modifier.as_str(), args.synth_identity.is_some()) {
                ("TreatAsDigimon", false) => errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.synth_identity"),
                    message: "TreatAsDigimon requires a synth_identity payload \
                              (e.g. `synth_identity: { dp: 3000 }`)"
                        .into(),
                }),
                (m, true) if m != "TreatAsDigimon" => errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.synth_identity"),
                    message: format!(
                        "synth_identity is only valid for modifier TreatAsDigimon, not {m}"
                    ),
                }),
                _ => {}
            }
        }
        StepSpec::AddPlayerModifier(args) => {
            if !is_known_modifier(&args.modifier) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.modifier"),
                    message: format!("unknown modifier: {}", args.modifier),
                });
            }
            if is_permanent_activation_modifier(&args.modifier) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.modifier"),
                    message: format!(
                        "{} requires a permanent target, not add_player_modifier",
                        args.modifier
                    ),
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
        StepSpec::PlaceOnSecurity(args) => {
            // collapse §3.3 — validate the source binding (card / permanent).
            // `self`/`self_option` markers carry no binding to check.
            match &args.source {
                crate::step::SecuritySource::Card { card } => {
                    validate_binding_ref(card, &format!("{prefix}.source.card"), card_id, errors);
                }
                crate::step::SecuritySource::Permanent { permanent } => {
                    validate_binding_ref(
                        permanent,
                        &format!("{prefix}.source.permanent"),
                        card_id,
                        errors,
                    );
                }
                crate::step::SecuritySource::Marker(_) => {}
            }
        }
        StepSpec::SecurityPlaceStackedCard(args) => {
            validate_binding_ref(&args.carrier, &format!("{prefix}.carrier"), card_id, errors);
            if args.source.is_none() && args.source_index_from_top.is_none() {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: prefix.into(),
                    message: "security_place_stacked_card requires source or source_index_from_top"
                        .into(),
                });
            }
            if let Some(source) = &args.source {
                validate_binding_ref(source, &format!("{prefix}.source"), card_id, errors);
            }
        }
        StepSpec::SecurityPlaceTopStackedCard(args) => {
            validate_binding_ref(&args.carrier, &format!("{prefix}.carrier"), card_id, errors);
        }
        StepSpec::TrashTopNDigivolutionCardsOfEach(args) => {
            validate_formula(&args.n, &format!("{prefix}.n"), card_id, ctx, errors);
        }
        StepSpec::TrashOpponentHandToCount(args) => {
            validate_formula(
                &args.target_count,
                &format!("{prefix}.target_count"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::DeDigivolve(args) => {
            validate_binding_ref(&args.target, &format!("{prefix}.target"), card_id, errors);
            // unify-dsl-scalar-and-comparators: `amount` is one `FormulaSpec`
            // field (a bare int or a formula); the old "amount vs amount_fn,
            // not both" mutual-exclusion check no longer applies.
            if let Some(formula) = &args.amount {
                validate_formula(formula, &format!("{prefix}.amount"), card_id, ctx, errors);
            }
        }
        StepSpec::SearchOwnSecurityStack(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
            if args.on_select.is_empty() {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.on_select"),
                    message: "search_own_security_stack requires a non-empty on_select body".into(),
                });
            }
            for (k, s) in args.on_select.iter().enumerate() {
                validate_step(s, &format!("{prefix}.on_select[{k}]"), card_id, ctx, errors);
            }
            if let Some(no_match) = &args.on_no_match {
                for (k, s) in no_match.iter().enumerate() {
                    validate_step(
                        s,
                        &format!("{prefix}.on_no_match[{k}]"),
                        card_id,
                        ctx,
                        errors,
                    );
                }
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
        StepSpec::UseOptionFromHand(args) => {
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::SelectRevealBuckets(args) => {
            if args.buckets.is_empty() {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.buckets"),
                    message: "select_reveal_buckets requires at least one bucket".into(),
                });
            }
            let mut seen = std::collections::BTreeSet::new();
            for (i, bucket) in args.buckets.iter().enumerate() {
                if !seen.insert(bucket.bind_as.clone()) {
                    errors.push(ValidationError {
                        card_id: card_id.into(),
                        path: format!("{prefix}.buckets[{i}].bind_as"),
                        message: format!(
                            "select_reveal_buckets duplicate bucket bind_as: {}",
                            bucket.bind_as
                        ),
                    });
                }
                let min = bucket.min.unwrap_or(0);
                let max = bucket.max.unwrap_or(1);
                if min > max {
                    errors.push(ValidationError {
                        card_id: card_id.into(),
                        path: format!("{prefix}.buckets[{i}]"),
                        message: format!(
                            "select_reveal_buckets bucket {} has min greater than max",
                            bucket.bind_as
                        ),
                    });
                }
                if let Some(filter) = &bucket.filter {
                    validate_predicate(
                        filter,
                        &format!("{prefix}.buckets[{i}].filter"),
                        card_id,
                        ctx,
                        errors,
                    );
                }
            }
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
        StepSpec::SelectMaterials(args) => {
            if let crate::step::CountBound::Formula { formula } = &args.max {
                validate_formula(
                    formula,
                    &format!("{prefix}.max.formula"),
                    card_id,
                    ctx,
                    errors,
                );
            }
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
            if let Some(zf) = &args.zone_filters {
                for (label, pred) in [
                    ("hand", &zf.hand),
                    ("trash", &zf.trash),
                    ("material", &zf.material),
                ] {
                    if let Some(pred) = pred {
                        validate_predicate(
                            pred,
                            &format!("{prefix}.zone_filters.{label}"),
                            card_id,
                            ctx,
                            errors,
                        );
                    }
                }
            }
            if let Some(pred) = &args.material_carrier_filter {
                validate_predicate(
                    pred,
                    &format!("{prefix}.material_carrier_filter"),
                    card_id,
                    ctx,
                    errors,
                );
            }
        }
        StepSpec::SelectCountCappedMulti(args) => {
            if let crate::step::CountBound::Formula { formula } = &args.max {
                validate_formula(
                    formula,
                    &format!("{prefix}.max.formula"),
                    card_id,
                    ctx,
                    errors,
                );
            }
            validate_predicate(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                ctx,
                errors,
            );
        }
        StepSpec::SelectOpponentSources(args) => {
            // G-DSL-SELECT-SOURCES-FORMULA-COUNT: formula-valued counts
            // validate like `select_count_capped_multi`'s `max`.
            if let crate::step::CountBound::Formula { formula } = &args.min {
                validate_formula(
                    formula,
                    &format!("{prefix}.min.formula"),
                    card_id,
                    ctx,
                    errors,
                );
            }
            if let crate::step::CountBound::Formula { formula } = &args.max {
                validate_formula(
                    formula,
                    &format!("{prefix}.max.formula"),
                    card_id,
                    ctx,
                    errors,
                );
            }
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

fn validate_steps_binding_scope(
    steps: &[StepSpec],
    prefix: &str,
    card_id: &str,
    scope: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    for (i, step) in steps.iter().enumerate() {
        validate_step_binding_scope(step, &format!("{prefix}[{i}]"), card_id, scope, errors);
    }
}

fn validate_step_binding_scope(
    step: &StepSpec,
    prefix: &str,
    card_id: &str,
    scope: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    match step {
        StepSpec::AddDpModifier(args) => {
            validate_modifier_value_binding_scope(
                &args.value,
                &format!("{prefix}.value"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::AddModifier(args) => {
            if let crate::step::ModifierTarget::Filter(filter) = &args.target {
                validate_predicate_binding_scope(
                    filter,
                    &format!("{prefix}.target"),
                    card_id,
                    scope,
                    errors,
                );
            }
            validate_modifier_value_binding_scope(
                &args.value,
                &format!("{prefix}.value"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::TrashTopNDigivolutionCardsOfEach(args) => {
            validate_formula_binding_scope(&args.n, &format!("{prefix}.n"), card_id, scope, errors);
        }
        StepSpec::TrashOpponentHandToCount(args) => {
            validate_formula_binding_scope(
                &args.target_count,
                &format!("{prefix}.target_count"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::DeDigivolve(args) => {
            validate_binding_ref(&args.target, &format!("{prefix}.target"), card_id, errors);
            if let Some(formula) = &args.amount {
                validate_formula_binding_scope(
                    formula,
                    &format!("{prefix}.amount"),
                    card_id,
                    scope,
                    errors,
                );
            }
        }
        StepSpec::SelectOwnPermanent(args)
        | StepSpec::SelectOpponentPermanent(args)
        | StepSpec::SelectAnyPermanent(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectHand(args)
        | StepSpec::SelectTrash(args)
        | StepSpec::SelectReveal(args)
        | StepSpec::SelectSecurity(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::UseOptionFromHand(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::SelectDnaPair(args) => {
            validate_predicate_binding_scope(
                &args.left_filter,
                &format!("{prefix}.left_filter"),
                card_id,
                scope,
                errors,
            );
            validate_predicate_binding_scope(
                &args.right_filter,
                &format!("{prefix}.right_filter"),
                card_id,
                scope,
                errors,
            );
            scope.insert(args.bind_left_as.clone());
            scope.insert(args.bind_right_as.clone());
        }
        StepSpec::SelectRevealBuckets(args) => {
            for (i, bucket) in args.buckets.iter().enumerate() {
                if let Some(filter) = &bucket.filter {
                    validate_predicate_binding_scope(
                        filter,
                        &format!("{prefix}.buckets[{i}].filter"),
                        card_id,
                        scope,
                        errors,
                    );
                }
            }
            for bucket in &args.buckets {
                scope.insert(bucket.bind_as.clone());
            }
        }
        StepSpec::SelectMaterial(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectMaterials(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectOwnSources(args) | StepSpec::SelectUnderTamerSources(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            declare_optional_binding(&mut child, &args.bind_as);
            validate_steps_binding_scope(
                &args.then,
                &format!("{prefix}.then"),
                card_id,
                &mut child,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectOpponentSources(args) => {
            // G-DSL-SELECT-SOURCES-FORMULA-COUNT: binding refs inside the
            // formula bounds must resolve in the current scope (mirrors
            // `SelectCountCappedMulti`'s `max`).
            if let crate::step::CountBound::Formula { formula } = &args.min {
                validate_formula_binding_scope(
                    formula,
                    &format!("{prefix}.min.formula"),
                    card_id,
                    scope,
                    errors,
                );
            }
            if let crate::step::CountBound::Formula { formula } = &args.max {
                validate_formula_binding_scope(
                    formula,
                    &format!("{prefix}.max.formula"),
                    card_id,
                    scope,
                    errors,
                );
            }
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            declare_optional_binding(&mut child, &args.bind_as);
            validate_steps_binding_scope(
                &args.then,
                &format!("{prefix}.then"),
                card_id,
                &mut child,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::DigiBurst(args) => {
            let mut child = scope.clone();
            declare_optional_binding(&mut child, &args.bind_as);
            validate_steps_binding_scope(
                &args.then,
                &format!("{prefix}.then"),
                card_id,
                &mut child,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectOpponentDpBudget(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            declare_optional_binding(&mut child, &args.bind_as);
            validate_steps_binding_scope(
                &args.then,
                &format!("{prefix}.then"),
                card_id,
                &mut child,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        // Play-cost-budget sibling of the DP-budget branch above. Same
        // binding-scope shape: `bind_as` is visible inside `then` and (once)
        // after the step. Added alongside the `play_cost_budget: FormulaSpec`
        // widening (P-094 Destromon).
        StepSpec::SelectOpponentPlayCostBudget(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            declare_optional_binding(&mut child, &args.bind_as);
            validate_steps_binding_scope(
                &args.then,
                &format!("{prefix}.then"),
                card_id,
                &mut child,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectOwnBreedingPermanent(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            declare_optional_binding(&mut child, &args.bind_as);
            validate_steps_binding_scope(
                &args.then,
                &format!("{prefix}.then"),
                card_id,
                &mut child,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectUnionZone(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            if let Some(zf) = &args.zone_filters {
                for (label, pred) in [
                    ("hand", &zf.hand),
                    ("trash", &zf.trash),
                    ("material", &zf.material),
                ] {
                    if let Some(pred) = pred {
                        validate_predicate_binding_scope(
                            pred,
                            &format!("{prefix}.zone_filters.{label}"),
                            card_id,
                            scope,
                            errors,
                        );
                    }
                }
            }
            if let Some(pred) = &args.material_carrier_filter {
                validate_predicate_binding_scope(
                    pred,
                    &format!("{prefix}.material_carrier_filter"),
                    card_id,
                    scope,
                    errors,
                );
            }
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectOrderedPermutation(args) => {
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectCountCappedMulti(args) => {
            if let crate::step::CountBound::Formula { formula } = &args.max {
                validate_formula_binding_scope(
                    formula,
                    &format!("{prefix}.max.formula"),
                    card_id,
                    scope,
                    errors,
                );
            }
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SelectEffectChoice(args) => {
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::SearchOwnSecurityStack(args) => {
            validate_predicate_binding_scope(
                &args.filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
            let mut on_select_scope = scope.clone();
            declare_optional_binding(&mut on_select_scope, &args.bind_as);
            validate_steps_binding_scope(
                &args.on_select,
                &format!("{prefix}.on_select"),
                card_id,
                &mut on_select_scope,
                errors,
            );
            if let Some(no_match) = &args.on_no_match {
                let mut no_match_scope = scope.clone();
                validate_steps_binding_scope(
                    no_match,
                    &format!("{prefix}.on_no_match"),
                    card_id,
                    &mut no_match_scope,
                    errors,
                );
            }
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::RevealTopDeck(args) => {
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::PlayFromMaterials(args) => {
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::PlayFromHandFree(args) => {
            // The played permanent's `bind_as` becomes available to later
            // steps in the same body (e.g. `schedule_delete_played_at_turn_end`).
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::PlayFromRevealedFree(args) => {
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::PlayToken(args) => {
            // The played token's `bind_as` becomes available to later
            // steps in the same body (e.g. `schedule_delete_played_at_turn_end`).
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::PlayUnionBoundFree(args) => {
            // The `binding` must name an in-scope `select_union_zone` bind_as.
            report_if_undeclared_binding(
                &args.binding,
                &format!("{prefix}.binding"),
                card_id,
                scope,
                errors,
            );
            declare_optional_binding(scope, &args.bind_as);
        }
        StepSpec::TrashUnionBound(args) => {
            report_if_undeclared_binding(
                &args.binding,
                &format!("{prefix}.binding"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::BindPermanentProperty(args) => {
            scope.insert(args.bind_as.clone());
        }
        StepSpec::RawRust(raw) => {
            for binding in &raw.binds {
                scope.insert(binding.clone());
            }
        }
        StepSpec::If(i) => {
            if let Ok(condition) =
                serde_yml::from_value::<crate::predicate::PredicateSpec>(i.condition.clone())
            {
                validate_predicate_binding_scope(
                    &condition,
                    &format!("{prefix}.condition"),
                    card_id,
                    scope,
                    errors,
                );
            }
            let mut then_scope = scope.clone();
            validate_steps_binding_scope(
                &i.then,
                &format!("{prefix}.then"),
                card_id,
                &mut then_scope,
                errors,
            );
            if let Some(else_) = &i.else_ {
                let mut else_scope = scope.clone();
                validate_steps_binding_scope(
                    else_,
                    &format!("{prefix}.else"),
                    card_id,
                    &mut else_scope,
                    errors,
                );
            }
        }
        StepSpec::ForEach(args) => {
            validate_predicate_binding_scope(
                &args.over,
                &format!("{prefix}.over"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            child.insert(args.bind_as.clone());
            validate_steps_binding_scope(
                &args.body,
                &format!("{prefix}.body"),
                card_id,
                &mut child,
                errors,
            );
            scope.insert(args.bind_as.clone());
        }
        StepSpec::PerSelected(args) => {
            report_if_undeclared_binding(
                &args.selection,
                &format!("{prefix}.selection"),
                card_id,
                scope,
                errors,
            );
            let mut child = scope.clone();
            child.insert(args.bind_as.clone());
            validate_steps_binding_scope(
                &args.body,
                &format!("{prefix}.body"),
                card_id,
                &mut child,
                errors,
            );
            scope.insert(args.bind_as.clone());
        }
        StepSpec::ScheduleDelayed(args) => {
            let mut child = scope.clone();
            validate_steps_binding_scope(
                &args.body,
                &format!("{prefix}.body"),
                card_id,
                &mut child,
                errors,
            );
        }
        StepSpec::ScheduleDeletePlayedAtTurnEnd(args) => {
            // The `binding` must name an in-scope permanent binding produced
            // by an earlier free-play step (PUPPETS-G003).
            report_if_undeclared_binding(
                &args.binding,
                &format!("{prefix}.binding"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::AsSelectingPlayer(args) => {
            validate_steps_binding_scope(
                &args.body,
                &format!("{prefix}.body"),
                card_id,
                scope,
                errors,
            );
        }
        StepSpec::Optional(optional) => {
            validate_steps_binding_scope(
                &optional.0,
                &format!("{prefix}.optional"),
                card_id,
                scope,
                errors,
            );
        }
        _ => {}
    }
}

fn declare_optional_binding(scope: &mut BTreeSet<String>, binding: &Option<String>) {
    if let Some(binding) = binding {
        scope.insert(binding.clone());
    }
}

fn validate_modifier_value_binding_scope(
    value: &crate::formula::FormulaSpec,
    prefix: &str,
    card_id: &str,
    scope: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    // unify-dsl-scalar-and-comparators: `value` is a `FormulaSpec`; a bare
    // `Literal` has no bindings, so validating it directly is a no-op.
    validate_formula_binding_scope(value, prefix, card_id, scope, errors);
}

fn validate_predicate_binding_scope(
    pred: &crate::predicate::PredicateSpec,
    prefix: &str,
    card_id: &str,
    scope: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    for (field, dp) in [
        ("level_lte", &pred.level_lte),
        ("level_gte", &pred.level_gte),
        ("play_cost_lte", &pred.play_cost_lte),
        ("play_or_use_cost_lte", &pred.play_or_use_cost_lte),
        ("dp_eq", &pred.dp_eq),
        ("dp_lte", &pred.dp_lte),
        ("dp_gte", &pred.dp_gte),
        ("stack_size_lte", &pred.stack_size_lte),
        ("stack_size_gte", &pred.stack_size_gte),
        ("materials_count_lte", &pred.materials_count_lte),
        ("materials_count_gte", &pred.materials_count_gte),
        ("memory_lte", &pred.memory_lte),
        ("memory_gte", &pred.memory_gte),
        ("own_memory_lte", &pred.own_memory_lte),
        ("own_memory_gte", &pred.own_memory_gte),
        ("security_count_lte", &pred.security_count_lte),
        ("security_count_gte", &pred.security_count_gte),
    ] {
        if let Some(crate::predicate::DpConstraint::Formula(formula)) = dp {
            validate_formula_binding_scope(
                formula,
                &format!("{prefix}.{field}"),
                card_id,
                scope,
                errors,
            );
        }
    }

    for (field, binding) in [
        ("binding_exists", &pred.binding_exists),
        ("binding_present", &pred.binding_present),
        ("binding_absent", &pred.binding_absent),
    ] {
        if let Some(binding) = binding {
            report_if_undeclared_binding(
                binding,
                &format!("{prefix}.{field}"),
                card_id,
                scope,
                errors,
            );
        }
    }
    if let Some(bc) = &pred.binding_count_eq {
        report_if_undeclared_binding(
            &bc.binding,
            &format!("{prefix}.binding_count_eq.binding"),
            card_id,
            scope,
            errors,
        );
    }

    for (i, sub) in pred.all_of.iter().enumerate() {
        validate_predicate_binding_scope(
            sub,
            &format!("{prefix}.all_of[{i}]"),
            card_id,
            scope,
            errors,
        );
    }
    for (i, sub) in pred.any_of.iter().enumerate() {
        validate_predicate_binding_scope(
            sub,
            &format!("{prefix}.any_of[{i}]"),
            card_id,
            scope,
            errors,
        );
    }
    for (i, sub) in pred.none_of.iter().enumerate() {
        validate_predicate_binding_scope(
            sub,
            &format!("{prefix}.none_of[{i}]"),
            card_id,
            scope,
            errors,
        );
    }
    if let Some(sub) = &pred.not {
        validate_predicate_binding_scope(sub, &format!("{prefix}.not"), card_id, scope, errors);
    }
    if let Some(sub) = &pred.returned_card_matching {
        validate_predicate_binding_scope(
            sub,
            &format!("{prefix}.returned_card_matching"),
            card_id,
            scope,
            errors,
        );
    }
    if let Some(inh) = &pred.has_inherited {
        validate_predicate_binding_scope(
            inh,
            &format!("{prefix}.has_inherited"),
            card_id,
            scope,
            errors,
        );
    }
    for (field, ex) in [
        ("any_permanent", &pred.any_permanent),
        ("any_field_permanent", &pred.any_field_permanent),
        ("no_permanent", &pred.no_permanent),
        ("all_permanents", &pred.all_permanents),
    ] {
        if let Some(ex) = ex {
            validate_predicate_binding_scope(
                &ex.predicate,
                &format!("{prefix}.{field}"),
                card_id,
                scope,
                errors,
            );
        }
    }
    if let Some(agg) = &pred.count_lte {
        if let crate::predicate::DpConstraint::Formula(formula) = &agg.n {
            validate_formula_binding_scope(
                formula,
                &format!("{prefix}.count_lte.n"),
                card_id,
                scope,
                errors,
            );
        }
        validate_predicate_binding_scope(
            &agg.filter,
            &format!("{prefix}.count_lte.filter"),
            card_id,
            scope,
            errors,
        );
    }
    if let Some(agg) = &pred.count_gte {
        if let crate::predicate::DpConstraint::Formula(formula) = &agg.n {
            validate_formula_binding_scope(
                formula,
                &format!("{prefix}.count_gte.n"),
                card_id,
                scope,
                errors,
            );
        }
        validate_predicate_binding_scope(
            &agg.filter,
            &format!("{prefix}.count_gte.filter"),
            card_id,
            scope,
            errors,
        );
    }
}

fn validate_formula_binding_scope(
    formula: &crate::formula::FormulaSpec,
    prefix: &str,
    card_id: &str,
    scope: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    use crate::formula::{CompoundFormula, FormulaSpec};

    match formula {
        FormulaSpec::BindingDp { binding_dp } => {
            report_if_undeclared_binding(binding_dp, prefix, card_id, scope, errors);
        }
        FormulaSpec::BindingPlayCost { binding_play_cost } => {
            report_if_undeclared_binding(binding_play_cost, prefix, card_id, scope, errors);
        }
        FormulaSpec::BindingValue { binding_value } => {
            report_if_undeclared_binding(binding_value, prefix, card_id, scope, errors);
        }
        FormulaSpec::BasePerDelta { per, .. } => {
            validate_per_selector_binding_scope(
                per,
                &format!("{prefix}.per"),
                card_id,
                scope,
                errors,
            );
        }
        FormulaSpec::SourceStackDpSum {
            source_stack_dp_sum,
        } => {
            report_if_undeclared_binding(
                &source_stack_dp_sum.target,
                &format!("{prefix}.source_stack_dp_sum.target"),
                card_id,
                scope,
                errors,
            );
            if let Some(filter) = &source_stack_dp_sum.filter {
                validate_predicate_binding_scope(
                    filter,
                    &format!("{prefix}.source_stack_dp_sum.filter"),
                    card_id,
                    scope,
                    errors,
                );
            }
        }
        FormulaSpec::SourceStackCount {
            source_stack_count,
        } => {
            report_if_undeclared_binding(
                &source_stack_count.target,
                &format!("{prefix}.source_stack_count.target"),
                card_id,
                scope,
                errors,
            );
            if let Some(filter) = &source_stack_count.filter {
                validate_predicate_binding_scope(
                    filter,
                    &format!("{prefix}.source_stack_count.filter"),
                    card_id,
                    scope,
                    errors,
                );
            }
        }
        FormulaSpec::Compound(CompoundFormula::FloorDiv(args))
        | FormulaSpec::Compound(CompoundFormula::Max(args))
        | FormulaSpec::Compound(CompoundFormula::Min(args)) => {
            for (i, arg) in args.iter().enumerate() {
                validate_formula_binding_scope(
                    arg,
                    &format!("{prefix}[{i}]"),
                    card_id,
                    scope,
                    errors,
                );
            }
        }
        FormulaSpec::Literal(_)
        | FormulaSpec::SourceDp { .. }
        | FormulaSpec::SourceMaterialCount { .. }
        | FormulaSpec::EventTargetLevel { .. }
        | FormulaSpec::SourceColorCount { .. }
        | FormulaSpec::Compound(CompoundFormula::Aggregate(_))
        | FormulaSpec::Compound(CompoundFormula::AggregateScoped(_))
        | FormulaSpec::Compound(CompoundFormula::RawRust(_)) => {}
    }
}

fn validate_per_selector_binding_scope(
    per: &crate::formula::PerSelector,
    prefix: &str,
    card_id: &str,
    scope: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    if let crate::formula::PerSelector::CardCountInZone(spec)
    | crate::formula::PerSelector::DistinctColorsCount(spec) = per
    {
        if let Some(filter) = &spec.filter {
            validate_predicate_binding_scope(
                filter,
                &format!("{prefix}.filter"),
                card_id,
                scope,
                errors,
            );
        }
    }
}

fn report_if_undeclared_binding(
    binding: &str,
    path: &str,
    card_id: &str,
    scope: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    if !scope.contains(binding) {
        errors.push(ValidationError {
            card_id: card_id.into(),
            path: path.into(),
            message: format!("undeclared binding referenced before declaration: {binding}"),
        });
    }
}

fn validate_binding_ref(
    binding_ref: &BindingRef,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) {
    let BindingRef::Structured(StructuredBindingRef {
        binding,
        permanent,
        source_permanent,
        of_permanent,
        deck_top,
        own_breeding,
        ..
    }) = binding_ref
    else {
        return;
    };

    if source_permanent.is_some() {
        errors.push(ValidationError {
            card_id: card_id.into(),
            path: prefix.into(),
            message: "source_permanent binding refs are not supported here".into(),
        });
    }

    let populated = [
        binding.is_some(),
        permanent.is_some(),
        of_permanent.is_some(),
        deck_top.is_some(),
        matches!(own_breeding, Some(true)),
    ]
    .iter()
    .filter(|present| **present)
    .count();
    if populated == 0 {
        errors.push(ValidationError {
            card_id: card_id.into(),
            path: prefix.into(),
            message: "binding ref must name a binding".into(),
        });
    } else if populated > 1 {
        errors.push(ValidationError {
            card_id: card_id.into(),
            path: prefix.into(),
            message: "binding ref must use only one binding field".into(),
        });
    }
}

fn validate_modifier_value(
    value: &crate::formula::FormulaSpec,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    // unify-dsl-scalar-and-comparators: `value` is a `FormulaSpec`; validating
    // a bare `Literal` is a no-op, so call the formula validator directly.
    validate_formula(value, prefix, card_id, ctx, errors);
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
        FormulaSpec::BasePerDelta { per, .. } => {
            validate_per_selector(per, &format!("{prefix}.per"), card_id, ctx, errors);
        }
        FormulaSpec::SourceStackDpSum {
            source_stack_dp_sum,
        } => {
            if let Some(filter) = &source_stack_dp_sum.filter {
                validate_predicate(
                    filter,
                    &format!("{prefix}.source_stack_dp_sum.filter"),
                    card_id,
                    ctx,
                    errors,
                );
            }
        }
        FormulaSpec::SourceStackCount {
            source_stack_count,
        } => {
            if let Some(filter) = &source_stack_count.filter {
                validate_predicate(
                    filter,
                    &format!("{prefix}.source_stack_count.filter"),
                    card_id,
                    ctx,
                    errors,
                );
            }
        }
        FormulaSpec::Literal(_)
        | FormulaSpec::SourceDp { .. }
        | FormulaSpec::SourceMaterialCount { .. }
        | FormulaSpec::EventTargetLevel { .. }
        | FormulaSpec::SourceColorCount { .. }
        | FormulaSpec::BindingDp { .. }
        | FormulaSpec::BindingValue { .. }
        | FormulaSpec::BindingPlayCost { .. }
        | FormulaSpec::Compound(CompoundFormula::Aggregate(_))
        | FormulaSpec::Compound(CompoundFormula::AggregateScoped(_)) => {}
    }
}

fn validate_per_selector(
    per: &crate::formula::PerSelector,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    if let crate::formula::PerSelector::CardCountInZone(spec)
    | crate::formula::PerSelector::DistinctColorsCount(spec) = per
    {
        if let Some(filter) = &spec.filter {
            validate_predicate(filter, &format!("{prefix}.filter"), card_id, ctx, errors);
        }
    }
    if let crate::formula::PerSelector::SourceStackCount(spec) = per {
        if let Some(filter) = &spec.filter {
            validate_predicate(filter, &format!("{prefix}.filter"), card_id, ctx, errors);
        }
    }
}

fn formula_uses_dp_aggregate(formula: &crate::formula::FormulaSpec) -> bool {
    use crate::formula::{AggregateSelector, CompoundFormula, FormulaSpec};

    match formula {
        FormulaSpec::Compound(CompoundFormula::Aggregate(
            AggregateSelector::HighestDp | AggregateSelector::LowestDp,
        )) => true,
        FormulaSpec::Compound(CompoundFormula::AggregateScoped(spec)) => {
            matches!(
                spec.selector,
                AggregateSelector::HighestDp | AggregateSelector::LowestDp
            )
        }
        FormulaSpec::Compound(
            CompoundFormula::FloorDiv(args)
            | CompoundFormula::Max(args)
            | CompoundFormula::Min(args),
        ) => args.iter().any(formula_uses_dp_aggregate),
        FormulaSpec::BasePerDelta { per, .. } => per_uses_dp_aggregate(per),
        FormulaSpec::SourceStackCount { .. } | FormulaSpec::SourceStackDpSum { .. } => false,
        FormulaSpec::Literal(_)
        | FormulaSpec::SourceDp { .. }
        | FormulaSpec::SourceMaterialCount { .. }
        | FormulaSpec::EventTargetLevel { .. }
        | FormulaSpec::SourceColorCount { .. }
        | FormulaSpec::BindingDp { .. }
        | FormulaSpec::BindingValue { .. }
        | FormulaSpec::BindingPlayCost { .. }
        | FormulaSpec::Compound(CompoundFormula::Aggregate(_))
        | FormulaSpec::Compound(CompoundFormula::RawRust(_)) => false,
    }
}

fn per_uses_dp_aggregate(per: &crate::formula::PerSelector) -> bool {
    match per {
        crate::formula::PerSelector::CardCountInZone(spec)
        | crate::formula::PerSelector::DistinctColorsCount(spec) => spec
            .filter
            .as_deref()
            .is_some_and(predicate_uses_dp_aggregate),
        _ => false,
    }
}

fn predicate_uses_dp_aggregate(pred: &crate::predicate::PredicateSpec) -> bool {
    [&pred.dp_eq, &pred.dp_lte, &pred.dp_gte]
        .into_iter()
        .flatten()
        .any(|dp| {
            matches!(
                dp,
                crate::predicate::DpConstraint::Formula(formula)
                    if formula_uses_dp_aggregate(formula)
            )
        })
        || pred
            .all_of
            .iter()
            .chain(&pred.any_of)
            .chain(&pred.none_of)
            .any(predicate_uses_dp_aggregate)
        || pred.not.as_deref().is_some_and(predicate_uses_dp_aggregate)
        || pred
            .has_inherited
            .as_deref()
            .is_some_and(predicate_uses_dp_aggregate)
        || [
            &pred.any_permanent,
            &pred.any_field_permanent,
            &pred.no_permanent,
            &pred.all_permanents,
        ]
        .into_iter()
        .flatten()
        .any(|ex| predicate_uses_dp_aggregate(&ex.predicate))
        || [&pred.count_lte, &pred.count_gte]
            .into_iter()
            .flatten()
            .any(|agg| predicate_uses_dp_aggregate(&agg.filter))
}

fn is_known_modifier(name: &str) -> bool {
    KNOWN_MODIFIER_KEYS.iter().any(|k| *k == name)
}

/// Every modifier name the validator accepts. Must stay in lockstep with
/// `digimon_engine::dsl_cards::modifier_map::lookup_modifier_type`; the
/// parity test `validator_keys_match_engine_table` in the engine's
/// `modifier_map` module fails the build if the two diverge. Drift here
/// means the validator wrongly rejects YAML the engine would handle, or
/// vice-versa.
pub const KNOWN_MODIFIER_KEYS: &[&str] = &[
    // DP / cost / metadata
    "ChangeDp",
    "ChangeBaseDp",
    "DpFloor",
    "DontHaveDp",
    "ChangePlayCost",
    "ChangeDigivolveCost",
    "CannotReduceCost",
    // Destruction / removal protection
    "CannotBeDestroyed",
    "CannotBeDestroyedByBattle",
    "CannotBeDestroyedByEffect",
    "CannotBeRemoved",
    "CannotBeReturnedToDeck",
    "CannotBeReturnedToHand",
    "CannotBeTrashedByEffect",
    "CannotBeDeDigivolved",
    // Attack restrictions / grants
    "CannotAttack",
    "CannotAttackPlayer",
    "VortexCanAttackPlayer",
    "CanAttackUnsuspended",
    "CanAttackActivePlayer",
    "CannotAttackTarget",
    "CannotBeAttackedBySecurityAttackChanged",
    // Suspend / select / affect
    "CannotSuspend",
    "CannotUnsuspend",
    "CannotBeSelectedByEffect",
    "CannotBeAffected",
    // Granted keywords
    "GrantBlocker",
    "GrantRush",
    "GrantJamming",
    "GrantPiercing",
    "GrantReboot",
    "GrantBlitz",
    "GrantAlliance",
    "GrantRaid",
    "GrantDecoy",
    "GrantVortex",
    "GrantOverclock",
    // End-of-turn attack / security
    "MayAttack",
    "ForceAttack",
    "SecurityAttackChange",
    "ChangeOwnSecurityDigimonDp",
    "SecurityDpChange",
    "ImmunityToOpponentEffects",
    "DontBattleSecurityDigimon",
    // Digivolution / color / level
    "CannotDigivolve",
    "CanOnlyDigivolveInto",
    "ChangeColor",
    "AddColor",
    "ChangeLevel",
    // Misc gates
    "CannotReturnToHand",
    "CannotTrash",
    "CannotBlock",
    "CannotCounter",
    "DrawBlock",
    "MemoryBlock",
    "CannotPlayFromHand",
    // Phase 6 player-scoped flood gates
    "CannotPlayDigimonByEffect",
    "CannotPlayTamerByEffect",
    "CannotGainMemoryByEffect",
    "CannotGainMemoryExceptFromTamers",
    "CannotPlayFromTrash",
    "CannotReducePlayCost",
    "CannotReduceDigivolveCost",
    "OpponentCannotReduceDigivolveCost",
    "CannotActivateOnPlayEffects",
    "CannotActivateMainEffects",
    "CannotActivateWhenDigivolvingEffects",
    "CannotActivateWhenAttackingEffects",
    "CannotActivateSecurityEffects",
    "CannotDigivolveDigimonByEffect",
    "CannotDrawByEffect",
    "CannotAddSecurityByEffect",
    "CannotTrashOpponentSecurity",
    "CannotReduceOpponentSecurity",
    "IgnoreColorRequirement",
    // Track C taxonomy completion (2026-05-06)
    "MayAttackPlayerOnly",
    "CannotMove",
    "CannotSwitchAttackTarget",
    "CanNotSwitchAttackTarget",
    "CannotBeRedirectedAsAttackTarget",
    "CanAttackTargetDefendingPermanent",
    "CannotAddMemory",
    "CannotAddSecurity",
    "ChangeEndTurnMinMemory",
    "ImmuneFromDPMinus",
    "ImmuneFromStackTrashing",
    "DisableEffect",
    "TreatAsDigimon",
    "ChangeCardDP",
    "ChangeOriginDP",
    "ChangeSAttack",
    "ChangeLinkCost",
    "ChangeLinkMax",
    "ChangePermanentLevel",
    "ChangeTraits",
    "ChangeBaseCardName",
    "SourceNameAliases",
    "ChangeBaseCardColor",
    "ChangeCardLevelForAssembly",
    "ChangeCardNamesForDigiXros",
];

fn is_permanent_activation_modifier(name: &str) -> bool {
    matches!(
        name,
        "CannotActivateOnPlayEffects"
            | "CannotActivateWhenDigivolvingEffects"
            | "CannotActivateWhenAttackingEffects"
    )
}

fn is_known_keyword(name: &str) -> bool {
    KNOWN_KEYWORD_KEYS.iter().any(|k| *k == name)
}

/// Every keyword name the validator accepts. Must stay in lockstep with
/// `digimon_engine::dsl_cards::modifier_map::lookup_keyword`. `Delay` is
/// validator-only — it's a clause-kind sigil rather than a runtime
/// `Keyword` enum variant — so the engine-side parity test allowlists it.
pub const KNOWN_KEYWORD_KEYS: &[&str] = &[
    "Blocker",
    "SecurityAttackPlus",
    "SecurityAttackMinus",
    "Rush",
    "Jamming",
    "Piercing",
    "Reboot",
    "DeDigivolve",
    "DrawX",
    "Blitz",
    "Raid",
    "Alliance",
    "BlastDigivolve",
    "Save",
    "Fortitude",
    "Ascension",
    "Overclock",
    "Barrier",
    "Decoy",
    "Partition",
    "Vortex",
    "Collision",
    "Progress",
    "Evade",
    "Iceclad",
    "MaterialSave",
    "DigiBurst",
    "Decode",
    "ArmorPurge",
    "Fragment",
    "Retaliation",
    // Validator-only sigil (engine side dispatches via clause kind, not a
    // runtime `Keyword` variant). Allowlisted on the engine parity test.
    "Delay",
];

/// Every snake_case expiry key the validator accepts. Must stay in lockstep
/// with `digimon_engine::dsl_cards::expiry_map::all_engine_expiry_keys()`;
/// the parity test in `code/digimon-engine/tests/dsl/expiry_parity.rs`
/// fails the build if the two lists diverge.
pub const KNOWN_EXPIRY_KEYS: &[&str] = &[
    "permanent",
    "end_of_turn",
    "end_of_opponents_turn",
    "end_of_your_turn",
    "end_of_opponents_next_turn",
    "end_of_your_next_turn",
    "end_of_attack",
    "end_of_battle",
    "until_leave_field",
    "until_condition",
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

    #[test]
    fn battle_step_rejects_source_permanent_binding_ref_until_compiler_lowers_it() {
        let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 6
color: [green]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - battle:
          attacker: { source_permanent: picked }
          defender: target
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs.iter().any(|e| {
            e.path.ends_with(".attacker")
                && e.message
                    .contains("source_permanent binding refs are not supported here")
        }));
    }

    #[test]
    fn filtered_count_predicate_raw_rust_is_validated() {
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
      base: 0
      per:
        card_count_in_zone:
          zone: trash
          of: any
          filter:
            dp_lte:
              formula:
                raw_rust: unregistered_filtered_formula_fn
      delta: 1
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message.contains("unregistered_filtered_formula_fn")));
    }

    #[test]
    fn level_matches_aggregate_rejects_dp_aggregate_selector() {
        let yaml = r#"
card: X-1
name: Test
kind: option
color: [red]
cost: 3
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: target
          prompt: Pick
          filter:
            level_matches_aggregate:
              selector: lowest_dp
              of: opponent
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.path.contains("level_matches_aggregate.selector")));
    }

    #[test]
    fn binding_formula_rejects_reference_before_bind_as_declaration() {
        let yaml = r#"
card: X-1
name: Test
kind: option
color: [black]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick
          filter:
            kind: digimon
            play_cost_lte:
              formula:
                binding_play_cost: source_digimon
      - select_own_permanent:
          bind_as: source_digimon
          prompt: Source
          filter: { kind: digimon }
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs.iter().any(|e| {
            e.path.ends_with(".play_cost_lte")
                && e.message
                    .contains("undeclared binding referenced before declaration: source_digimon")
        }));
    }

    #[test]
    fn binding_formula_accepts_reference_after_bind_as_declaration() {
        let yaml = r#"
card: X-1
name: Test
kind: option
color: [black]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          bind_as: source_digimon
          prompt: Source
          filter: { kind: digimon }
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick
          filter:
            kind: digimon
            play_cost_lte:
              formula:
                binding_play_cost: source_digimon
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        assert!(validate(&spec, &ValidationContext { raw_rust: &reg }).is_ok());
    }

    #[test]
    fn treat_as_digimon_without_synth_identity_is_rejected() {
        let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
effects:
  - when: when_digivolving
    process:
      - add_modifier:
          target: self
          modifier: TreatAsDigimon
          value: 0
          expiry: end_of_turn
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message.contains("TreatAsDigimon requires a synth_identity")));
    }

    #[test]
    fn synth_identity_on_non_treat_as_digimon_is_rejected() {
        let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
effects:
  - when: when_digivolving
    process:
      - add_modifier:
          target: self
          modifier: CannotDigivolve
          value: 0
          expiry: end_of_turn
          synth_identity:
            dp: 3000
"#;
        let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
        let reg = StubRegistry::empty();
        let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
        assert!(errs.iter().any(|e| e
            .message
            .contains("synth_identity is only valid for modifier TreatAsDigimon")));
    }
}
