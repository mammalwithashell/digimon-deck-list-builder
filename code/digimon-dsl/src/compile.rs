//! CardSpec → CompiledCard lowering. Pure function over authored data;
//! no engine types touched.

use crate::compiled::*;
use crate::errors::ValidationError;
use crate::spec::CardSpec;

/// Compile a parsed `CardSpec` to the bincode-compatible `CompiledCard` IR.
/// Errors accumulate into a `Vec<ValidationError>`, analogous to the
/// semantic validator — compile is strictly more demanding since it must
/// resolve every shape to a concrete type.
pub fn compile(spec: &CardSpec) -> Result<CompiledCard, Vec<ValidationError>> {
    let mut errors = Vec::new();

    let identity = spec.identity.as_ref().map(compile_identity);
    let alt_paths = spec
        .alt_paths
        .iter()
        .enumerate()
        .map(|(i, ap)| compile_alt_path(ap, &format!("alt_paths[{i}]"), &spec.card, &mut errors))
        .collect();
    let effects = spec
        .effects
        .iter()
        .enumerate()
        .map(|(i, c)| compile_clause(c, &format!("effects[{i}]"), &spec.card, &mut errors))
        .collect();

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(CompiledCard {
        card: spec.card.clone(),
        name: spec.name.clone(),
        kind: compile_card_kind(spec.kind),
        level: spec.level,
        color: spec.color.iter().map(|c| compile_color(*c)).collect(),
        cost: spec.cost,
        dp: spec.dp,
        traits: spec.traits.clone(),
        form: spec.form.clone(),
        attribute: spec.attribute.clone(),
        ace_overflow: spec.ace_overflow,
        identity,
        alt_paths,
        effects,
    })
}

// ── Enum mappings ───────────────────────────────────────────────────

fn compile_card_kind(k: crate::spec::CardKind) -> CompiledCardKind {
    use crate::spec::CardKind as S;
    match k {
        S::Digimon => CompiledCardKind::Digimon,
        S::Tamer => CompiledCardKind::Tamer,
        S::Option => CompiledCardKind::Option,
        S::DigiEgg => CompiledCardKind::DigiEgg,
        S::Token => CompiledCardKind::Token,
    }
}

fn compile_color(c: crate::spec::ColorSpec) -> CompiledColor {
    use crate::spec::ColorSpec as S;
    match c {
        S::Red => CompiledColor::Red,
        S::Blue => CompiledColor::Blue,
        S::Yellow => CompiledColor::Yellow,
        S::Green => CompiledColor::Green,
        S::Black => CompiledColor::Black,
        S::Purple => CompiledColor::Purple,
        S::White => CompiledColor::White,
    }
}

fn compile_player_ref(p: crate::common::PlayerRef) -> CompiledPlayerRef {
    use crate::common::PlayerRef as S;
    match p {
        S::You => CompiledPlayerRef::You,
        S::Opponent => CompiledPlayerRef::Opponent,
        S::Any => CompiledPlayerRef::Any,
        S::Active => CompiledPlayerRef::Active,
    }
}

fn compile_zone(z: crate::predicate::Zone) -> CompiledZone {
    use crate::predicate::Zone as S;
    match z {
        S::Hand => CompiledZone::Hand,
        S::Deck => CompiledZone::Deck,
        S::Trash => CompiledZone::Trash,
        S::BattleArea => CompiledZone::BattleArea,
        S::Security => CompiledZone::Security,
        S::Breeding => CompiledZone::Breeding,
        S::Reveal => CompiledZone::Reveal,
        S::DigiEggDeck => CompiledZone::DigiEggDeck,
        S::Material => CompiledZone::Material,
    }
}

fn compile_scope(s: crate::clause::ClauseScope) -> CompiledScope {
    use crate::clause::ClauseScope as S;
    match s {
        S::FaceUp => CompiledScope::FaceUp,
        S::Inherited => CompiledScope::Inherited,
        S::Both => CompiledScope::Both,
    }
}

