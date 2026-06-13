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
    let dual = spec
        .dual
        .as_ref()
        .map(|dual| compile_dual(dual, &spec.card, &mut errors));
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
    let use_requirement = spec
        .use_requirement
        .as_ref()
        .map(|p| compile_predicate(p, "use_requirement", &spec.card, &mut errors));

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
        digixros_aliases: spec.digixros_aliases.clone(),
        also_treated_as: spec.also_treated_as.clone(),
        dual,
        use_requirement,
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
        S::Dual => CompiledCardKind::Dual,
    }
}

fn compile_field_selector(s: crate::step::FieldSelector) -> CompiledFieldSelector {
    match s {
        crate::step::FieldSelector::LowestDp => CompiledFieldSelector::LowestDp,
        crate::step::FieldSelector::HighestDp => CompiledFieldSelector::HighestDp,
        crate::step::FieldSelector::LowestPlayCost => CompiledFieldSelector::LowestPlayCost,
        crate::step::FieldSelector::HighestPlayCost => CompiledFieldSelector::HighestPlayCost,
        crate::step::FieldSelector::LowestMaterialCount => {
            CompiledFieldSelector::LowestMaterialCount
        }
    }
}

fn compile_dual(
    dual: &crate::spec::DualSpec,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledDual {
    CompiledDual {
        digimon: CompiledDualDigimon {
            level: dual.digimon.level,
            dp: dual.digimon.dp,
            colors: dual
                .digimon
                .colors
                .iter()
                .map(|c| compile_color(*c))
                .collect(),
            traits: dual.digimon.traits.clone(),
            effect_text: dual.digimon.effect_text.clone(),
            inherited_text: dual.digimon.inherited_text.clone(),
        },
        option: CompiledDualOption {
            use_cost: dual.option.use_cost,
            colors: dual
                .option
                .colors
                .iter()
                .map(|c| compile_color(*c))
                .collect(),
            effect_text: dual.option.effect_text.clone(),
            security_text: dual.option.security_text.clone(),
            keywords: dual.option.keywords.clone(),
            use_requirement: dual.option.use_requirement.as_ref().map(|p| {
                Box::new(compile_predicate(
                    p,
                    "dual.option.use_requirement",
                    card_id,
                    errors,
                ))
            }),
        },
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

fn compile_synth_identity(
    s: &crate::step::SynthIdentitySpec,
) -> crate::compiled::CompiledSynthIdentity {
    crate::compiled::CompiledSynthIdentity {
        kind: compile_card_kind(s.kind),
        level: s.level,
        colors: s.colors.iter().copied().map(compile_color).collect(),
        traits: s.traits.clone(),
        dp: s.dp,
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

fn validate_digixros_wildcard_zone(
    zone: Option<crate::predicate::Zone>,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<CompiledZone> {
    match zone {
        Some(crate::predicate::Zone::Hand) => Some(CompiledZone::Hand),
        Some(crate::predicate::Zone::Trash) => Some(CompiledZone::Trash),
        Some(crate::predicate::Zone::Material) => Some(CompiledZone::Material),
        Some(other) => {
            errors.push(ValidationError {
                card_id: card_id.to_string(),
                path: format!("{prefix}.zone"),
                message: format!(
                    "digixros wildcard zone must be one of hand, trash, or material (got {other:?})"
                ),
            });
            Some(compile_zone(other))
        }
        None => None,
    }
}

fn compile_scope(s: crate::clause::ClauseScope) -> CompiledScope {
    use crate::clause::ClauseScope as S;
    match s {
        S::FaceUp => CompiledScope::FaceUp,
        S::Inherited => CompiledScope::Inherited,
        S::Both => CompiledScope::Both,
        S::Linked => CompiledScope::Linked,
        S::Security => CompiledScope::Security,
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
        S::OnBlock => CompiledTiming::OnBlock,
        S::OnAllyAttack => CompiledTiming::OnAllyAttack,
        S::OnOpponentAttack => CompiledTiming::OnOpponentAttack,
        S::OnDeletion => CompiledTiming::OnDeletion,
        S::OnAnyDeletion => CompiledTiming::OnAnyDeletion,
        S::OnEnterFieldAnyone => CompiledTiming::OnEnterFieldAnyone,
        S::OnAnyDigimonPlayed => CompiledTiming::OnAnyDigimonPlayed,
        S::OnAllyPlayed => CompiledTiming::OnAllyPlayed,
        S::OnLeaveField => CompiledTiming::OnLeaveField,
        S::OnSuspend => CompiledTiming::OnSuspend,
        S::OnUnsuspend => CompiledTiming::OnUnsuspend,
        S::OnAddToHand => CompiledTiming::OnAddToHand,
        S::OnHatch => CompiledTiming::OnHatch,
        S::OnMove => CompiledTiming::OnMove,
        S::OnDigivolve => CompiledTiming::OnDigivolve,
        S::OnDnaDigivolve => CompiledTiming::OnDnaDigivolve,
        S::OnDigixros => CompiledTiming::OnDigixros,
        S::OnOpponentSecurityRemoved => CompiledTiming::OnOpponentSecurityRemoved,
        S::OnOwnSecurityRemoved => CompiledTiming::OnOwnSecurityRemoved,
        S::OnDigivolutionCardTrashed => CompiledTiming::OnDigivolutionCardTrashed,
        S::OnSecurityCheck => CompiledTiming::OnSecurityCheck,
        S::OnCheckFaceUpSecurity => CompiledTiming::OnCheckFaceUpSecurity,
        S::OnLoseSecurity => CompiledTiming::OnLoseSecurity,
        S::OnDiscardSecurity => CompiledTiming::OnDiscardSecurity,
        S::OnSecurity => CompiledTiming::OnSecurity,
        S::OnOptionPlaced => CompiledTiming::OnOptionPlaced,
        S::OnOptionTrashed => CompiledTiming::OnOptionTrashed,
        S::OnPlaceSecurity => CompiledTiming::OnPlaceSecurity,
        S::OnAddedToSecurity => CompiledTiming::OnAddedToSecurity,
        S::Main => CompiledTiming::Main,
        S::StartOfYourTurn => CompiledTiming::StartOfYourTurn,
        S::StartOfOpponentsTurn => CompiledTiming::StartOfOpponentsTurn,
        S::StartOfYourMainPhase => CompiledTiming::StartOfYourMainPhase,
        S::EndOfYourTurn => CompiledTiming::EndOfYourTurn,
        S::EndOfOpponentsTurn => CompiledTiming::EndOfOpponentsTurn,
        S::EndOfYourNextTurn => CompiledTiming::EndOfYourNextTurn,
        S::EndOfOpponentsNextTurn => CompiledTiming::EndOfOpponentsNextTurn,
        S::UntilNextUnsuspend => CompiledTiming::UntilNextUnsuspend,
        S::OnAttackTargetChange => CompiledTiming::OnAttackTargetChange,
        S::MainFromHand => CompiledTiming::MainFromHand,
        S::MainOnField => CompiledTiming::MainOnField,
        S::MainFromTrash => CompiledTiming::MainFromTrash,
        S::Counter => CompiledTiming::Counter,
        S::BeforePayCost => CompiledTiming::BeforePayCost,
        S::BeforePayCostObserve => CompiledTiming::BeforePayCostObserve,
        S::Delayed => CompiledTiming::Delayed,
        S::WhenLinked => CompiledTiming::WhenLinked,
        S::WhenCardLinkedToThis => CompiledTiming::WhenCardLinkedToThis,
        S::WhenWouldLinkToThis => CompiledTiming::WhenWouldLinkToThis,
        S::OnAnyLink => CompiledTiming::OnAnyLink,
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

fn compile_remainder_destination(
    d: crate::step::RemainderDestination,
) -> CompiledRemainderDestination {
    use crate::step::RemainderDestination as S;
    match d {
        S::DeckTop => CompiledRemainderDestination::DeckTop,
        S::DeckBottom => CompiledRemainderDestination::DeckBottom,
    }
}

fn compile_reveal_destination(d: &crate::step::RevealDestination) -> CompiledRevealDestination {
    use crate::step::RevealDestination as S;
    match d {
        S::Hand => CompiledRevealDestination::Hand,
        S::DeckTop => CompiledRevealDestination::DeckTop,
        S::DeckBottom => CompiledRevealDestination::DeckBottom,
        S::PlayFree => CompiledRevealDestination::PlayFree,
        S::BottomSourceOf { target } => {
            CompiledRevealDestination::BottomSourceOf(compile_binding_ref(target))
        }
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

fn compile_attack_target_spec(t: crate::step::AttackTargetSpec) -> CompiledAttackTargetSpec {
    match t {
        crate::step::AttackTargetSpec::Any => CompiledAttackTargetSpec::Any,
        crate::step::AttackTargetSpec::Player => CompiledAttackTargetSpec::Player,
        crate::step::AttackTargetSpec::Digimon => CompiledAttackTargetSpec::Digimon,
    }
}

fn compile_per_selector(
    p: &crate::formula::PerSelector,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledPerSelector {
    use crate::formula::PerSelector as S;
    match p {
        S::MaterialCount => CompiledPerSelector::MaterialCount,
        S::StackSize => CompiledPerSelector::StackSize,
        S::AllyCount => CompiledPerSelector::AllyCount,
        S::SuspendedCount { of, exclude_source } => CompiledPerSelector::SuspendedCount {
            of: compile_player_ref(*of),
            exclude_source: *exclude_source,
        },
        S::DigivolutionColorCount => CompiledPerSelector::DigivolutionColorCount,
        S::SourceColorCount => CompiledPerSelector::SourceColorCount,
        S::SameLevelPairsInSources => CompiledPerSelector::SameLevelPairsInSources,
        S::SharedTrashCount { bucket } => CompiledPerSelector::SharedTrashCount { bucket: *bucket },
        S::CardCountInZone(spec) => {
            if let Some(filter) = &spec.filter {
                CompiledPerSelector::FilteredCardCountInZoneScoped {
                    zone: compile_zone(spec.zone),
                    of: compile_player_ref(spec.of),
                    filter: Box::new(compile_predicate(
                        filter,
                        &format!("{prefix}.filter"),
                        card_id,
                        errors,
                    )),
                }
            } else {
                CompiledPerSelector::CardCountInZoneScoped {
                    zone: compile_zone(spec.zone),
                    of: compile_player_ref(spec.of),
                }
            }
        }
        S::DistinctColorsCount(spec) => CompiledPerSelector::DistinctColorsCountScoped {
            zone: compile_zone(spec.zone),
            of: compile_player_ref(spec.of),
            filter: spec.filter.as_ref().map(|filter| {
                Box::new(compile_predicate(
                    filter,
                    &format!("{prefix}.filter"),
                    card_id,
                    errors,
                ))
            }),
        },
        S::SourceStackCount(spec) => CompiledPerSelector::SourceStackCountFiltered {
            filter: spec.filter.as_ref().map(|filter| {
                Box::new(compile_predicate(
                    filter,
                    &format!("{prefix}.filter"),
                    card_id,
                    errors,
                ))
            }),
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
        S::FewestMaterials => CompiledAggregateSelector::FewestMaterials,
        S::LowestPlayCost => CompiledAggregateSelector::LowestPlayCost,
    }
}

fn compile_cost_delta(
    c: &crate::step::CostDelta,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledCostDelta {
    use crate::step::{CostDelta, CostDeltaKeyword};
    match c {
        CostDelta::Keyword(CostDeltaKeyword::Free) => CompiledCostDelta::Free,
        CostDelta::Keyword(CostDeltaKeyword::Printed) => CompiledCostDelta::Printed,
        CostDelta::Literal(n) => CompiledCostDelta::Literal(*n),
        CostDelta::Reduce { reduce } => CompiledCostDelta::Reduce(*reduce),
        CostDelta::ReduceFn { reduce_fn } => CompiledCostDelta::ReduceFn(compile_formula(
            reduce_fn,
            &format!("{prefix}.cost_delta.reduce_fn"),
            card_id,
            errors,
        )),
    }
}

// ── Identity ────────────────────────────────────────────────────────

fn compile_identity(id: &crate::identity::IdentitySpec) -> CompiledIdentity {
    CompiledIdentity {
        name_aliases: id.name_aliases.iter().map(compile_name_alias).collect(),
        source_name_aliases: id
            .source_name_aliases
            .iter()
            .map(compile_source_name_alias)
            .collect(),
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

fn compile_source_name_alias(a: &crate::identity::SourceNameAliasSpec) -> CompiledSourceNameAlias {
    CompiledSourceNameAlias {
        level_lte: a.level_lte,
    }
}

// ── Formula ─────────────────────────────────────────────────────────

fn compile_formula(
    f: &crate::formula::FormulaSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledFormula {
    use crate::formula::{CompoundFormula, FormulaSpec};
    match f {
        FormulaSpec::Literal(n) => CompiledFormula::Literal(*n),
        FormulaSpec::BasePerDelta {
            base,
            per,
            bucket,
            delta,
        } => CompiledFormula::BasePerDelta {
            base: *base,
            per: compile_per_selector(
                &per_with_bucket(per, *bucket),
                &format!("{prefix}.per"),
                card_id,
                errors,
            ),
            delta: *delta,
        },
        FormulaSpec::BindingDp { binding_dp } => CompiledFormula::BindingDp(binding_dp.clone()),
        FormulaSpec::BindingPlayCost { binding_play_cost } => {
            CompiledFormula::BindingPlayCost(binding_play_cost.clone())
        }
        FormulaSpec::BindingValue { binding_value } => {
            CompiledFormula::BindingValue(binding_value.clone())
        }
        FormulaSpec::SourceDp { source_dp: _ } => CompiledFormula::SourceDp,
        FormulaSpec::SourceMaterialCount {
            source_material_count: _,
        } => CompiledFormula::SourceMaterialCount,
        FormulaSpec::EventTargetLevel {
            event_target_level: _,
        } => CompiledFormula::EventTargetLevel,
        FormulaSpec::SourceColorCount {
            source_color_count: _,
        } => CompiledFormula::SourceColorCount,
        FormulaSpec::SourceStackCount { source_stack_count } => CompiledFormula::SourceStackCount {
            target: source_stack_count.target.clone(),
            filter: source_stack_count.filter.as_ref().map(|filter| {
                Box::new(compile_predicate(
                    filter,
                    &format!("{prefix}.source_stack_count.filter"),
                    card_id,
                    errors,
                ))
            }),
        },
        FormulaSpec::SourceStackDpSum {
            source_stack_dp_sum,
        } => CompiledFormula::SourceStackDpSum {
            target: source_stack_dp_sum.target.clone(),
            filter: source_stack_dp_sum.filter.as_ref().map(|filter| {
                Box::new(compile_predicate(
                    filter,
                    &format!("{prefix}.source_stack_dp_sum.filter"),
                    card_id,
                    errors,
                ))
            }),
        },
        FormulaSpec::Compound(CompoundFormula::FloorDiv(v)) => CompiledFormula::FloorDiv(
            v.iter()
                .enumerate()
                .map(|(i, f)| {
                    compile_formula(f, &format!("{prefix}.floor_div[{i}]"), card_id, errors)
                })
                .collect(),
        ),
        FormulaSpec::Compound(CompoundFormula::Max(v)) => CompiledFormula::Max(
            v.iter()
                .enumerate()
                .map(|(i, f)| compile_formula(f, &format!("{prefix}.max[{i}]"), card_id, errors))
                .collect(),
        ),
        FormulaSpec::Compound(CompoundFormula::Min(v)) => CompiledFormula::Min(
            v.iter()
                .enumerate()
                .map(|(i, f)| compile_formula(f, &format!("{prefix}.min[{i}]"), card_id, errors))
                .collect(),
        ),
        FormulaSpec::Compound(CompoundFormula::Aggregate(a)) => CompiledFormula::AggregateScoped {
            selector: compile_aggregate_selector(*a),
            scope: CompiledPlayerRef::You,
        },
        FormulaSpec::Compound(CompoundFormula::AggregateScoped(spec)) => {
            CompiledFormula::AggregateScoped {
                selector: compile_aggregate_selector(spec.selector),
                scope: compile_player_ref(spec.scope),
            }
        }
        FormulaSpec::Compound(CompoundFormula::RawRust(s)) => CompiledFormula::RawRust(s.clone()),
    }
}

fn compile_count_bound(
    bound: &crate::step::CountBound,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledCountBound {
    match bound {
        crate::step::CountBound::Literal(n) => CompiledCountBound::Literal(*n),
        crate::step::CountBound::Formula { formula } => {
            CompiledCountBound::Formula(compile_formula(formula, prefix, card_id, errors))
        }
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
        level_eq_binding: p.level_eq_binding.clone(),
        level_lte: p
            .level_lte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.level_lte"), card_id, errors)),
        level_gte: p
            .level_gte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.level_gte"), card_id, errors)),
        level_matches_aggregate: p.level_matches_aggregate.map(|m| {
            (
                compile_aggregate_selector(m.selector),
                compile_player_ref(m.of),
            )
        }),
        materials_count_matches_aggregate: p.materials_count_matches_aggregate.map(|m| {
            (
                compile_aggregate_selector(m.selector),
                compile_player_ref(m.of),
            )
        }),
        color_is: p.color_is.map(compile_color),
        color_only: p
            .color_only
            .as_ref()
            .map(|v| v.iter().map(|c| compile_color(*c)).collect()),
        color_matches_any_field_digimon: p
            .color_matches_any_field_digimon
            .map(|s| compile_player_ref(s.player())),
        color_matches_binding: p.color_matches_binding.clone(),
        trait_has: p.trait_has.clone(),
        trait_contains: p.trait_contains.clone(),
        form_is: p.form_is.clone(),
        attribute_is: p.attribute_is.clone(),
        name_is: p.name_is.clone(),
        name_contains: p.name_contains.clone(),
        effect_text_contains: p.effect_text_contains.clone(),
        name_in: p.name_in.clone(),
        name_not_shared_by_field_digimon: p
            .name_not_shared_by_field_digimon
            .map(|s| compile_player_ref(s.player())),
        name_not_shared_by_field_tamer: p
            .name_not_shared_by_field_tamer
            .map(|s| compile_player_ref(s.player())),
        card_number_is: p.card_number_is.clone(),
        play_cost_lte: p
            .play_cost_lte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.play_cost_lte"), card_id, errors)),
        play_cost_gte: p
            .play_cost_gte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.play_cost_gte"), card_id, errors)),
        can_digivolve_from_source: p.can_digivolve_from_source,
        dp_eq: p
            .dp_eq
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.dp_eq"), card_id, errors)),
        dp_lte: p
            .dp_lte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.dp_lte"), card_id, errors)),
        dp_gte: p
            .dp_gte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.dp_gte"), card_id, errors)),
        stack_size_lte: p.stack_size_lte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.stack_size_lte"), card_id, errors)
        }),
        stack_size_gte: p.stack_size_gte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.stack_size_gte"), card_id, errors)
        }),
        materials_count_lte: p.materials_count_lte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.materials_count_lte"), card_id, errors)
        }),
        materials_count_gte: p.materials_count_gte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.materials_count_gte"), card_id, errors)
        }),
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
        has_security_attack_change: p.has_security_attack_change,
        has_on_deletion_effect: p.has_on_deletion_effect,
        self_color_count_gte: p.self_color_count_gte,
        has_face_down_source: p.has_face_down_source,
        distinct_tamer_colors_gte: p.distinct_tamer_colors_gte,
        battle_opponent_no_sources: p.battle_opponent_no_sources,
        zone: p.zone.iter().map(|z| compile_zone(*z)).collect(),
        owner: p.owner.map(compile_player_ref),
        other: p.other,
        is_source: p.is_source,
        of_permanent: p.of_permanent.clone(),
        not_in_binding: p.not_in_binding.clone(),
        binding_owner: p
            .binding_owner
            .as_ref()
            .map(|b| CompiledBindingOwnerPredicate {
                binding: b.binding.clone(),
                of: compile_player_ref(b.of),
            }),
        binding_card_kind: p.binding_card_kind.as_ref().map(|b| {
            crate::compiled::CompiledBindingCardKindPredicate {
                binding: b.binding.clone(),
                kind: compile_card_kind(b.kind),
            }
        }),
        source_is_tamer: p.source_is_tamer,
        source_is_unsuspended: p.source_is_unsuspended,
        source_name_contains: p.source_name_contains.clone(),
        source_permanent_trait_has: p.source_permanent_trait_has.clone(),
        is_face_down: p.is_face_down,
        is_bottom_source: p.is_bottom_source,
        host_kind_is: p.host_kind_is.map(compile_card_kind),
        rules_text_contains: p.rules_text_contains.clone(),
        self_digivolution_contains_name: p.self_digivolution_contains_name.clone(),
        self_digivolution_sources_contain_name: p.self_digivolution_sources_contain_name.clone(),
        self_digivolution_sources_trait_has: p.self_digivolution_sources_trait_has.clone(),
        memory_lte: p
            .memory_lte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.memory_lte"), card_id, errors)),
        memory_gte: p
            .memory_gte
            .as_ref()
            .map(|d| compile_dp_constraint(d, &format!("{prefix}.memory_gte"), card_id, errors)),
        own_memory_lte: p.own_memory_lte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.own_memory_lte"), card_id, errors)
        }),
        own_memory_gte: p.own_memory_gte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.own_memory_gte"), card_id, errors)
        }),
        security_count_lte: p.security_count_lte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.security_count_lte"), card_id, errors)
        }),
        security_count_gte: p.security_count_gte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.security_count_gte"), card_id, errors)
        }),
        opponent_security_count_lte: p.opponent_security_count_lte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.opponent_security_count_lte"),
                card_id,
                errors,
            )
        }),
        opponent_security_count_gte: p.opponent_security_count_gte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.opponent_security_count_gte"),
                card_id,
                errors,
            )
        }),
        face_up_security_count_lte: p.face_up_security_count_lte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.face_up_security_count_lte"),
                card_id,
                errors,
            )
        }),
        face_up_security_count_gte: p.face_up_security_count_gte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.face_up_security_count_gte"),
                card_id,
                errors,
            )
        }),
        no_face_up_security_named: p.no_face_up_security_named.as_ref().map(|f| {
            let set_count = [
                f.card_number_is.is_some(),
                f.name_is.is_some(),
                f.color_is.is_some(),
            ]
            .iter()
            .filter(|b| **b)
            .count();
            if set_count != 1 {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.no_face_up_security_named"),
                    message: "exactly one of `card_number_is` / `name_is` / `color_is` must be set"
                        .to_string(),
                });
            }
            CompiledFaceUpSecurityNamed {
                of: compile_player_ref(f.of),
                card_number_is: f.card_number_is.clone(),
                name_is: f.name_is.clone(),
                color_is: f.color_is.map(compile_color),
            }
        }),
        binding_count_eq: p
            .binding_count_eq
            .as_ref()
            .map(|b| (b.binding.clone(), b.n)),
        your_turn: p.your_turn,
        opponents_turn: p.opponents_turn,
        all_turns: p.all_turns,
        can_hatch: p.can_hatch.map(compile_player_ref),
        digimon_attacked_this_turn: p.digimon_attacked_this_turn.map(compile_player_ref),
        in_breeding: p.in_breeding,
        on_field: p.on_field,
        dna_origin: p.dna_origin,
        event_target_kind: p.event_target_kind.map(compile_card_kind),
        event_target_trait_has: p.event_target_trait_has.clone(),
        event_target_level_eq: p.event_target_level_eq,
        event_target_level_lte: p.event_target_level_lte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.event_target_level_lte"),
                card_id,
                errors,
            )
        }),
        event_target_level_gte: p.event_target_level_gte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.event_target_level_gte"),
                card_id,
                errors,
            )
        }),
        event_target_dp_eq: p.event_target_dp_eq.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.event_target_dp_eq"), card_id, errors)
        }),
        event_target_dp_lte: p.event_target_dp_lte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.event_target_dp_lte"), card_id, errors)
        }),
        event_target_dp_gte: p.event_target_dp_gte.as_ref().map(|d| {
            compile_dp_constraint(d, &format!("{prefix}.event_target_dp_gte"), card_id, errors)
        }),
        event_target_name_contains: p.event_target_name_contains.clone(),
        event_target_is_player: p.event_target_is_player,
        event_target_is_source: p.event_target_is_source,
        event_target_was_self: p.event_target_was_self,
        attack_target_change_reason: p.attack_target_change_reason.clone(),
        attacker_trait_has: p.attacker_trait_has.clone(),
        event_target_owner: p.event_target_owner.map(compile_player_ref),
        event_add_to_hand_player: p.event_add_to_hand_player.map(compile_player_ref),
        event_target_color_any_of: p
            .event_target_color_any_of
            .as_ref()
            .map(|v| v.iter().map(|c| compile_color(*c)).collect()),
        event_permanent_is_source: p.event_permanent_is_source,
        source_deleted_battle_opponent: p.source_deleted_battle_opponent,
        event_host_permanent_is_source: p.event_host_permanent_is_source,
        event_is_effect_initiated: p.event_is_effect_initiated,
        event_card_trait_has: p.event_card_trait_has.clone(),
        event_card_name_contains: p.event_card_name_contains.clone(),
        event_card_level_eq: p.event_card_level_eq,
        event_card_level_gte: p.event_card_level_gte.as_ref().map(|d| {
            compile_dp_constraint(
                d,
                &format!("{prefix}.event_card_level_gte"),
                card_id,
                errors,
            )
        }),
        event_card_color_only: p
            .event_card_color_only
            .as_ref()
            .map(|colors| colors.iter().copied().map(compile_color).collect()),
        event_card_color_has: p
            .event_card_color_has
            .as_ref()
            .map(|colors| colors.iter().copied().map(compile_color).collect()),
        event_card_color_count: p.event_card_color_count,
        event_target_same_level_as_previous: p.event_target_same_level_as_previous,
        event_cause: p.event_cause.map(compile_event_cause),
        host_permanent_trait_has: p.host_permanent_trait_has.clone(),
        trashed_source_trait_has: p.trashed_source_trait_has.clone(),
        trashed_source_card_id_is: p.trashed_source_card_id_is.clone(),
        replacement_cause: p.replacement_cause.map(compile_replacement_cause),
        replacement_source_is_opponent: p.replacement_source_is_opponent,
        replacement_subject_is_mine: p.replacement_subject_is_mine,
        would_link_card_trait_any_of: p.would_link_card_trait_any_of.clone(),
        equals: p
            .equals
            .as_ref()
            .map(|v| v.iter().map(compile_binding_compare).collect()),
        not_equals: p
            .not_equals
            .as_ref()
            .map(|v| v.iter().map(compile_binding_compare).collect()),
        binding_exists: p.binding_exists.clone(),
        binding_present: p.binding_present.clone(),
        binding_absent: p.binding_absent.clone(),
        effect_suspended_any_own_digimon: p.effect_suspended_any_own_digimon,
        effect_suspended_any_opponent_digimon: p.effect_suspended_any_opponent_digimon,
        effect_returned_any_card: p.effect_returned_any_card,
        returned_card_matching: p.returned_card_matching.as_ref().map(|b| {
            Box::new(compile_predicate(
                b,
                &format!("{prefix}.returned_card_matching"),
                card_id,
                errors,
            ))
        }),
        effect_deleted_any_own_digimon: p.effect_deleted_any_own_digimon,
        effect_deleted_any_opponent_digimon: p.effect_deleted_any_opponent_digimon,
        effect_played_any_digimon: p.effect_played_any_digimon,
        effect_digivolved_any_digimon: p.effect_digivolved_any_digimon,
        effect_added_any_card_to_hand: p.effect_added_any_card_to_hand,
        count_lte: p.count_lte.as_ref().map(|c| CompiledCountAggregate {
            filter: Box::new(compile_predicate(
                &c.filter,
                &format!("{prefix}.count_lte.filter"),
                card_id,
                errors,
            )),
            n: compile_dp_constraint(&c.n, &format!("{prefix}.count_lte.n"), card_id, errors),
        }),
        count_gte: p.count_gte.as_ref().map(|c| CompiledCountAggregate {
            filter: Box::new(compile_predicate(
                &c.filter,
                &format!("{prefix}.count_gte.filter"),
                card_id,
                errors,
            )),
            n: compile_dp_constraint(&c.n, &format!("{prefix}.count_gte.n"), card_id, errors),
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
        any_field_permanent: p.any_field_permanent.as_ref().map(|e| {
            Box::new(CompiledExistential {
                of: compile_player_ref(e.of),
                predicate: compile_predicate(
                    &e.predicate,
                    &format!("{prefix}.any_field_permanent"),
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
        cost_target: p.cost_target.as_ref().map(|b| {
            Box::new(compile_predicate(
                b,
                &format!("{prefix}.cost_target"),
                card_id,
                errors,
            ))
        }),
        source_is_cost_target_permanent: p.source_is_cost_target_permanent,
    }
}

fn compile_replacement_cause(
    cause: crate::predicate::ReplacementCauseSpec,
) -> CompiledReplacementCause {
    match cause {
        crate::predicate::ReplacementCauseSpec::Battle => CompiledReplacementCause::Battle,
        crate::predicate::ReplacementCauseSpec::OwnEffect => CompiledReplacementCause::OwnEffect,
        crate::predicate::ReplacementCauseSpec::OpponentEffect => {
            CompiledReplacementCause::OpponentEffect
        }
        crate::predicate::ReplacementCauseSpec::SecurityCheck => {
            CompiledReplacementCause::SecurityCheck
        }
        crate::predicate::ReplacementCauseSpec::Cost => CompiledReplacementCause::Cost,
        crate::predicate::ReplacementCauseSpec::Overclock => CompiledReplacementCause::Overclock,
    }
}

fn compile_event_cause(cause: crate::predicate::EventCauseSpec) -> CompiledEventCause {
    match cause {
        crate::predicate::EventCauseSpec::BattleDeletion => CompiledEventCause::BattleDeletion,
        crate::predicate::EventCauseSpec::EffectDeletion => CompiledEventCause::EffectDeletion,
        crate::predicate::EventCauseSpec::OwnEffect => CompiledEventCause::OwnEffect,
        crate::predicate::EventCauseSpec::OpponentEffect => CompiledEventCause::OpponentEffect,
        crate::predicate::EventCauseSpec::Overclock => CompiledEventCause::Overclock,
        crate::predicate::EventCauseSpec::Return => CompiledEventCause::Return,
        crate::predicate::EventCauseSpec::DeckBottom => CompiledEventCause::DeckBottom,
        crate::predicate::EventCauseSpec::SecurityPlacement => {
            CompiledEventCause::SecurityPlacement
        }
        crate::predicate::EventCauseSpec::SecurityRemoval => CompiledEventCause::SecurityRemoval,
        crate::predicate::EventCauseSpec::Cost => CompiledEventCause::Cost,
        crate::predicate::EventCauseSpec::Rule => CompiledEventCause::Rule,
    }
}

fn compile_dp_constraint(
    d: &crate::predicate::DpConstraint,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledDpConstraint {
    use crate::predicate::DpConstraint as S;
    match d {
        S::Literal(n) => CompiledDpConstraint::Literal(*n),
        S::Formula(f) => CompiledDpConstraint::Formula(compile_formula(f, prefix, card_id, errors)),
    }
}

fn per_with_bucket(
    per: &crate::formula::PerSelector,
    bucket: Option<u32>,
) -> crate::formula::PerSelector {
    match per {
        crate::formula::PerSelector::SharedTrashCount { bucket: inner } => {
            crate::formula::PerSelector::SharedTrashCount {
                bucket: inner.or(bucket),
            }
        }
        other => other.clone(),
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
        cost: ap
            .cost
            .as_ref()
            .map(|c| compile_cost(c, &format!("{prefix}.cost"), card_id, errors)),
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
        condition: ap.condition.as_ref().map(|p| {
            Box::new(compile_predicate(
                p,
                &format!("{prefix}.condition"),
                card_id,
                errors,
            ))
        }),
        direction: match ap.direction {
            crate::alt_path::AltPathDirection::From => {
                crate::compiled::CompiledAltPathDirection::From
            }
            crate::alt_path::AltPathDirection::Into => {
                crate::compiled::CompiledAltPathDirection::Into
            }
        },
    }
}

fn compile_alt_path_kind(k: crate::alt_path::AltPathKind) -> CompiledAltPathKind {
    use crate::alt_path::AltPathKind as S;
    match k {
        S::Digivolve => CompiledAltPathKind::Digivolve,
        S::DnaDigivolve => CompiledAltPathKind::DnaDigivolve,
        S::BlastDnaDigivolve => CompiledAltPathKind::BlastDnaDigivolve,
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
        cost_delta: m.cost_delta,
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

fn compile_cost(
    c: &crate::alt_path::CostSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledCost {
    use crate::alt_path::{CostSpec, FormulaCost};
    match c {
        CostSpec::Literal(n) => CompiledCost::Literal(*n),
        CostSpec::Formula(FormulaCost { formula }) => {
            CompiledCost::Formula(compile_formula(formula, prefix, card_id, errors))
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
    // Phase 2 Track B — `activation_cost:` is only valid as the FIRST
    // step of a triggered clause body. The lowering on the engine side
    // lifts it onto `EffectBuilder::activation_cost(...)`; mid-body uses
    // would silently no-op at runtime, so reject at compile time.
    for (i, s) in t.process.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if matches!(s, crate::step::StepSpec::ActivationCost(_)) {
            errors.push(ValidationError {
                card_id: card_id.to_string(),
                path: format!("{prefix}.process[{i}]"),
                message:
                    "activation_cost must be the first step of a triggered clause body — it is lifted onto Effect::activation_cost and cannot appear mid-body"
                        .to_string(),
            });
        }
    }
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
        outer_prompt: t.outer_prompt,
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

fn compile_replacement_process(
    r: &crate::clause::ReplacementBody,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> Vec<CompiledStep> {
    if !r.process.is_empty() {
        return r
            .process
            .iter()
            .enumerate()
            .map(|(i, s)| compile_step(s, &format!("{prefix}.process[{i}]"), card_id, errors))
            .collect();
    }

    let mut process = Vec::new();

    // Gap 3a — `cost: { trash_own_link_card: true }` synthesizes a single
    // self-contained step that trashes a chosen link card AND cancels the
    // leave. It owns the `outcome: prevent`, so we emit only this step (and
    // suppress the trailing `CancelReplacement` below).
    let trash_own_link_card = r
        .cost
        .as_ref()
        .is_some_and(|cost| cost.trash_own_link_card);
    if trash_own_link_card {
        if !matches!(r.outcome, Some(crate::clause::ReplacementOutcome::Prevent)) {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.cost"),
                message: "cost: { trash_own_link_card } requires outcome: prevent".into(),
            });
        }
        process.push(CompiledStep::TrashOwnLinkCardAndCancelLeave);
        return process;
    }

    if r.cost.as_ref().is_some_and(|cost| cost.delay_self) {
        process.push(CompiledStep::DeletePermanent {
            target: CompiledBindingRef::Source,
        });
    }

    if let Some(choose) = &r.choose {
        if choose.min != 1 || choose.max != 1 {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.choose"),
                message: "replacement choose currently supports exactly min: 1, max: 1".into(),
            });
        }

        process.push(CompiledStep::SelectHand {
            of: CompiledPlayerRef::You,
            filter: compile_predicate(
                &choose.card_filter,
                &format!("{prefix}.choose.card_filter"),
                card_id,
                errors,
            ),
            bind_as: Some("chosen".into()),
            prompt: format!("Choose a card for {card_id}"),
            prompt_key: None,
            optional: true,
            cost: false,
        });
    }

    for (i, step) in r.then_steps.iter().enumerate() {
        let args = &step.digivolve_without_cost;
        process.push(CompiledStep::EffectInitiatedDigivolve {
            target: compile_binding_ref(&args.target),
            from_hand: compile_binding_ref(&args.card),
            cost: CompiledCostDelta::Free,
            ignore_requirements: true,
        });
        if r.choose.is_none() {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.then[{i}]"),
                message: "replacement then steps currently require a choose block".into(),
            });
        }
    }

    if matches!(r.outcome, Some(crate::clause::ReplacementOutcome::Prevent)) {
        process.push(CompiledStep::CancelReplacement);
    }

    process
}

fn compile_replacement_trigger(
    r: &crate::clause::ReplacementBody,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> String {
    let trigger = r
        .trigger
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let timing = r
        .timing
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let value = match (trigger, timing) {
        (Some(_), Some(_)) | (None, None) => {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: prefix.into(),
                message: "replacement requires exactly one of timing or trigger".into(),
            });
            trigger.or(timing).unwrap_or_default()
        }
        (Some(value), None) | (None, Some(value)) => value,
    };

    if !value.is_empty() && !is_known_replacement_timing(value) {
        errors.push(ValidationError {
            card_id: card_id.into(),
            path: format!("{prefix}.timing"),
            message: format!("unknown replacement timing: {value}"),
        });
    }

    value.to_string()
}

fn is_known_replacement_timing(value: &str) -> bool {
    matches!(
        value,
        "when_would_be_deleted"
            | "when_would_leave_battle_area"
            | "when_would_be_returned_to_hand"
            | "when_would_be_returned_to_deck"
            | "when_would_be_trashed"
            | "when_would_be_de_digivolved"
            | "when_would_lose_security"
            | "when_would_draw"
            | "when_would_place_in_security"
            | "when_would_digivolve"
            | "when_would_play"
            | "when_would_link"
    )
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
            target: a
                .target
                .as_ref()
                .map(|target| {
                    compile_predicate(target, &format!("{prefix}.target"), card_id, errors)
                })
                .unwrap_or_default(),
            target_player: a.target_player.map(compile_player_ref),
            dp_modifier: a.dp_modifier,
            dp_modifier_fn: a
                .dp_modifier_fn
                .as_ref()
                .map(|f| compile_formula(f, &format!("{prefix}.dp_modifier_fn"), card_id, errors)),
            security_attack: a.security_attack,
            security_attack_fn: a.security_attack_fn.as_ref().map(|f| {
                compile_formula(f, &format!("{prefix}.security_attack_fn"), card_id, errors)
            }),
            grant_keyword: a.grant_keyword.map(|gk| CompiledGrantKeywordValue {
                keyword: gk.keyword,
                value: gk.value,
            }),
            modifier: a.modifier,
            modifier_value: a.modifier_value,
            modifier_name: a.modifier_name,
            while_condition: a.while_condition.as_ref().map(|p| {
                compile_predicate(p, &format!("{prefix}.while_condition"), card_id, errors)
            }),
            applies_to_opponent_security_dp: a.applies_to_opponent_security_dp.unwrap_or(false),
            applies_to_own_security_dp: a.applies_to_own_security_dp.unwrap_or(false),
            effect_immunity: a.effect_immunity.map(|imm| CompiledAuraEffectImmunity {
                source_kind: imm.source_kind.map(compile_effect_source_kind),
                source_controller: compile_effect_controller(imm.source_controller),
            }),
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
            when_any_ally_digivolves_into: c.when_any_ally_digivolves_into.as_ref().map(|p| {
                compile_predicate(
                    p,
                    &format!("{prefix}.when_any_ally_digivolves_into"),
                    card_id,
                    errors,
                )
            }),
            condition: c
                .condition
                .as_ref()
                .map(|p| compile_predicate(p, &format!("{prefix}.condition"), card_id, errors)),
            optional: c.optional,
            once_per_turn: c.once_per_turn,
            amount: c.amount,
            amount_fn: c
                .amount_fn
                .as_ref()
                .map(|f| compile_formula(f, &format!("{prefix}.amount_fn"), card_id, errors)),
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
        B::Replacement(r) => {
            let trigger = compile_replacement_trigger(&r, prefix, card_id, errors);
            CompiledDeclarativeClause::Replacement {
                scope,
                active_when,
                trigger,
                optional: r.optional,
                once_per_turn: r.once_per_turn,
                process: compile_replacement_process(&r, prefix, card_id, errors),
                summary,
                summary_key,
            }
        }
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
            process: p
                .process
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.process[{i}]"), card_id, errors))
                .collect(),
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
            overclock_cost_filter: gk.overclock_cost_filter.as_ref().map(|p| {
                compile_predicate(
                    p,
                    &format!("{prefix}.overclock_cost_filter"),
                    card_id,
                    errors,
                )
            }),
            active_when,
            summary,
            summary_key,
        },
        B::Delay(dl) => {
            if active_when.is_some() && dl.active_when.is_some() {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.delay.active_when"),
                    message: "delay.active_when cannot be combined with clause active_when yet"
                        .into(),
                });
            }
            let delay_active_when = dl.active_when.as_ref().map(|p| {
                compile_predicate(p, &format!("{prefix}.delay.active_when"), card_id, errors)
            });
            CompiledDeclarativeClause::Delay {
                scope,
                active_when: delay_active_when.or(active_when),
                trigger: compile_timing(dl.trigger),
                process: dl
                    .process
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        compile_step(s, &format!("{prefix}.process[{i}]"), card_id, errors)
                    })
                    .collect(),
                summary,
                summary_key,
            }
        }
        B::LinkRequirement(link) => CompiledDeclarativeClause::LinkRequirement {
            scope,
            cost: link.cost,
            filter: compile_predicate(&link.filter, &format!("{prefix}.filter"), card_id, errors),
            summary,
            summary_key,
        },
        B::LinkCondition(link) => CompiledDeclarativeClause::LinkCondition {
            scope,
            cost: link.cost,
            filter: compile_predicate(&link.filter, &format!("{prefix}.filter"), card_id, errors),
            summary,
            summary_key,
        },
        B::FloodGate(fg) => CompiledDeclarativeClause::FloodGate {
            scope,
            active_when,
            modifier: fg.modifier,
            target: fg.target.as_ref().map(|target| {
                compile_predicate(target, &format!("{prefix}.target"), card_id, errors)
            }),
            target_player: fg.target_player.map(compile_player_ref),
            expiry: fg.expiry,
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
            "this" => CompiledBindingRef::Source,
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
            deck_top,
            ..
        }) => {
            if let Some(p) = permanent {
                CompiledBindingRef::Permanent(p.clone())
            } else if let Some(b) = binding {
                CompiledBindingRef::Binding(b.clone())
            } else if let Some(o) = of_permanent {
                CompiledBindingRef::OfPermanent(o.clone())
            } else if let Some(p) = deck_top {
                CompiledBindingRef::DeckTop(compile_player_ref(*p))
            } else {
                CompiledBindingRef::Named(String::new())
            }
        }
    }
}

fn compile_effect_source_kind(kind: crate::step::EffectSourceKindSpec) -> CompiledEffectSourceKind {
    match kind {
        crate::step::EffectSourceKindSpec::Digimon => CompiledEffectSourceKind::Digimon,
        crate::step::EffectSourceKindSpec::Tamer => CompiledEffectSourceKind::Tamer,
        crate::step::EffectSourceKindSpec::Option => CompiledEffectSourceKind::Option,
        crate::step::EffectSourceKindSpec::Rule => CompiledEffectSourceKind::Rule,
    }
}

fn compile_effect_controller(
    controller: crate::step::EffectControllerSpec,
) -> CompiledEffectController {
    match controller {
        crate::step::EffectControllerSpec::Any => CompiledEffectController::Any,
        crate::step::EffectControllerSpec::Opponent => CompiledEffectController::Opponent,
        crate::step::EffectControllerSpec::Own => CompiledEffectController::Own,
    }
}

fn compile_modifier_value(
    v: &crate::step::ModifierValueSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledModifierValue {
    use crate::step::ModifierValueSpec as S;
    match v {
        S::Literal(n) => CompiledModifierValue::Literal(*n),
        S::Formula(fc) => {
            CompiledModifierValue::Formula(compile_formula(&fc.formula, prefix, card_id, errors))
        }
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
        S::GainMemoryFn(a) => CompiledStep::GainMemoryFn {
            formula: compile_formula(
                &a.formula,
                &format!("{prefix}.gain_memory_fn"),
                card_id,
                errors,
            ),
        },
        S::LoseMemoryFn(a) => CompiledStep::LoseMemoryFn {
            formula: compile_formula(
                &a.formula,
                &format!("{prefix}.lose_memory_fn"),
                card_id,
                errors,
            ),
        },

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
        S::AddToHandFromSecurity(a) => CompiledStep::AddToHandFromSecurity {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::PlaySecurityCard(a) => CompiledStep::PlaySecurityCard {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::TrashSelectedSecurity(a) => CompiledStep::TrashSelectedSecurity {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::ReturnSelectedSecurityToDeck(a) => CompiledStep::ReturnSelectedSecurityToDeck {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
            position: compile_stack_position(a.position),
        },
        S::AddTopSecurityToHand(a) => CompiledStep::AddTopSecurityToHand {
            of: compile_player_ref(a.of),
        },
        S::MayAddTopSecurityToHand(a) => CompiledStep::MayAddTopSecurityToHand {
            of: compile_player_ref(a.of),
        },
        S::AddToHandFromReveal(a) => CompiledStep::AddToHandFromReveal {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::AddThisOptionToHand(_) => CompiledStep::AddThisOptionToHand,
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
        S::ShuffleSecurity(a) => CompiledStep::ShuffleSecurity {
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
        S::ChooseFromReveal(a) => CompiledStep::ChooseFromReveal {
            of: compile_player_ref(a.of),
            filter: compile_predicate(
                &a.filter,
                &format!("{prefix}.choose_from_reveal.filter"),
                card_id,
                errors,
            ),
            destination: compile_reveal_destination(&a.destination),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::OrderRemainder(a) => {
            if a.destinations.is_empty() {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.order_remainder.destinations"),
                    message: "order_remainder requires at least one destination".to_string(),
                });
            }
            if a.destinations.len() > 2 {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.order_remainder.destinations"),
                    message: format!(
                        "order_remainder supports at most 2 destinations (got {}); printed text variants beyond [deck_top, deck_bottom] are not modelled",
                        a.destinations.len()
                    ),
                });
            }
            CompiledStep::OrderRemainder {
                of: compile_player_ref(a.of),
                destinations: a
                    .destinations
                    .iter()
                    .map(|d| compile_remainder_destination(*d))
                    .collect(),
                prompt: a.prompt.clone(),
                prompt_key: a.prompt_key.clone(),
            }
        }

        S::DeletePermanent(a) => CompiledStep::DeletePermanent {
            target: compile_binding_ref(&a.target),
        },
        S::DeleteBoundPermanents(a) => CompiledStep::DeleteBoundPermanents {
            binding: a.binding.clone(),
        },
        S::TrashBreedingPermanent(a) => CompiledStep::TrashBreedingPermanent {
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
            amount_fn: a
                .amount_fn
                .as_ref()
                .map(|f| compile_formula(f, &format!("{prefix}.amount_fn"), card_id, errors)),
            stop_at_level: a.stop_at_level.or(Some(3)),
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
            bind_as: a.bind_as.clone(),
        },
        S::PlaceAsBottomSource(a) => CompiledStep::PlaceAsBottomSource {
            source: compile_binding_ref(&a.source),
            target: compile_binding_ref(&a.target),
            face_down: a.face_down,
        },
        S::PlaceTopSourceAsBottom(a) => CompiledStep::PlaceTopSourceAsBottom {
            target: compile_binding_ref(&a.target),
        },
        S::TrashTopSource(a) => CompiledStep::TrashTopSource {
            target: compile_binding_ref(&a.target),
        },
        S::TrashBottomSources(a) => CompiledStep::TrashBottomSources {
            target: compile_binding_ref(&a.target),
            count: a.count,
        },
        S::TrashAllSources(a) => CompiledStep::TrashAllSources {
            target: compile_binding_ref(&a.target),
        },
        S::TrashBottomFaceDownSourceUnderTamer(a) => {
            CompiledStep::TrashBottomFaceDownSourceUnderTamer {
                of: compile_player_ref(a.of),
            }
        }
        S::TrashSelectedSources(a) => CompiledStep::TrashSelectedSources {
            source_refs: a.source_refs.clone(),
        },
        S::PlaceSelectedCardUnderTamer(a) => CompiledStep::PlaceSelectedCardUnderTamer {
            card: compile_binding_ref(&a.card),
            tamer: compile_binding_ref(&a.tamer),
            face_down: a.face_down,
            bind_as: a.bind_as.clone(),
        },
        S::PlaceSelectedSourcesUnderTamer(a) => CompiledStep::PlaceSelectedSourcesUnderTamer {
            source_refs: a.source_refs.clone(),
            tamer: compile_binding_ref(&a.tamer),
            bind_count_as: a.bind_count_as.clone(),
        },
        S::MoveMatchingSourcesUnderTamer(a) => CompiledStep::MoveMatchingSourcesUnderTamer {
            from: compile_binding_ref(&a.from),
            tamer: compile_binding_ref(&a.tamer),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_count_as: a.bind_count_as.clone(),
        },
        S::TrashTopStackedSources(a) => CompiledStep::TrashTopStackedSources {
            target: compile_binding_ref(&a.target),
            count: compile_formula(&a.count, &format!("{prefix}.count"), card_id, errors),
        },
        S::ReturnSelectedSourcesToHand(a) => CompiledStep::ReturnSelectedSourcesToHand {
            source_refs: a.source_refs.clone(),
        },
        S::BindPermanentProperty(a) => CompiledStep::BindPermanentProperty {
            from: compile_binding_ref(&a.from),
            property: match a.property {
                crate::step::PermanentProperty::Level => {
                    crate::compiled::CompiledPermanentProperty::Level
                }
            },
            bind_as: a.bind_as.clone(),
        },
        S::Hatch(a) => CompiledStep::Hatch {
            of: compile_player_ref(a.of),
        },
        S::MoveFromBreeding(a) => CompiledStep::MoveFromBreeding {
            of: compile_player_ref(a.of),
        },

        // PlayFromHand, PlayFromTrash, PlayFromTrashFree all use PlayFromHandArgs
        // (with hand_index field), but the compiled variants use different field
        // names for trash steps.
        //
        // PUPPETS-G030: `suppress_on_play` is honored ONLY by
        // `play_from_trash_free`. Reject it on the other play steps so an
        // author cannot silently expect a no-op flag to take effect.
        S::PlayFromHand(a) => {
            if a.suppress_on_play {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.play_from_hand.suppress_on_play"),
                    message: "suppress_on_play is only supported on play_from_trash_free"
                        .to_string(),
                });
            }
            CompiledStep::PlayFromHand {
                of: compile_player_ref(a.of),
                hand_index: compile_binding_ref(&a.hand_index),
                cost_delta: a
                    .cost_delta
                    .as_ref()
                    .map(|c| compile_cost_delta(c, prefix, card_id, errors)),
            }
        }
        S::PlayFromHandFree(a) => CompiledStep::PlayFromHandFree {
            of: compile_player_ref(a.of),
            hand_index: compile_binding_ref(&a.hand_index),
            bind_as: a.bind_as.clone(),
        },
        S::UseOptionFromHand(a) => CompiledStep::UseOptionFromHand {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            use_cost_lte_opponent_memory: a.use_cost_lte_opponent_memory,
            optional: a.optional,
            prompt: a.prompt.clone(),
        },
        S::PlayFromRevealedFree(a) => CompiledStep::PlayFromRevealedFree {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
            bind_as: a.bind_as.clone(),
            cost_delta: a
                .cost_delta
                .as_ref()
                .map(|c| compile_cost_delta(c, prefix, card_id, errors)),
        },
        // PlayFromTrash reuses PlayFromHandArgs but the compiled form uses `trash_index`
        S::PlayFromTrash(a) => {
            if a.suppress_on_play {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.play_from_trash.suppress_on_play"),
                    message: "suppress_on_play is only supported on play_from_trash_free"
                        .to_string(),
                });
            }
            CompiledStep::PlayFromTrash {
                of: compile_player_ref(a.of),
                trash_index: compile_binding_ref(&a.hand_index),
                cost_delta: a
                    .cost_delta
                    .as_ref()
                    .map(|c| compile_cost_delta(c, prefix, card_id, errors)),
            }
        }
        S::PlayFromTrashFree(a) => CompiledStep::PlayFromTrashFree {
            of: compile_player_ref(a.of),
            trash_index: compile_binding_ref(&a.hand_index),
            suppress_on_play: a.suppress_on_play,
            suspended: a.suspended,
        },
        S::PlayUnionBoundFree(a) => CompiledStep::PlayUnionBoundFree {
            binding: a.binding.clone(),
            bind_as: a.bind_as.clone(),
            suppress_on_play: a.suppress_on_play,
        },
        S::TrashUnionBound(a) => CompiledStep::TrashUnionBound {
            binding: a.binding.clone(),
        },
        S::PlayFromSecurity(_) => CompiledStep::PlayFromSecurity,
        S::PlayFromMaterials(a) => CompiledStep::PlayFromMaterials {
            target: compile_binding_ref(&a.target),
            source_index: compile_binding_ref(&a.source_index),
            cost_delta: a
                .cost_delta
                .as_ref()
                .map(|c| compile_cost_delta(c, prefix, card_id, errors)),
            suppress_on_play: a.suppress_on_play,
            bind_as: a.bind_as.clone(),
        },
        S::PlaySelectedSourcesFree(a) => CompiledStep::PlaySelectedSourcesFree {
            source_refs: a.source_refs.clone(),
        },
        S::PlayUnderTamerSource(a) => CompiledStep::PlayUnderTamerSource {
            source_refs: a.source_refs.clone(),
            cost_delta: a
                .cost_delta
                .as_ref()
                .map(|c| compile_cost_delta(c, prefix, card_id, errors)),
            bind_as: a.bind_as.clone(),
        },
        S::EffectInitiatedDigivolve(a) => CompiledStep::EffectInitiatedDigivolve {
            target: compile_binding_ref(&a.target),
            from_hand: match a.source.as_ref().or(a.from_hand.as_ref()) {
                Some(source) => compile_binding_ref(source),
                None => {
                    errors.push(ValidationError {
                        card_id: card_id.to_string(),
                        path: format!("{prefix}.effect_initiated_digivolve"),
                        message: "effect_initiated_digivolve requires source or from_hand"
                            .to_string(),
                    });
                    CompiledBindingRef::SelfRef
                }
            },
            cost: compile_cost_delta(&a.cost, prefix, card_id, errors),
            ignore_requirements: a.ignore_requirements,
        },
        S::EffectInitiatedDnaDigivolve(a) => CompiledStep::EffectInitiatedDnaDigivolve {
            target_a: compile_binding_ref(&a.target_a),
            target_b: compile_binding_ref(&a.target_b),
            from_hand: compile_binding_ref(&a.from_hand),
            cost: compile_cost_delta(&a.cost, prefix, card_id, errors),
            ignore_requirements: a.ignore_requirements,
        },
        S::EffectInitiatedDnaDigivolveHandPartner(a) => {
            CompiledStep::EffectInitiatedDnaDigivolveHandPartner {
                target: compile_binding_ref(&a.target),
                hand_partner: compile_binding_ref(&a.hand_partner),
                from_hand: compile_binding_ref(&a.from_hand),
                cost: compile_cost_delta(&a.cost, prefix, card_id, errors),
                ignore_requirements: a.ignore_requirements,
            }
        }

        S::TrashTopSecurity(a) => CompiledStep::TrashTopSecurity {
            of: compile_player_ref(a.of),
            count: a
                .count
                .as_ref()
                .map(|f| compile_formula(f, &format!("{prefix}.count"), card_id, errors)),
        },
        S::TrashBottomSecurity(a) => CompiledStep::TrashBottomSecurity {
            of: compile_player_ref(a.of),
        },
        S::AddBottomSecurityToHand(a) => CompiledStep::AddBottomSecurityToHand {
            of: compile_player_ref(a.of),
        },
        S::TrashTopSecurityAndCancelReplacement(a) => {
            CompiledStep::TrashTopSecurityAndCancelReplacement {
                of: compile_player_ref(a.of),
            }
        }
        S::BounceSelf(_) => CompiledStep::BounceSelf,
        S::PlaceSelfAtSecurity(a) => CompiledStep::PlaceSelfAtSecurity {
            position: compile_stack_position(a.position),
            face_up: a.face.is_up(),
        },
        S::PlaceSelfOptionAtSecurity(a) => CompiledStep::PlaceSelfOptionAtSecurity {
            position: compile_stack_position(a.position),
            face_up: a.face.is_up(),
        },
        S::PlacePermanentBottomSecurityAndCancelReplacement(a) => {
            CompiledStep::PlacePermanentBottomSecurityAndCancelReplacement {
                of: compile_player_ref(a.of),
                target: compile_binding_ref(&a.target),
            }
        }
        S::PlacePermanentOnSecurity(a) => CompiledStep::PlacePermanentOnSecurity {
            of: compile_player_ref(a.of),
            target: compile_binding_ref(&a.target),
            position: compile_stack_position(a.position),
            face_up: a.face_up,
        },
        S::PlacePermanentOnSecurityAndHandleReplacement(a) => {
            CompiledStep::PlacePermanentOnSecurityAndHandleReplacement {
                of: compile_player_ref(a.of),
                target: compile_binding_ref(&a.target),
                position: compile_stack_position(a.position),
                face_up: a.face_up,
            }
        }
        S::PlacePermanentOnSecurityObserved(a) => CompiledStep::PlacePermanentOnSecurityObserved {
            of: compile_player_ref(a.of),
            target: compile_binding_ref(&a.target),
            position: compile_stack_position(a.position),
            face_up: a.face.is_up(),
            include_sources: a.include_sources,
        },
        S::SecurityPlaceStackedCard(a) => {
            if a.source.is_none() && a.source_index_from_top.is_none() {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.security_place_stacked_card"),
                    message: "security_place_stacked_card requires source or source_index_from_top"
                        .to_string(),
                });
            }
            CompiledStep::SecurityPlaceStackedCard {
                carrier: compile_binding_ref(&a.carrier),
                source: a.source.as_ref().map(compile_binding_ref),
                source_index_from_top: a.source_index_from_top,
                of: compile_player_ref(a.of),
                position: compile_stack_position(a.position),
                face_up: a.face.is_up(),
            }
        }
        S::SecurityPlaceTopStackedCard(a) => CompiledStep::SecurityPlaceTopStackedCard {
            carrier: compile_binding_ref(&a.carrier),
            of: compile_player_ref(a.of),
            position: compile_stack_position(a.position),
            face_up: a.face.is_up(),
        },
        S::ReturnAllTrashToDeckBottom(a) => CompiledStep::ReturnAllTrashToDeckBottom {
            of: compile_player_ref(a.of),
        },
        S::ReturnTrashListToDeckBottom(a) => CompiledStep::ReturnTrashListToDeckBottom {
            of: compile_player_ref(a.of),
            cards: compile_binding_ref(&a.cards),
            to_top: matches!(a.destination, crate::step::DeckDestination::Top),
        },
        S::MoveTrashCardToDeckTop(a) => CompiledStep::MoveTrashCardToDeckTop {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::TrashTopNDigivolutionCardsOfEach(a) => CompiledStep::TrashTopNDigivolutionCardsOfEach {
            of: compile_player_ref(a.of),
            n: compile_formula(&a.n, &format!("{prefix}.n"), card_id, errors),
        },
        S::TrashOpponentHandToCount(a) => CompiledStep::TrashOpponentHandToCount {
            opponent: compile_player_ref(a.opponent),
            target_count: compile_formula(
                &a.target_count,
                &format!("{prefix}.target_count"),
                card_id,
                errors,
            ),
        },
        S::SearchOwnSecurityStack(a) => CompiledStep::SearchOwnSecurityStack {
            filter: Box::new(compile_predicate(
                &a.filter,
                &format!("{prefix}.filter"),
                card_id,
                errors,
            )),
            prompt: a.prompt.clone(),
            bind_as: a.bind_as.clone(),
            optional: a.optional,
            on_select: a
                .on_select
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.on_select[{i}]"), card_id, errors))
                .collect(),
            on_no_match: a.on_no_match.as_ref().map(|steps| {
                steps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        compile_step(s, &format!("{prefix}.on_no_match[{i}]"), card_id, errors)
                    })
                    .collect()
            }),
        },
        S::Recover(a) => CompiledStep::Recover {
            of: compile_player_ref(a.of),
            count: a.count,
        },
        S::MarkSecurityFaceUp(a) => CompiledStep::MarkSecurityFaceUp {
            of: compile_player_ref(a.of),
            card: compile_binding_ref(&a.card),
        },
        S::FlipSecurityFaceUp(a) => CompiledStep::FlipSecurityFaceUp {
            of: compile_player_ref(a.of),
        },

        S::AddDpModifier(a) => CompiledStep::AddDpModifier {
            target: compile_binding_ref(&a.target),
            value: compile_modifier_value(&a.value, &format!("{prefix}.value"), card_id, errors),
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
            value: compile_modifier_value(&a.value, &format!("{prefix}.value"), card_id, errors),
            expiry: a.expiry.clone(),
            synth_identity: a.synth_identity.as_ref().map(compile_synth_identity),
            continuous: a.continuous,
        },
        S::AddPlayerModifier(a) => CompiledStep::AddPlayerModifier {
            target_player: compile_player_ref(a.target_player),
            modifier: a.modifier.clone(),
            value: a
                .value
                .as_ref()
                .map(|v| compile_modifier_value(v, &format!("{prefix}.value"), card_id, errors))
                .unwrap_or(CompiledModifierValue::Literal(0)),
            expiry: a.expiry.clone(),
        },
        S::GrantKeyword(a) => CompiledStep::GrantKeyword {
            target: compile_binding_ref(&a.target),
            keyword: a.keyword.clone(),
            expiry: a.expiry.clone(),
            value: a.value,
        },
        S::AllowDigixrosMaterialZone(a) => CompiledStep::AllowDigixrosMaterialZone {
            zone: compile_zone(a.zone),
            max_count: a.max_count,
        },
        S::AddDigixrosCostDelta(a) => CompiledStep::AddDigixrosCostDelta { delta: a.delta },
        S::PreattachDigixrosMaterial(a) => CompiledStep::PreattachDigixrosMaterial {
            card: compile_binding_ref(&a.card),
            cost_delta: a.cost_delta.unwrap_or(0),
        },
        S::RegisterDigixrosWildcardForTurn(a) => CompiledStep::RegisterDigixrosWildcardForTurn {
            card: compile_binding_ref(&a.card),
            zone: validate_digixros_wildcard_zone(
                a.zone,
                &format!("{prefix}.register_digixros_wildcard_for_turn"),
                card_id,
                errors,
            ),
        },
        S::AddDigixrosWildcardToPendingTransaction(a) => {
            CompiledStep::AddDigixrosWildcardToPendingTransaction {
                card: compile_binding_ref(&a.card),
                zone: validate_digixros_wildcard_zone(
                    a.zone,
                    &format!("{prefix}.add_digixros_wildcard_to_pending_transaction"),
                    card_id,
                    errors,
                ),
            }
        }
        S::GrantTriggeredEffect(a) => CompiledStep::GrantTriggeredEffect {
            target: compile_modifier_target(
                &a.target,
                &format!("{prefix}.target"),
                card_id,
                errors,
            ),
            timing: a.timing.clone(),
            expiry: a.expiry.clone(),
            body: a
                .body
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.body[{i}]"), card_id, errors))
                .collect(),
        },
        S::GrantEffectImmunity(a) => {
            // Continuous form requires `targets` and no `target`; the
            // per-permanent form requires `target` (Q28 / BT20-059).
            if a.continuous {
                if a.targets.is_none() || a.target.is_some() {
                    errors.push(ValidationError {
                        card_id: card_id.to_string(),
                        path: format!("{prefix}.grant_effect_immunity"),
                        message: "continuous: true requires `targets` and no `target`"
                            .to_string(),
                    });
                }
            } else if a.target.is_none() {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.grant_effect_immunity"),
                    message: "`target` is required unless `continuous: true`".to_string(),
                });
            }
            CompiledStep::GrantEffectImmunity {
                target: a.target.as_ref().map(compile_binding_ref),
                source_kind: compile_effect_source_kind(a.source_kind),
                source_controller: compile_effect_controller(a.source_controller),
                expiry: a.expiry.clone(),
                continuous: a.continuous,
                targets: a.targets.as_ref().map(|p| {
                    compile_predicate(
                        p,
                        &format!("{prefix}.grant_effect_immunity.targets"),
                        card_id,
                        errors,
                    )
                }),
            }
        }
        S::GrantNarrowOpponentEffectProtection(a) => {
            CompiledStep::GrantNarrowOpponentEffectProtection {
                target: compile_binding_ref(&a.target),
                expiry: a.expiry.clone(),
            }
        }

        S::SelectOwnPermanent(a) => CompiledStep::SelectOwnPermanent {
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            selector: a.selector.map(compile_field_selector),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
            continue_on_decline: a.continue_on_decline,
        },
        S::SelectOpponentPermanent(a) => CompiledStep::SelectOpponentPermanent {
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            selector: a.selector.map(compile_field_selector),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
            continue_on_decline: a.continue_on_decline,
        },
        S::SelectAnyPermanent(a) => CompiledStep::SelectAnyPermanent {
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            selector: a.selector.map(compile_field_selector),
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
            cost: a.cost,
        },
        S::SelectTrash(a) => CompiledStep::SelectTrash {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
            cost: a.cost,
        },
        S::SelectMaterial(a) => CompiledStep::SelectMaterial {
            of_permanent: compile_binding_ref(&a.of_permanent),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectMaterials(a) => CompiledStep::SelectMaterials {
            of_permanent: compile_binding_ref(&a.of_permanent),
            max: compile_count_bound(&a.max, &format!("{prefix}.max"), card_id, errors),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            uniqueness: a.uniqueness.map(compile_distinct_by),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional_zero: a.optional_zero,
        },
        S::SelectOwnSources(a) => CompiledStep::SelectOwnSources {
            target: a
                .target
                .as_ref()
                .or(a.from.as_ref())
                .map(compile_binding_ref),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            min: a.min,
            max: a.max,
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            then: a
                .then
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                .collect(),
        },
        S::SelectUnderTamerSources(a) => {
            let mut filter =
                compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors);
            match filter.host_kind_is {
                Some(crate::compiled::CompiledCardKind::Tamer) | None => {
                    filter.host_kind_is = Some(crate::compiled::CompiledCardKind::Tamer);
                }
                Some(_) => errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.select_under_tamer_sources.filter.host_kind_is"),
                    message: "select_under_tamer_sources requires host_kind_is: tamer".to_string(),
                }),
            }
            CompiledStep::SelectOwnSources {
                target: a
                    .target
                    .as_ref()
                    .or(a.from.as_ref())
                    .map(compile_binding_ref),
                filter,
                min: a.min,
                max: a.max,
                bind_as: a.bind_as.clone(),
                prompt: a.prompt.clone(),
                then: a
                    .then
                    .iter()
                    .enumerate()
                    .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                    .collect(),
            }
        }
        S::SelectOpponentSources(a) => CompiledStep::SelectOpponentSources {
            target: a.target.as_ref().map(compile_binding_ref),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            min: a.min,
            max: a.max,
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            then: a
                .then
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                .collect(),
        },
        S::DigiBurst(a) => {
            let bind_as = a
                .bind_as
                .clone()
                .unwrap_or_else(|| "__digi_burst_sources".to_string());
            let mut then = Vec::with_capacity(a.then.len() + 1);
            then.push(CompiledStep::TrashSelectedSources {
                source_refs: bind_as.clone(),
            });
            then.extend(
                a.then
                    .iter()
                    .enumerate()
                    .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors)),
            );
            CompiledStep::SelectOwnSources {
                target: Some(CompiledBindingRef::Source),
                filter: CompiledPredicate::default(),
                min: a.count,
                max: a.count,
                bind_as: Some(bind_as),
                prompt: a.prompt.clone(),
                then,
            }
        }
        S::SelectOpponentDpBudget(a) => CompiledStep::SelectOpponentDpBudget {
            dp_budget: compile_formula(
                &a.dp_budget,
                &format!("{prefix}.dp_budget"),
                card_id,
                errors,
            ),
            min_picks: a.min_picks,
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            then: a
                .then
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                .collect(),
        },
        S::SelectOpponentPlayCostBudget(a) => CompiledStep::SelectOpponentPlayCostBudget {
            play_cost_budget: a.play_cost_budget,
            min_picks: a.min_picks,
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            then: a
                .then
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                .collect(),
        },
        S::SelectOwnBreedingPermanent(a) => CompiledStep::SelectOwnBreedingPermanent {
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            optional: a.optional,
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            then: a
                .then
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                .collect(),
        },
        S::SelectReveal(a) => CompiledStep::SelectReveal {
            of: compile_player_ref(a.of),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
        },
        S::SelectRevealBuckets(a) => CompiledStep::SelectRevealBuckets {
            from: a.from.clone(),
            buckets: a
                .buckets
                .iter()
                .enumerate()
                .map(|(i, bucket)| CompiledRevealBucket {
                    bind_as: bucket.bind_as.clone(),
                    filter: bucket.filter.as_ref().map(|filter| {
                        compile_predicate(
                            filter,
                            &format!("{prefix}.buckets[{i}].filter"),
                            card_id,
                            errors,
                        )
                    }),
                    min: bucket.min.unwrap_or(0),
                    max: bucket.max.unwrap_or(1),
                })
                .collect(),
            no_duplicate_cards: a.no_duplicate_cards,
            prompt: a.prompt.clone(),
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
            material_of: a.material_of.as_ref().map(compile_binding_ref),
            filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            bind_as: a.bind_as.clone(),
            prompt: a.prompt.clone(),
            prompt_key: a.prompt_key.clone(),
            optional: a.optional,
            cost: a.cost,
            then: a
                .then
                .iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.then[{i}]"), card_id, errors))
                .collect(),
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
            max: compile_count_bound(&a.max, &format!("{prefix}.max"), card_id, errors),
            min: a.min,
            clamp_to_available: a.clamp_to_available,
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
        S::ScheduleDeletePlayedAtTurnEnd(a) => CompiledStep::ScheduleDeletePlayedAtTurnEnd {
            binding: a.binding.clone(),
            at_opponents_turn: matches!(a.at, crate::step::DeleteTurnBoundary::OpponentsTurn),
        },
        S::PlaceSelfAsDelayOption(_) => CompiledStep::PlaceSelfAsDelayOption,
        S::LinkToOwnDigimon(a) => {
            if !a.free {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.link_to_own_digimon.free"),
                    message: "link_to_own_digimon currently supports only free: true".to_string(),
                });
            }
            CompiledStep::LinkToOwnDigimon {
                optional: a.optional,
                free: a.free,
                filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
            }
        }
        S::LinkCardToSelf(a) => {
            if a.from.is_empty() {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.link_card_to_self.from"),
                    message: "link_card_to_self requires at least one source zone".to_string(),
                });
            }
            CompiledStep::LinkCardToSelf {
                from: a.from.clone(),
                filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
                to: a.to,
                cost: a.cost,
                optional: a.optional,
            }
        }
        S::ReduceLinkCost(a) => CompiledStep::ReduceLinkCost { amount: a.amount },
        S::LinkCards(a) => {
            use crate::compiled::{
                CompiledLinkCount, CompiledLinkSourceZone, CompiledLinkTo,
            };
            use crate::step::{LinkCardSourceZone, LinkCardsCost, LinkCardsCount, LinkCardsTo};

            if a.from.is_empty() {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.link_cards.from"),
                    message: "link_cards requires at least one source zone".to_string(),
                });
            }

            let from = a
                .from
                .iter()
                .map(|z| match z {
                    LinkCardSourceZone::Hand => CompiledLinkSourceZone::Hand,
                    LinkCardSourceZone::Trash => CompiledLinkSourceZone::Trash,
                    LinkCardSourceZone::SelfSources => CompiledLinkSourceZone::SelfSources,
                    LinkCardSourceZone::OwnDigimonSources => {
                        CompiledLinkSourceZone::OwnDigimonSources
                    }
                    LinkCardSourceZone::SelfOption => CompiledLinkSourceZone::SelfOption,
                })
                .collect();

            let to = match a.to {
                LinkCardsTo::SelfPermanent => CompiledLinkTo::SelfPermanent,
                LinkCardsTo::OwnDigimon => CompiledLinkTo::OwnDigimon,
            };

            let count = match a.count {
                LinkCardsCount::Exactly(n) => CompiledLinkCount::Exactly(n),
                LinkCardsCount::UpTo(n) => CompiledLinkCount::UpTo(n),
            };

            // `free` pays 0; `reduce: N` pays max(0, base - N). The cards this
            // step serves have a base link cost of 0 in this context, so both
            // resolve to 0. Threaded faithfully so a non-zero base cost extends
            // here later without a schema change.
            let cost = match a.cost {
                LinkCardsCost::Free => 0u8,
                LinkCardsCost::Reduce(n) => 0u8.saturating_sub(n),
            };

            CompiledStep::LinkCards {
                from,
                filter: compile_predicate(&a.filter, &format!("{prefix}.filter"), card_id, errors),
                to,
                count,
                cost,
                prompt: a.prompt.clone(),
            }
        }
        // OptionalStep is a newtype wrapping Vec<StepSpec> — access via .0
        S::Optional(o) => CompiledStep::Optional(
            o.0.iter()
                .enumerate()
                .map(|(i, s)| compile_step(s, &format!("{prefix}.optional[{i}]"), card_id, errors))
                .collect(),
        ),
        S::Battle(b) => CompiledStep::Battle {
            attacker: compile_binding_ref(&b.attacker),
            defender: compile_binding_ref(&b.defender),
        },
        S::MayAttackNow(a) => CompiledStep::MayAttackNow {
            attacker: compile_binding_ref(&a.attacker),
            targets: compile_attack_target_spec(a.targets),
            without_suspending: a.without_suspending,
            ignore_summoning_sickness: a.ignore_summoning_sickness,
            optional: a.optional,
            windowed: a.windowed,
            prompt: a.prompt.clone(),
            cost_upgrade: a.cost_upgrade.map(|u| CompiledAttackCostUpgrade {
                dp: u.dp.unwrap_or(0),
                security_attack: u.security_attack.unwrap_or(0),
            }),
        },
        S::MayDnaDigivolveNow(a) => CompiledStep::MayDnaDigivolveNow {
            anchor: compile_binding_ref(&a.anchor),
            partner_filter: compile_predicate(
                &a.partner_filter,
                &format!("{prefix}.partner_filter"),
                card_id,
                errors,
            ),
            target_filter: compile_predicate(
                &a.target_filter,
                &format!("{prefix}.target_filter"),
                card_id,
                errors,
            ),
            cost: a.cost,
            ignore_requirements: a.ignore_requirements,
            optional: a.optional,
            prompt: a.prompt.clone(),
        },
        S::ForceAttack(a) => CompiledStep::ForceAttack {
            attacker: compile_binding_ref(&a.attacker),
            targets: compile_attack_target_spec(a.targets),
            without_suspending: a.without_suspending,
            prompt: a.prompt.clone(),
            cost_upgrade: a.cost_upgrade.map(|u| CompiledAttackCostUpgrade {
                dp: u.dp.unwrap_or(0),
                security_attack: u.security_attack.unwrap_or(0),
            }),
        },
        S::RedirectAttackTarget(a) => CompiledStep::RedirectAttackTarget {
            new_target: a.new_target.as_ref().map(compile_binding_ref),
            player: a.player.map(compile_player_ref),
            targets: compile_attack_target_spec(a.targets),
            optional: a.optional,
            prompt: a.prompt.clone(),
        },
        S::CancelAttack(_) => CompiledStep::CancelAttack,
        S::RefundOpt(_) => CompiledStep::RefundOpt,
        S::OpenCounterWindow(_) => CompiledStep::OpenCounterWindow,
        S::RefireEffect(a) => {
            if a.timing == "on_play_or_when_digivolving" && a.optional {
                errors.push(ValidationError {
                    card_id: card_id.to_string(),
                    path: format!("{prefix}.refire_effect.optional"),
                    message: "refire_effect optional: true is not supported with timing: on_play_or_when_digivolving; put optionality on the target selection or containing clause".to_string(),
                });
            }
            CompiledStep::RefireEffect {
                source: compile_binding_ref(&a.source),
                timing: if matches!(
                    a.timing.as_str(),
                    "on_play" | "when_digivolving" | "on_play_or_when_digivolving"
                ) {
                    a.timing.clone()
                } else {
                    errors.push(ValidationError {
                        card_id: card_id.to_string(),
                        path: format!("{prefix}.refire_effect.timing"),
                        message: format!(
                            "refire_effect only supports timing: on_play, when_digivolving, or on_play_or_when_digivolving, got {}",
                            a.timing
                        ),
                    });
                    "when_digivolving".to_string()
                },
                optional: a.optional,
            }
        }
        S::EndAttack(enabled) => CompiledStep::EndAttack { enabled: *enabled },
        S::CancelReplacement(_) => CompiledStep::CancelReplacement,
        S::HandleReplacement(_) => CompiledStep::HandleReplacement,
        S::RedirectReplacement(r) => CompiledStep::RedirectReplacement {
            zone: compile_zone(r.zone),
        },
        S::SubstituteReplacement(s) => CompiledStep::SubstituteReplacement {
            subject: compile_binding_ref(&s.subject),
        },
        S::RawRust(r) => CompiledStep::RawRust {
            fn_name: r.fn_name.clone(),
            consumes: r.consumes.clone(),
            binds: r.binds.clone(),
        },
        S::ActivationCost(a) => {
            let kind = match (a.suspend_self, a.return_self_to_deck_bottom, a.trash_self) {
                (true, false, false) => {
                    Some(crate::compiled::CompiledActivationCostKind::SuspendSelf)
                }
                (false, true, false) => {
                    Some(crate::compiled::CompiledActivationCostKind::ReturnSelfToDeckBottom)
                }
                (false, false, true) => {
                    Some(crate::compiled::CompiledActivationCostKind::TrashSelf)
                }
                (false, false, false) => {
                    errors.push(ValidationError {
                        card_id: card_id.to_string(),
                        path: format!("{}.activation_cost", prefix),
                        message:
                            "activation_cost requires exactly one cost kind: suspend_self, return_self_to_deck_bottom, or trash_self"
                                .to_string(),
                    });
                    None
                }
                _ => {
                    errors.push(ValidationError {
                        card_id: card_id.to_string(),
                        path: format!("{}.activation_cost", prefix),
                        message:
                            "activation_cost: suspend_self, return_self_to_deck_bottom, and trash_self are mutually exclusive"
                                .to_string(),
                    });
                    None
                }
            };
            CompiledStep::ActivationCost {
                kind: kind.unwrap_or(crate::compiled::CompiledActivationCostKind::SuspendSelf),
            }
        }
        S::ArmDigivolveCostReducer(a) => CompiledStep::ArmDigivolveCostReducer {
            amount: a.amount,
            single_fire: a.single_fire,
            target_color: a.target_color.map(compile_color),
            suspend_cost: a.suspend_cost,
        },
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::{
        CompiledBindingRef, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause,
        CompiledPlayerRef, CompiledStep,
    };
    use crate::loader::load_dir_ok;
    use crate::CardSpec;
    use std::path::PathBuf;

    fn parse_spec(yaml: &str) -> CardSpec {
        serde_yml::from_str(yaml).expect("test YAML parses")
    }

    fn replacement_spec(effect_body: &str) -> CardSpec {
        parse_spec(&format!(
            r#"
card: TEST-001
name: "Replacement Test"
kind: option
color: [blue]
cost: 0
traits: []
effects:
  - kind: replacement
{effect_body}
"#
        ))
    }

    fn compile_error_text(spec: &CardSpec) -> String {
        compile(spec)
            .expect_err("spec should fail compilation")
            .into_iter()
            .map(|err| err.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn high_level_delay_replacement_body(timing_line: &str) -> String {
        format!(
            r#"    {timing_line}
    active_when:
      all:
        - subject_trait: Free
        - replacement_cause: opponent_effect
    cost:
      delay_self: true
    choose:
      from: hand
      card_filter:
        trait: Imperialdramon
      min: 1
      max: 1
    outcome: prevent
    then:
      - digivolve_without_cost:
          target: replacement_subject
          card: chosen"#
        )
    }

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

    #[test]
    fn high_level_delay_replacement_lowers_to_process_steps() {
        let body = high_level_delay_replacement_body("timing: when_would_be_deleted");
        let compiled = compile(&replacement_spec(&body)).expect("replacement compiles");
        let CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            trigger,
            process,
            ..
        }) = &compiled.effects[0]
        else {
            panic!("expected replacement clause");
        };

        assert_eq!(trigger, "when_would_be_deleted");
        assert!(
            matches!(
                process.as_slice(),
                [
                    CompiledStep::DeletePermanent {
                        target: CompiledBindingRef::Source
                    },
                    CompiledStep::SelectHand {
                        of: CompiledPlayerRef::You,
                        bind_as: Some(chosen),
                        optional: true,
                        ..
                    },
                    CompiledStep::EffectInitiatedDigivolve {
                        target: CompiledBindingRef::Named(target),
                        from_hand: CompiledBindingRef::Named(card),
                        cost: CompiledCostDelta::Free,
                        ignore_requirements: true,
                    },
                    CompiledStep::CancelReplacement
                ] if chosen == "chosen" && target == "replacement_subject" && card == "chosen"
            ),
            "unexpected replacement process: {process:#?}"
        );
    }

    #[test]
    fn replacement_requires_a_timing_or_trigger() {
        let body = high_level_delay_replacement_body("");
        let errors = compile_error_text(&replacement_spec(&body));

        assert!(
            errors.contains("requires exactly one of timing or trigger"),
            "unexpected errors:\n{errors}"
        );
    }

    #[test]
    fn replacement_rejects_both_timing_and_trigger() {
        let body = high_level_delay_replacement_body(
            "timing: when_would_be_deleted\n    trigger: when_would_be_deleted",
        );
        let errors = compile_error_text(&replacement_spec(&body));

        assert!(
            errors.contains("requires exactly one of timing or trigger"),
            "unexpected errors:\n{errors}"
        );
    }

    #[test]
    fn replacement_rejects_unknown_timing() {
        let body = high_level_delay_replacement_body("timing: when_would_be_smudged");
        let errors = compile_error_text(&replacement_spec(&body));

        assert!(
            errors.contains("unknown replacement timing"),
            "unexpected errors:\n{errors}"
        );
    }

    #[test]
    fn delay_rejects_both_clause_and_nested_active_when() {
        let mut spec = parse_spec(
            r#"
card: TEST-DELAY
name: "Delay Test"
kind: option
color: [yellow]
cost: 0
traits: []
effects:
  - kind: delay
    scope: face_up
    trigger: on_suspend
    process: []
"#,
        );
        let clause_active_when: crate::predicate::PredicateSpec =
            serde_yml::from_str("your_turn: true").unwrap();
        let nested_active_when: crate::predicate::PredicateSpec =
            serde_yml::from_str("event_card_name_contains: \"Arisa Kinosaki\"").unwrap();
        let crate::clause::ClauseSpec::Declarative(delay) = &mut spec.effects[0] else {
            panic!("expected delay declarative");
        };
        delay.active_when = Some(clause_active_when);
        delay.body.insert(
            "active_when".into(),
            serde_yml::to_value(nested_active_when).unwrap(),
        );
        let errors = compile_error_text(&spec);

        assert!(
            errors.contains("delay.active_when cannot be combined with clause active_when yet"),
            "unexpected errors:\n{errors}"
        );
    }
}