fn compile_timing(t: crate::clause::Timing) -> CompiledTiming {
    use crate::clause::Timing as S;
    match t {
        S::OnPlay => CompiledTiming::OnPlay,
        S::WhenDigivolving => CompiledTiming::WhenDigivolving,
        S::WhenAttacking => CompiledTiming::WhenAttacking,
        S::EndOfAttack => CompiledTiming::EndOfAttack,
        S::EndOfBattle => CompiledTiming::EndOfBattle,
        S::OnAttack => CompiledTiming::OnAttack,
        S::OnDeletion => CompiledTiming::OnDeletion,
        S::OnAnyDeletion => CompiledTiming::OnAnyDeletion,
        S::OnEnterFieldAnyone => CompiledTiming::OnEnterFieldAnyone,
        S::OnAllyPlayed => CompiledTiming::OnAllyPlayed,
        S::OnLeaveField => CompiledTiming::OnLeaveField,
        S::OnSuspend => CompiledTiming::OnSuspend,
        S::OnUnsuspend => CompiledTiming::OnUnsuspend,
        S::OnHatch => CompiledTiming::OnHatch,
        S::OnDigivolve => CompiledTiming::OnDigivolve,
        S::OnDnaDigivolve => CompiledTiming::OnDnaDigivolve,
        S::OnDigixros => CompiledTiming::OnDigixros,
        S::OnOpponentSecurityRemoved => CompiledTiming::OnOpponentSecurityRemoved,
        S::OnDigivolutionCardTrashed => CompiledTiming::OnDigivolutionCardTrashed,
        S::OnSecurityCheck => CompiledTiming::OnSecurityCheck,
        S::OnLoseSecurity => CompiledTiming::OnLoseSecurity,
        S::OnSecurity => CompiledTiming::OnSecurity,
        S::OnOptionPlaced => CompiledTiming::OnOptionPlaced,
        S::StartOfYourTurn => CompiledTiming::StartOfYourTurn,
        S::StartOfOpponentsTurn => CompiledTiming::StartOfOpponentsTurn,
        S::StartOfYourMainPhase => CompiledTiming::StartOfYourMainPhase,
        S::EndOfYourTurn => CompiledTiming::EndOfYourTurn,
        S::EndOfOpponentsTurn => CompiledTiming::EndOfOpponentsTurn,
        S::OnAttackTargetChange => CompiledTiming::OnAttackTargetChange,
        S::MainFromHand => CompiledTiming::MainFromHand,
        S::MainOnField => CompiledTiming::MainOnField,
        S::MainFromTrash => CompiledTiming::MainFromTrash,
        S::Counter => CompiledTiming::Counter,
        S::BeforePayCost => CompiledTiming::BeforePayCost,
        S::Delayed => CompiledTiming::Delayed,
    }
}

fn compile_stack_position(p: crate::step::StackPosition) -> CompiledStackPosition {
    use crate::step::StackPosition as S;
    match p {
        S::Top => CompiledStackPosition::Top,
        S::Bottom => CompiledStackPosition::Bottom,
        S::Random => CompiledStackPosition::Random,
    }
}

fn compile_distinct_by(d: crate::alt_path::DistinctBy) -> CompiledDistinctBy {
    use crate::alt_path::DistinctBy as S;
    match d {
        S::CardNumber => CompiledDistinctBy::CardNumber,
        S::Level => CompiledDistinctBy::Level,
        S::Name => CompiledDistinctBy::Name,
    }
}

fn compile_per_selector(p: crate::formula::PerSelector) -> CompiledPerSelector {
    use crate::formula::PerSelector as S;
    match p {
        S::MaterialCount => CompiledPerSelector::MaterialCount,
        S::StackSize => CompiledPerSelector::StackSize,
        S::AllyCount => CompiledPerSelector::AllyCount,
        S::DigivolutionColorCount => CompiledPerSelector::DigivolutionColorCount,
        S::CardCountInZone { of, zone } => CompiledPerSelector::CardCountInZone {
            of: compile_player_ref(of),
            zone: compile_zone(zone),
        },
    }
}

fn compile_aggregate_selector(a: crate::formula::AggregateSelector) -> CompiledAggregateSelector {
    use crate::formula::AggregateSelector as S;
    match a {
        S::LowestDp => CompiledAggregateSelector::LowestDp,
        S::HighestDp => CompiledAggregateSelector::HighestDp,
        S::LowestLevel => CompiledAggregateSelector::LowestLevel,
        S::HighestLevel => CompiledAggregateSelector::HighestLevel,
    }
}

fn compile_cost_delta(c: &crate::step::CostDelta) -> CompiledCostDelta {
    use crate::step::{CostDelta, CostDeltaKeyword};
    match c {
        CostDelta::Keyword(CostDeltaKeyword::Free) => CompiledCostDelta::Free,
        CostDelta::Keyword(CostDeltaKeyword::Printed) => CompiledCostDelta::Printed,
        CostDelta::Literal(n) => CompiledCostDelta::Literal(*n),
        CostDelta::Reduce { reduce } => CompiledCostDelta::Reduce(*reduce),
    }
}

// ── Identity ────────────────────────────────────────────────────────

fn compile_identity(id: &crate::identity::IdentitySpec) -> CompiledIdentity {
    CompiledIdentity {
        name_aliases: id.name_aliases.iter().map(compile_name_alias).collect(),
    }
}

fn compile_name_alias(a: &crate::identity::NameAliasSpec) -> CompiledNameAlias {
    CompiledNameAlias {
        treat_as: a.treat_as.clone(),
        zone: a.when.zone.iter().map(|z| compile_zone(*z)).collect(),
        has_inherited_card_number: a
            .when
            .has_inherited
            .as_ref()
            .and_then(|i| i.card_number_is.clone()),
        has_inherited_name: a
            .when
            .has_inherited
            .as_ref()
            .and_then(|i| i.name_is.clone()),
    }
}

// ── Formula ─────────────────────────────────────────────────────────

fn compile_formula(f: &crate::formula::FormulaSpec) -> CompiledFormula {
    use crate::formula::{CompoundFormula, FormulaSpec};
    match f {
        FormulaSpec::Literal(n) => CompiledFormula::Literal(*n),
        FormulaSpec::BasePerDelta { base, per, delta } => CompiledFormula::BasePerDelta {
            base: *base,
            per: compile_per_selector(*per),
            delta: *delta,
        },
        FormulaSpec::Compound(CompoundFormula::FloorDiv(v)) => {
            CompiledFormula::FloorDiv(v.iter().map(compile_formula).collect())
        }
        FormulaSpec::Compound(CompoundFormula::Max(v)) => {
            CompiledFormula::Max(v.iter().map(compile_formula).collect())
        }
        FormulaSpec::Compound(CompoundFormula::Min(v)) => {
            CompiledFormula::Min(v.iter().map(compile_formula).collect())
        }
        FormulaSpec::Compound(CompoundFormula::Aggregate(a)) => {
            CompiledFormula::Aggregate(compile_aggregate_selector(*a))
        }
        FormulaSpec::Compound(CompoundFormula::RawRust(s)) => CompiledFormula::RawRust(s.clone()),
    }
}

// ── Predicate ───────────────────────────────────────────────────────

fn compile_predicate(
    p: &crate::predicate::PredicateSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledPredicate {
    // Unknown fields in `extra` are silently dropped here — they are absorbed
    // by serde's flatten+unknown-fields mechanism and flagged by the semantic
    // validator (Task 12), not by the compiler.

    CompiledPredicate {
        kind: p.kind.map(compile_card_kind),
        level_eq: p.level_eq,
        level_lte: p.level_lte,
        level_gte: p.level_gte,
        color_is: p.color_is.map(compile_color),
        color_only: p
            .color_only
            .as_ref()
            .map(|v| v.iter().map(|c| compile_color(*c)).collect()),
        trait_has: p.trait_has.clone(),
        form_is: p.form_is.clone(),
        attribute_is: p.attribute_is.clone(),
        name_is: p.name_is.clone(),
        name_contains: p.name_contains.clone(),
        name_in: p.name_in.clone(),
        card_number_is: p.card_number_is.clone(),
        dp_eq: p.dp_eq.as_ref().map(compile_dp_constraint),
        dp_lte: p.dp_lte.as_ref().map(compile_dp_constraint),
        dp_gte: p.dp_gte.as_ref().map(compile_dp_constraint),
        stack_size_lte: p.stack_size_lte,
        stack_size_gte: p.stack_size_gte,
        materials_count_lte: p.materials_count_lte,
        materials_count_gte: p.materials_count_gte,
        has_inherited: p.has_inherited.as_ref().map(|b| {
            Box::new(compile_predicate(
                b,
                &format!("{prefix}.has_inherited"),
                card_id,
                errors,
            ))
        }),
        is_suspended: p.is_suspended,
        is_unsuspended: p.is_unsuspended,
        has_keyword: p.has_keyword.clone(),
        zone: p.zone.iter().map(|z| compile_zone(*z)).collect(),
        owner: p.owner.map(compile_player_ref),
        other: p.other,
        of_permanent: p.of_permanent.clone(),
        source_is_tamer: p.source_is_tamer,
        source_name_contains: p.source_name_contains.clone(),
        source_permanent_trait_has: p.source_permanent_trait_has.clone(),
        memory_lte: p.memory_lte,
        memory_gte: p.memory_gte,
        security_count_lte: p.security_count_lte,
        security_count_gte: p.security_count_gte,
        your_turn: p.your_turn,
        opponents_turn: p.opponents_turn,
        all_turns: p.all_turns,
        in_breeding: p.in_breeding,
        on_field: p.on_field,
        dna_origin: p.dna_origin,
        event_target_kind: p.event_target_kind.map(compile_card_kind),
        event_target_trait_has: p.event_target_trait_has.clone(),
        event_card_trait_has: p.event_card_trait_has.clone(),
        equals: p
            .equals
            .as_ref()
            .map(|v| v.iter().map(compile_binding_compare).collect()),
        not_equals: p
            .not_equals
            .as_ref()
            .map(|v| v.iter().map(compile_binding_compare).collect()),
        count_lte: p.count_lte.as_ref().map(|c| CompiledCountAggregate {
            filter: Box::new(compile_predicate(
                &c.filter,
                &format!("{prefix}.count_lte.filter"),
                card_id,
                errors,
            )),
            n: c.n,
        }),
        count_gte: p.count_gte.as_ref().map(|c| CompiledCountAggregate {
            filter: Box::new(compile_predicate(
                &c.filter,
                &format!("{prefix}.count_gte.filter"),
                card_id,
                errors,
            )),
            n: c.n,
        }),
        any_permanent: p.any_permanent.as_ref().map(|e| {
            Box::new(CompiledExistential {
                of: compile_player_ref(e.of),
                predicate: compile_predicate(
                    &e.predicate,
                    &format!("{prefix}.any_permanent"),
                    card_id,
                    errors,
                ),
            })
        }),
        no_permanent: p.no_permanent.as_ref().map(|e| {
            Box::new(CompiledExistential {
                of: compile_player_ref(e.of),
                predicate: compile_predicate(
                    &e.predicate,
                    &format!("{prefix}.no_permanent"),
                    card_id,
                    errors,
                ),
            })
        }),
        all_permanents: p.all_permanents.as_ref().map(|e| {
            Box::new(CompiledExistential {
                of: compile_player_ref(e.of),
                predicate: compile_predicate(
                    &e.predicate,
                    &format!("{prefix}.all_permanents"),
                    card_id,
                    errors,
                ),
            })
        }),
        all_of: p
            .all_of
            .iter()
            .enumerate()
            .map(|(i, sub)| {
                compile_predicate(sub, &format!("{prefix}.all_of[{i}]"), card_id, errors)
            })
            .collect(),
        any_of: p
            .any_of
            .iter()
            .enumerate()
            .map(|(i, sub)| {
                compile_predicate(sub, &format!("{prefix}.any_of[{i}]"), card_id, errors)
            })
            .collect(),
        none_of: p
            .none_of
            .iter()
            .enumerate()
            .map(|(i, sub)| {
                compile_predicate(sub, &format!("{prefix}.none_of[{i}]"), card_id, errors)
            })
            .collect(),
        not: p.not.as_ref().map(|b| {
            Box::new(compile_predicate(
                b,
                &format!("{prefix}.not"),
                card_id,
                errors,
            ))
        }),
        has_alt_path: p.has_alt_path.clone(),
    }
}

fn compile_dp_constraint(d: &crate::predicate::DpConstraint) -> CompiledDpConstraint {
    use crate::predicate::DpConstraint as S;
    match d {
        S::Literal(n) => CompiledDpConstraint::Literal(*n),
        S::Formula(f) => CompiledDpConstraint::Formula(compile_formula(f)),
    }
}

fn compile_binding_compare(v: &serde_yml::Value) -> CompiledBindingCompare {
    match v {
        serde_yml::Value::String(s) => CompiledBindingCompare::Binding(s.clone()),
        serde_yml::Value::Number(n) => CompiledBindingCompare::Literal(n.as_i64().unwrap_or(0)),
        _ => CompiledBindingCompare::Literal(0),
    }
}

// ── Alt paths ───────────────────────────────────────────────────────

fn compile_alt_path(
    ap: &crate::alt_path::AltPathSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledAltPath {
    CompiledAltPath {
        kind: compile_alt_path_kind(ap.kind),
        from: ap.from.as_ref().map(|p| {
            Box::new(compile_predicate(
                p,
                &format!("{prefix}.from"),
                card_id,
                errors,
            ))
        }),
        materials: ap
            .materials
            .iter()
            .enumerate()
            .map(|(i, m)| compile_material(m, &format!("{prefix}.materials[{i}]"), card_id, errors))
            .collect(),
        cost: ap.cost.as_ref().map(compile_cost),
        stacks_unsuspended: ap.stacks_unsuspended,
        ignore_requirements: ap.ignore_requirements,
        source_treated_as: ap.source_treated_as.clone(),
        extra_cost: ap
            .extra_cost
            .as_ref()
            .map(|v| {
                v.iter()
                    .enumerate()
                    .map(|(i, s)| {
                        compile_step(s, &format!("{prefix}.extra_cost[{i}]"), card_id, errors)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        on_burst_turn_end: ap
            .on_burst_turn_end
            .as_ref()
            .map(|v| {
                v.iter()
                    .enumerate()
                    .map(|(i, s)| {
                        compile_step(
                            s,
                            &format!("{prefix}.on_burst_turn_end[{i}]"),
                            card_id,
                            errors,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        marker: ap.marker,
    }
}

fn compile_alt_path_kind(k: crate::alt_path::AltPathKind) -> CompiledAltPathKind {
    use crate::alt_path::AltPathKind as S;
    match k {
        S::Digivolve => CompiledAltPathKind::Digivolve,
        S::DnaDigivolve => CompiledAltPathKind::DnaDigivolve,
        S::DigiXros => CompiledAltPathKind::DigiXros,
        S::BurstDigivolve => CompiledAltPathKind::BurstDigivolve,
        S::AppFusion => CompiledAltPathKind::AppFusion,
        S::Assembly => CompiledAltPathKind::Assembly,
        S::ActivatedDigivolve => CompiledAltPathKind::ActivatedDigivolve,
    }
}

fn compile_material(
    m: &crate::alt_path::MaterialSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledMaterial {
    // MaterialSpec has dual filter/inline_filter fields. Prefer the explicit
    // `filter` wrapper if present; otherwise fall through to the inline form.
    let filter_spec = m.filter.as_ref().unwrap_or(&m.inline_filter);
    CompiledMaterial {
        filter: compile_predicate(filter_spec, &format!("{prefix}.filter"), card_id, errors),
        repeat: m.repeat.as_ref().map(compile_repeat),
        distinct_by: m.distinct_by.map(compile_distinct_by),
        zones: m.zones.iter().map(|z| compile_zone(*z)).collect(),
        stack_under: m.stack_under,
    }
}

fn compile_repeat(r: &crate::alt_path::RepeatSpec) -> CompiledRepeat {
    use crate::alt_path::{RepeatKeyword, RepeatSpec};
    match r {
        RepeatSpec::Keyword(RepeatKeyword::Unbounded) => CompiledRepeat::Unbounded,
        RepeatSpec::Range { min, max } => CompiledRepeat::Range {
            min: *min,
            max: *max,
        },
    }
}

fn compile_cost(c: &crate::alt_path::CostSpec) -> CompiledCost {
    use crate::alt_path::{CostSpec, FormulaCost};
    match c {
        CostSpec::Literal(n) => CompiledCost::Literal(*n),
        CostSpec::Formula(FormulaCost { formula }) => {
            CompiledCost::Formula(compile_formula(formula))
        }
    }
}

// ── Clauses ─────────────────────────────────────────────────────────

fn compile_clause(
    c: &crate::clause::ClauseSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledClause {
    use crate::clause::ClauseSpec as C;
    match c {
        C::Triggered(t) => CompiledClause::Triggered(compile_triggered(t, prefix, card_id, errors)),
        C::Declarative(d) => match d.typed_body() {
            Ok(body) => {
                CompiledClause::Declarative(compile_declarative(d, body, prefix, card_id, errors))
            }
            Err(e) => {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: prefix.into(),
                    message: format!("declarative body schema: {e}"),
                });
                // Placeholder — errors is non-empty so compile() will Err.
                CompiledClause::Declarative(CompiledDeclarativeClause::AceOverflow {
                    value: 0,
                    summary: None,
                    summary_key: None,
                })
            }
        },
    }
}

fn compile_triggered(
    t: &crate::clause::TriggeredClause,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledTriggeredClause {
    use crate::clause::TimingSet;
    let when = match &t.when {
        TimingSet::Single(x) => vec![compile_timing(*x)],
        TimingSet::Multi(v) => v.iter().map(|x| compile_timing(*x)).collect(),
    };
    CompiledTriggeredClause {
        when,
        scope: compile_scope(t.scope),
        active_when: t
            .active_when
            .as_ref()
            .map(|p| compile_predicate(p, &format!("{prefix}.active_when"), card_id, errors)),
        condition: t
            .condition
            .as_ref()
            .map(|p| compile_predicate(p, &format!("{prefix}.condition"), card_id, errors)),
        optional: t.optional,
        once_per_turn: t.once_per_turn,
        max_per_turn: t.max_per_turn,
        process: t
            .process
            .iter()
            .enumerate()
            .map(|(i, s)| compile_step(s, &format!("{prefix}.process[{i}]"), card_id, errors))
            .collect(),
        summary: t.summary.clone(),
        summary_key: t.summary_key.clone(),
    }
}

fn compile_declarative(
    d: &crate::clause::DeclarativeClause,
    body: crate::clause::TypedDeclarativeBody,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledDeclarativeClause {
    use crate::clause::TypedDeclarativeBody as B;
    let scope = compile_scope(d.scope);
    let active_when = d
        .active_when
        .as_ref()
        .map(|p| compile_predicate(p, &format!("{prefix}.active_when"), card_id, errors));
    let summary = d.summary.clone();
    let summary_key = d.summary_key.clone();

    match body {
        B::Aura(a) => CompiledDeclarativeClause::Aura {
            scope,
            active_when,
            target: compile_predicate(&a.target, &format!("{prefix}.target"), card_id, errors),
            dp_modifier: a.dp_modifier,
            grant_keyword: a.grant_keyword.map(|gk| CompiledGrantKeywordValue {
                keyword: gk.keyword,
                value: gk.value,
            }),
            modifier: a.modifier,
            summary,
            summary_key,
        },
        B::CostReduction(c) => CompiledDeclarativeClause::CostReduction {
            scope,
            active_when,
            reduction_timing: c.reduction_timing,
            when_playing_this: c.when_playing_this,
            when_any_ally_played: c.when_any_ally_played.as_ref().map(|p| {
                compile_predicate(
                    p,
                    &format!("{prefix}.when_any_ally_played"),
                    card_id,
                    errors,
                )
            }),
            condition: c
                .condition
                .as_ref()
                .map(|p| compile_predicate(p, &format!("{prefix}.condition"), card_id, errors)),
            once_per_turn: c.once_per_turn,
            amount: c.amount,
            amount_fn: c.amount_fn.as_ref().map(compile_formula),
            pay_cost: c
                .pay_cost
                .as_ref()
                .map(|v| {
                    v.iter()
                        .enumerate()
                        .map(|(i, s)| {
                            compile_step(s, &format!("{prefix}.pay_cost[{i}]"), card_id, errors)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            summary,
            summary_key,
        },
        B::Replacement(r) => CompiledDeclarativeClause::Replacement {
            scope,
            active_when,
            trigger: r.trigger,
            process: r
                .process
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.process[{i}]"), card_id, errors))
                .collect(),
            summary,
            summary_key,
        },
        B::Partition(p) => CompiledDeclarativeClause::Partition {
            scope,
            active_when,
            sources: p
                .sources
                .iter()
                .enumerate()
                .map(|(i, pr)| {
                    compile_predicate(pr, &format!("{prefix}.sources[{i}]"), card_id, errors)
                })
                .collect(),
            exclude_cause: p.exclude_cause,
            summary,
            summary_key,
        },
        B::AceOverflow(a) => CompiledDeclarativeClause::AceOverflow {
            value: a.value,
            summary,
            summary_key,
        },
        B::GrantKeyword(gk) => CompiledDeclarativeClause::GrantKeyword {
            keyword: gk.keyword,
            value: gk.value,
            scope,
            active_when,
            summary,
            summary_key,
        },
        B::Delay(dl) => CompiledDeclarativeClause::Delay {
            scope,
            active_when,
            trigger: compile_timing(dl.trigger),
            process: dl
                .process
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.process[{i}]"), card_id, errors))
                .collect(),
            summary,
            summary_key,
        },
        B::FloodGate(fg) => CompiledDeclarativeClause::FloodGate {
            scope,
            active_when,
            modifier: fg.modifier,
            target: compile_predicate(&fg.target, &format!("{prefix}.target"), card_id, errors),
            summary,
            summary_key,
        },
        B::AltPathRegistration(ap) => {
            // Reassemble the `registers` IndexMap into a YAML mapping and
            // deserialize as AltPathSpec, then compile it.
            let registers_value = serde_yml::Value::Mapping(
                ap.registers
                    .iter()
                    .map(|(k, v)| (serde_yml::Value::String(k.clone()), v.clone()))
                    .collect(),
            );
            let registers_spec: crate::alt_path::AltPathSpec =
                match serde_yml::from_value(registers_value) {
                    Ok(v) => v,
                    Err(e) => {
                        errors.push(ValidationError {
                            card_id: card_id.into(),
                            path: format!("{prefix}.registers"),
                            message: format!("alt-path registration parse failed: {e}"),
                        });
                        return CompiledDeclarativeClause::AceOverflow {
                            value: 0,
                            summary,
                            summary_key,
                        };
                    }
                };
            let compiled_registers = compile_alt_path(
                &registers_spec,
                &format!("{prefix}.registers"),
                card_id,
                errors,
            );
            CompiledDeclarativeClause::AltPathRegistration {
                scope,
                active_when,
                trigger: compile_timing(ap.trigger),
                applies_to: ap.applies_to.as_ref().map(|p| {
                    compile_predicate(p, &format!("{prefix}.applies_to"), card_id, errors)
                }),
                registers: compiled_registers,
                summary,
                summary_key,
            }
        }
        B::RawRust(r) => CompiledDeclarativeClause::RawRust {
            fn_name: r.fn_name,
            triggers: r.triggers.iter().map(|t| compile_timing(*t)).collect(),
            scope,
            summary,
            summary_key,
        },
    }
}

// ── Steps ───────────────────────────────────────────────────────────

fn compile_binding_ref(b: &crate::step::BindingRef) -> CompiledBindingRef {
    use crate::step::{BindingRef as B, StructuredBindingRef};
    match b {
        B::Named(n) => match n.as_str() {
            "self" => CompiledBindingRef::SelfRef,
            "carrier" => CompiledBindingRef::Carrier,
            "source" => CompiledBindingRef::Source,
            "event_target" => CompiledBindingRef::EventTarget,
            "event_card" => CompiledBindingRef::EventCard,
            _ => CompiledBindingRef::Named(n.clone()),
        },
        B::Structured(StructuredBindingRef {
            permanent,
            binding,
            of_permanent,
            ..
        }) => {
            if let Some(p) = permanent {
                CompiledBindingRef::Permanent(p.clone())
            } else if let Some(b) = binding {
                CompiledBindingRef::Binding(b.clone())
            } else if let Some(o) = of_permanent {
                CompiledBindingRef::OfPermanent(o.clone())
            } else {
                CompiledBindingRef::Named(String::new())
            }
        }
    }
}

fn compile_modifier_value(v: &crate::step::ModifierValueSpec) -> CompiledModifierValue {
    use crate::step::ModifierValueSpec as S;
    match v {
        S::Literal(n) => CompiledModifierValue::Literal(*n),
        S::Formula(fc) => CompiledModifierValue::Formula(compile_formula(&fc.formula)),
    }
}

fn compile_modifier_target(
    t: &crate::step::ModifierTarget,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledModifierTarget {
    use crate::step::ModifierTarget as T;
    match t {
        T::Binding(b) => CompiledModifierTarget::Binding(compile_binding_ref(b)),
        T::Filter(p) => {
            CompiledModifierTarget::Filter(compile_predicate(p, prefix, card_id, errors))
        }
    }
}

/// Compile an `IfStep.condition` — stored as `serde_yml::Value` in the spec —
/// to a `CompiledPredicate`. If the value cannot be deserialized as a
/// `PredicateSpec`, an empty default predicate is returned and an error is
/// recorded.
fn compile_if_condition(
    v: &serde_yml::Value,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledPredicate {
    match serde_yml::from_value::<crate::predicate::PredicateSpec>(v.clone()) {
        Ok(p) => compile_predicate(&p, prefix, card_id, errors),
        Err(e) => {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: prefix.into(),
                message: format!("if condition parse failed: {e}"),
            });
            CompiledPredicate::default()
        }
    }
}

fn compile_step(
    s: &crate::step::StepSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledStep {
    use crate::step::StepSpec as S;
    match s {
        S::GainMemory(n) => CompiledStep::GainMemory(*n),
        S::LoseMemory(n) => CompiledStep::LoseMemory(*n),
        S::SetMemory(n) => CompiledStep::SetMemory(*n),

        S::Draw(a) => CompiledStep::Draw {
            of: compile_player_ref(a.of),
            count: a.count,
        },
        S::TrashFromTop(a) => CompiledStep::TrashFromTop {
            of: compile_player_ref(a.of),
            count: a.count,
        },
        S::AddToHandFromDeck(a) => CompiledStep::AddToHandFromDeck {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::AddToHandFromTrash(a) => CompiledStep::AddToHandFromTrash {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::AddToHandFromReveal(a) => CompiledStep::AddToHandFromReveal {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::TrashFromHandByIndex(a) => CompiledStep::TrashFromHandByIndex {
            of: compile_player_ref(a.of),
            hand_index: compile_binding_ref(&a.hand_index),
        },
        S::TrashFromReveal(a) => CompiledStep::TrashFromReveal {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::ReturnToDeckFromReveal(a) => CompiledStep::ReturnToDeckFromReveal {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
            position: compile_stack_position(a.position),
        },
        S::ShuffleDeck(a) => CompiledStep::ShuffleDeck {
            of: compile_player_ref(a.of),
        },
        S::RevealTopDeck(a) => CompiledStep::RevealTopDeck {
            of: compile_player_ref(a.of),
            count: a.count,
            zone: a.zone.map(compile_zone),
            bind_as: a.bind_as.clone(),
        },
        S::PlaceRemainderOnDeck(a) => CompiledStep::PlaceRemainderOnDeck {
            of: compile_player_ref(a.of),
            position: compile_stack_position(a.position),
        },

        S::DeletePermanent(a) => CompiledStep::DeletePermanent {
            target: compile_binding_ref(&a.target),
        },
        S::ReturnToHand(a) => CompiledStep::ReturnToHand {
            target: compile_binding_ref(&a.target),
        },
        S::ReturnToDeck(a) => CompiledStep::ReturnToDeck {
            target: compile_binding_ref(&a.target),
            position: compile_stack_position(a.position),
            include_sources: a.include_sources,
        },
        S::Suspend(a) => CompiledStep::Suspend {
            target: compile_binding_ref(&a.target),
        },
        S::Unsuspend(a) => CompiledStep::Unsuspend {
            target: compile_binding_ref(&a.target),
        },
        S::DeDigivolve(a) => CompiledStep::DeDigivolve {
            target: compile_binding_ref(&a.target),
            amount: a.amount,
            stop_at_level: a.stop_at_level,
        },
        S::PlaceOnSecurity(a) => CompiledStep::PlaceOnSecurity {
            of: compile_player_ref(a.of),
            source: compile_binding_ref(&a.source),
            position: compile_stack_position(a.position),
            face_up: a.face_up,
        },
        S::PlayToken(a) => CompiledStep::PlayToken {
            controller: compile_player_ref(a.controller),
            token_name: a.token_name.clone(),
        },
        S::PlaceAsBottomSource(a) => CompiledStep::PlaceAsBottomSource {
            source: compile_binding_ref(&a.source),
            target: compile_binding_ref(&a.target),
        },
        S::TrashTopSource(a) => CompiledStep::TrashTopSource {
            target: compile_binding_ref(&a.target),
        },
        S::CancelLeave(_) => CompiledStep::CancelLeave,
        S::HandleReplacement(_) => CompiledStep::HandleReplacement,
        S::RedirectReplacement(a) => CompiledStep::RedirectReplacement {
            destination: compile_zone(a.destination),
        },
        S::SubstitutePermanent(a) => CompiledStep::SubstitutePermanent {
            target: compile_binding_ref(&a.target),
        },
        S::Hatch(a) => CompiledStep::Hatch {
            of: compile_player_ref(a.of),
        },

        // PlayFromHand, PlayFromTrash, PlayFromTrashFree all use PlayFromHandArgs
        // (with hand_index field), but the compiled variants use different field
        // names for trash steps.
        S::PlayFromHand(a) => CompiledStep::PlayFromHand {
            of: compile_player_ref(a.of),
            hand_index: compile_binding_ref(&a.hand_index),
            cost_delta: a.cost_delta.as_ref().map(compile_cost_delta),
        },
        S::PlayFromHandFree(a) => CompiledStep::PlayFromHandFree {
            of: compile_player_ref(a.of),
            hand_index: compile_binding_ref(&a.hand_index),
        },
        // PlayFromTrash reuses PlayFromHandArgs but the compiled form uses `trash_index`
        S::PlayFromTrash(a) => CompiledStep::PlayFromTrash {
            of: compile_player_ref(a.of),
            trash_index: compile_binding_ref(&a.hand_index),
            cost_delta: a.cost_delta.as_ref().map(compile_cost_delta),
        },
        S::PlayFromTrashFree(a) => CompiledStep::PlayFromTrashFree {
            of: compile_player_ref(a.of),
            trash_index: compile_binding_ref(&a.hand_index),
        },
        S::PlayFromSecurity(_) => CompiledStep::PlayFromSecurity,
        S::PlayFromMaterials(a) => CompiledStep::PlayFromMaterials {
            target: compile_binding_ref(&a.target),
            source_index: compile_binding_ref(&a.source_index),
            cost_delta: a.cost_delta.as_ref().map(compile_cost_delta),
        },
        S::EffectInitiatedDigivolve(a) => CompiledStep::EffectInitiatedDigivolve {
            target: compile_binding_ref(&a.target),
            from_hand: compile_binding_ref(&a.from_hand),
            cost: a.cost,
            ignore_requirements: a.ignore_requirements,
        },
        S::EffectInitiatedDnaDigivolve(a) => CompiledStep::EffectInitiatedDnaDigivolve {
            target_a: compile_binding_ref(&a.target_a),
            target_b: compile_binding_ref(&a.target_b),
            from_hand: compile_binding_ref(&a.from_hand),
            cost: a.cost,
            ignore_requirements: a.ignore_requirements,
        },

        S::TrashTopSecurity(a) => CompiledStep::TrashTopSecurity {
            of: compile_player_ref(a.of),
        },
        S::MarkSecurityFaceUp(a) => CompiledStep::MarkSecurityFaceUp {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },

        S::AddDpModifier(a) => CompiledStep::AddDpModifier {
            target: compile_binding_ref(&a.target),
            value: compile_modifier_value(&a.value),
            expiry: a.expiry.clone(),
        },
        S::AddModifier(a) => CompiledStep::AddModifier {
            target: compile_modifier_target(
                &a.target,
                &format!("{prefix}.target"),
                card_id,
                errors,
            ),
            modifier: a.modifier.clone(),
            value: compile_modifier_value(&a.value),
            expiry: a.expiry.clone(),
        },
        S::GrantKeyword(a) => CompiledStep::GrantKeyword {
            target: compile_binding_ref(&a.target),
            keyword: a.keyword.clone(),
            expiry: a.expiry.clone(),
            value: a.value,
        },

        S::SelectOwnPermanent(a) => CompiledStep::SelectOwnPermanent {
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectOpponentPermanent(a) => CompiledStep::SelectOpponentPermanent {
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectAnyPermanent(a) => CompiledStep::SelectAnyPermanent {
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectDnaPair(a) => CompiledStep::SelectDnaPair {
            left_filter: compile_predicate(
                &a.left_filter,
                &format!("{prefix}.left_filter"),
                card_id,
                errors,
            ),
            right_filter: compile_predicate(
                &a.right_filter,
                &format!("{prefix}.right_filter"),
                card_id,
                errors,
            ),
            bind_left_as: a.bind_left_as.clone(),
            bind_right_as: a.bind_right_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectHand(a) => CompiledStep::SelectHand {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectTrash(a) => CompiledStep::SelectTrash {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectMaterial(a) => CompiledStep::SelectMaterial {
            of_permanent: compile_binding_ref(&a.of_permanent),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectReveal(a) => CompiledStep::SelectReveal {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectSecurity(a) => CompiledStep::SelectSecurity {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectUnionZone(a) => CompiledStep::SelectUnionZone {
            of: compile_player_ref(a.of),
            zones: a.zones.iter().map(|z| compile_zone(*z)).collect(),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectOrderedPermutation(a) => CompiledStep::SelectOrderedPermutation {
            items: compile_binding_ref(&a.items),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
        },
        S::SelectCountCappedMulti(a) => CompiledStep::SelectCountCappedMulti {
            of: compile_player_ref(a.of),
            zone: compile_zone(a.zone),
            max: a.max,
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional_zero: a.optional_zero,
            distinct_by: a.distinct_by.map(compile_distinct_by),
        },
        S::SelectEffectChoice(a) => CompiledStep::SelectEffectChoice {
            labels: a.labels.clone(),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
        },
        S::AsSelectingPlayer(a) => CompiledStep::AsSelectingPlayer {
            of: compile_player_ref(a.of),
            body: a
                .body
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.body[{i}]"), card_id, errors))
                .collect(),
        },

        // IfStep.condition is serde_yml::Value, not PredicateSpec — needs special handling.
        S::If(i) => CompiledStep::If {
            condition: compile_if_condition(
                &i.condition,
                &format!("{prefix}.if.condition"),
                card_id,
                errors,
            ),
            then: i
                .then
                .iter()
                .enumerate()
                .map(|(k, s)| compile_step(s, &format!("{prefix}.then[{k}]"), card_id, errors))
                .collect(),
            else_branch: i
                .else_
                .as_ref()
                .map(|v| {
                    v.iter()
                        .enumerate()
                        .map(|(k, s)| {
                            compile_step(s, &format!("{prefix}.else[{k}]"), card_id, errors)
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        S::ForEach(f) => CompiledStep::ForEach {
            over: compile_predicate(&f.over, &format!("{prefix}.over"), card_id, errors),
            bind_as: f.bind_as.clone(),
            body: f
                .body
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.body[{i}]"), card_id, errors))
                .collect(),
        },
        S::PerSelected(ps) => CompiledStep::PerSelected {
            selection: ps.selection.clone(),
            bind_as: ps.bind_as.clone(),
            body: ps
                .body
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.body[{i}]"), card_id, errors))
                .collect(),
        },
        S::ScheduleDelayed(sd) => CompiledStep::ScheduleDelayed {
            when: compile_timing(sd.when),
            body: sd
                .body
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.body[{i}]"), card_id, errors))
                .collect(),
        },
        // OptionalStep is a newtype wrapping Vec<StepSpec> — access via .0
        S::Optional(o) => CompiledStep::Optional(
            o.0.iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.optional[{i}]"), card_id, errors))
                .collect(),
        ),
        S::RawRust(r) => CompiledStep::RawRust {
            fn_name: r.fn_name.clone(),
            consumes: r.consumes.clone(),
            binds: r.binds.clone(),
        },
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_dir_ok;
    use std::path::PathBuf;

    #[test]
    fn every_example_compiles() {
        // digimon-dsl is the CWD at test time; fixtures live under
        // ../digimon-engine/cards/_examples/.
        let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("digimon-engine")
            .join("cards")
            .join("_examples");
        let (specs, errs) = load_dir_ok(&examples);
        assert!(errs.is_empty(), "parse errors: {errs:#?}");
        assert_eq!(specs.len(), 15);

        let mut failures = Vec::new();
        for spec in &specs {
            if let Err(e) = compile(spec) {
                failures.push(format!("{}: {e:#?}", spec.card));
            }
        }
        assert!(
            failures.is_empty(),
            "compile failures:\n{}",
            failures.join("\n")
        );
    }
}
