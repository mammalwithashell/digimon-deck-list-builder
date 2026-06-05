//! Pack container: manifest + compiled cards.
//!
//! Pack format (bincode-serialized):
//!   pack_version: pack-level semver
//!   min_engine_version: minimum digimon-engine semver that can load this pack
//!   max_engine_version: optional upper bound for breaking schema changes
//!   required_raw_rust_fns: fn names the pack references; desktop rejects if missing
//!   pack_id: "BT17", "core", etc. for cache-dir segregation
//!   cards: every compiled card in the pack

use crate::compiled::CompiledCard;
use crate::compiled::*;
use crate::raw_rust_registry::RawRustRegistry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PACK_FORMAT_VERSION: &str = "0.4.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardPack {
    pub manifest: PackManifest,
    pub cards: Vec<CompiledCard>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub pack_id: String,
    pub pack_version: String,
    pub min_engine_version: String,
    pub max_engine_version: Option<String>,
    pub required_raw_rust_fns: Vec<String>,
}

impl CardPack {
    pub fn new(pack_id: impl Into<String>, cards: Vec<CompiledCard>) -> Self {
        Self::from_compiled_cards(pack_id, cards)
    }

    pub fn from_compiled_cards(pack_id: impl Into<String>, cards: Vec<CompiledCard>) -> Self {
        let required_raw_rust_fns = required_raw_rust_fns_for_cards(&cards);
        Self::new_with_required_raw_rust_fns(pack_id, cards, required_raw_rust_fns)
    }

    pub fn new_with_required_raw_rust_fns<I, S>(
        pack_id: impl Into<String>,
        cards: Vec<CompiledCard>,
        required_raw_rust_fns: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            manifest: PackManifest {
                pack_id: pack_id.into(),
                pack_version: PACK_FORMAT_VERSION.to_string(),
                min_engine_version: "0.1.0".into(),
                max_engine_version: None,
                required_raw_rust_fns: normalize_raw_rust_names(required_raw_rust_fns),
            },
            cards,
        }
    }

    /// Serialize to bincode bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("bincode serialize failed: {e}"))
    }

    /// Deserialize from bincode bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes).map_err(|e| format!("bincode deserialize failed: {e}"))
    }
}

pub fn missing_required_raw_rust_fns(
    manifest: &PackManifest,
    raw_rust: &dyn RawRustRegistry,
) -> Vec<String> {
    manifest
        .required_raw_rust_fns
        .iter()
        .filter(|name| !raw_rust.contains_fn(name))
        .cloned()
        .collect()
}

pub fn required_raw_rust_fns_for_cards(cards: &[CompiledCard]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for card in cards {
        collect_card_raw_rust_fns(card, &mut names);
    }
    names.into_iter().collect()
}

fn normalize_raw_rust_names<I, S>(names: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    names
        .into_iter()
        .map(Into::into)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_card_raw_rust_fns(card: &CompiledCard, names: &mut BTreeSet<String>) {
    collect_optional_predicate_raw_rust_fns(card.use_requirement.as_ref(), names);
    if let Some(dual) = &card.dual {
        collect_optional_predicate_raw_rust_fns(dual.option.use_requirement.as_deref(), names);
    }
    for alt_path in &card.alt_paths {
        collect_alt_path_raw_rust_fns(alt_path, names);
    }
    for clause in &card.effects {
        collect_clause_raw_rust_fns(clause, names);
    }
}

fn collect_alt_path_raw_rust_fns(alt_path: &CompiledAltPath, names: &mut BTreeSet<String>) {
    if let Some(from) = &alt_path.from {
        collect_predicate_raw_rust_fns(from, names);
    }
    for material in &alt_path.materials {
        collect_predicate_raw_rust_fns(&material.filter, names);
    }
    if let Some(CompiledCost::Formula(formula)) = &alt_path.cost {
        collect_formula_raw_rust_fns(formula, names);
    }
    for step in &alt_path.extra_cost {
        collect_step_raw_rust_fns(step, names);
    }
    for step in &alt_path.on_burst_turn_end {
        collect_step_raw_rust_fns(step, names);
    }
}

fn collect_clause_raw_rust_fns(clause: &CompiledClause, names: &mut BTreeSet<String>) {
    match clause {
        CompiledClause::Triggered(triggered) => {
            if let Some(active_when) = &triggered.active_when {
                collect_predicate_raw_rust_fns(active_when, names);
            }
            if let Some(condition) = &triggered.condition {
                collect_predicate_raw_rust_fns(condition, names);
            }
            for step in &triggered.process {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledClause::Declarative(declarative) => {
            collect_declarative_raw_rust_fns(declarative, names);
        }
    }
}

fn collect_declarative_raw_rust_fns(
    declarative: &CompiledDeclarativeClause,
    names: &mut BTreeSet<String>,
) {
    match declarative {
        CompiledDeclarativeClause::Aura {
            active_when,
            target,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            collect_predicate_raw_rust_fns(target, names);
        }
        CompiledDeclarativeClause::CostReduction {
            active_when,
            when_any_ally_played,
            condition,
            amount_fn,
            pay_cost,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            collect_optional_predicate_raw_rust_fns(when_any_ally_played.as_ref(), names);
            collect_optional_predicate_raw_rust_fns(condition.as_ref(), names);
            if let Some(formula) = amount_fn {
                collect_formula_raw_rust_fns(formula, names);
            }
            for step in pay_cost {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledDeclarativeClause::Replacement {
            active_when,
            process,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            for step in process {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledDeclarativeClause::Partition {
            active_when,
            sources,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            for source in sources {
                collect_predicate_raw_rust_fns(source, names);
            }
        }
        CompiledDeclarativeClause::GrantKeyword { active_when, .. } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
        }
        CompiledDeclarativeClause::Delay {
            active_when,
            process,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            for step in process {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledDeclarativeClause::LinkRequirement { filter, .. } => {
            collect_predicate_raw_rust_fns(filter, names);
        }
        CompiledDeclarativeClause::FloodGate {
            active_when,
            target,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            if let Some(target) = target {
                collect_predicate_raw_rust_fns(target, names);
            }
        }
        CompiledDeclarativeClause::AltPathRegistration {
            active_when,
            applies_to,
            registers,
            ..
        } => {
            collect_optional_predicate_raw_rust_fns(active_when.as_ref(), names);
            collect_optional_predicate_raw_rust_fns(applies_to.as_ref(), names);
            collect_alt_path_raw_rust_fns(registers, names);
        }
        CompiledDeclarativeClause::RawRust { fn_name, .. } => {
            names.insert(fn_name.clone());
        }
        CompiledDeclarativeClause::AceOverflow { .. } => {}
    }
}

fn collect_step_raw_rust_fns(step: &CompiledStep, names: &mut BTreeSet<String>) {
    match step {
        CompiledStep::AddDpModifier { value, .. } => {
            collect_modifier_value_raw_rust_fns(value, names);
        }
        CompiledStep::AddModifier { target, value, .. } => {
            if let CompiledModifierTarget::Filter(filter) = target {
                collect_predicate_raw_rust_fns(filter, names);
            }
            collect_modifier_value_raw_rust_fns(value, names);
        }
        CompiledStep::AddPlayerModifier { .. } => {}
        CompiledStep::SelectOwnPermanent { filter, .. }
        | CompiledStep::SelectOpponentPermanent { filter, .. }
        | CompiledStep::SelectHand { filter, .. }
        | CompiledStep::UseOptionFromHand { filter, .. }
        | CompiledStep::SelectTrash { filter, .. }
        | CompiledStep::SelectMaterial { filter, .. }
        | CompiledStep::SelectReveal { filter, .. }
        | CompiledStep::SelectSecurity { filter, .. }
        | CompiledStep::SelectUnionZone { filter, .. }
        | CompiledStep::SelectCountCappedMulti { filter, .. }
        | CompiledStep::LinkToOwnDigimon { filter, .. } => {
            collect_predicate_raw_rust_fns(filter, names);
        }
        CompiledStep::SelectRevealBuckets { buckets, .. } => {
            for bucket in buckets {
                if let Some(filter) = &bucket.filter {
                    collect_predicate_raw_rust_fns(filter, names);
                }
            }
        }
        CompiledStep::If {
            condition,
            then,
            else_branch,
        } => {
            collect_predicate_raw_rust_fns(condition, names);
            for step in then {
                collect_step_raw_rust_fns(step, names);
            }
            for step in else_branch {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledStep::ForEach { over, body, .. } => {
            collect_predicate_raw_rust_fns(over, names);
            for step in body {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. }
        | CompiledStep::AsSelectingPlayer { body, .. } => {
            for step in body {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledStep::Optional(body) => {
            for step in body {
                collect_step_raw_rust_fns(step, names);
            }
        }
        CompiledStep::RawRust { fn_name, .. } => {
            names.insert(fn_name.clone());
        }
        CompiledStep::DeDigivolve { amount_fn, .. } => {
            if let Some(formula) = amount_fn {
                collect_formula_raw_rust_fns(formula, names);
            }
        }
        _ => {}
    }
}

fn collect_modifier_value_raw_rust_fns(
    value: &CompiledModifierValue,
    names: &mut BTreeSet<String>,
) {
    if let CompiledModifierValue::Formula(formula) = value {
        collect_formula_raw_rust_fns(formula, names);
    }
}

fn collect_optional_predicate_raw_rust_fns(
    predicate: Option<&CompiledPredicate>,
    names: &mut BTreeSet<String>,
) {
    if let Some(predicate) = predicate {
        collect_predicate_raw_rust_fns(predicate, names);
    }
}

fn collect_predicate_raw_rust_fns(predicate: &CompiledPredicate, names: &mut BTreeSet<String>) {
    for dp in [
        &predicate.level_lte,
        &predicate.level_gte,
        &predicate.play_cost_lte,
        &predicate.dp_eq,
        &predicate.dp_lte,
        &predicate.dp_gte,
        &predicate.stack_size_lte,
        &predicate.stack_size_gte,
        &predicate.materials_count_lte,
        &predicate.materials_count_gte,
        &predicate.memory_lte,
        &predicate.memory_gte,
        &predicate.security_count_lte,
        &predicate.security_count_gte,
        &predicate.face_up_security_count_lte,
        &predicate.face_up_security_count_gte,
    ]
    .into_iter()
    .flatten()
    {
        if let CompiledDpConstraint::Formula(formula) = dp {
            collect_formula_raw_rust_fns(formula, names);
        }
    }
    if let Some(has_inherited) = &predicate.has_inherited {
        collect_predicate_raw_rust_fns(has_inherited, names);
    }
    for aggregate in [&predicate.count_lte, &predicate.count_gte]
        .into_iter()
        .flatten()
    {
        if let CompiledDpConstraint::Formula(formula) = &aggregate.n {
            collect_formula_raw_rust_fns(formula, names);
        }
        collect_predicate_raw_rust_fns(&aggregate.filter, names);
    }
    for existential in [
        &predicate.any_permanent,
        &predicate.any_field_permanent,
        &predicate.no_permanent,
        &predicate.all_permanents,
    ]
    .into_iter()
    .flatten()
    {
        collect_predicate_raw_rust_fns(&existential.predicate, names);
    }
    for child in predicate
        .all_of
        .iter()
        .chain(&predicate.any_of)
        .chain(&predicate.none_of)
    {
        collect_predicate_raw_rust_fns(child, names);
    }
    if let Some(not) = &predicate.not {
        collect_predicate_raw_rust_fns(not, names);
    }
}

fn collect_formula_raw_rust_fns(formula: &CompiledFormula, names: &mut BTreeSet<String>) {
    match formula {
        CompiledFormula::RawRust(name) => {
            names.insert(name.clone());
        }
        CompiledFormula::FloorDiv(args)
        | CompiledFormula::Max(args)
        | CompiledFormula::Min(args) => {
            for arg in args {
                collect_formula_raw_rust_fns(arg, names);
            }
        }
        CompiledFormula::BasePerDelta { per, .. } => {
            collect_per_selector_raw_rust_fns(per, names);
        }
        CompiledFormula::SourceStackCount { filter, .. }
        | CompiledFormula::SourceStackDpSum { filter, .. } => {
            if let Some(filter) = filter {
                collect_predicate_raw_rust_fns(filter, names);
            }
        }
        CompiledFormula::Literal(_)
        | CompiledFormula::SourceDp
        | CompiledFormula::SourceMaterialCount
        | CompiledFormula::SourceColorCount
        | CompiledFormula::Aggregate(_)
        | CompiledFormula::AggregateScoped { .. }
        | CompiledFormula::BindingDp(_)
        | CompiledFormula::BindingPlayCost(_)
        | CompiledFormula::BindingValue(_) => {}
    }
}

fn collect_per_selector_raw_rust_fns(sel: &CompiledPerSelector, names: &mut BTreeSet<String>) {
    match sel {
        CompiledPerSelector::FilteredCardCountInZoneScoped { filter, .. } => {
            collect_predicate_raw_rust_fns(filter, names);
        }
        CompiledPerSelector::DistinctColorsCountScoped {
            filter: Some(filter),
            ..
        } => {
            collect_predicate_raw_rust_fns(filter, names);
        }
        CompiledPerSelector::SourceStackCountFiltered {
            filter: Some(filter),
        } => {
            collect_predicate_raw_rust_fns(filter, names);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::*;
    use crate::raw_rust_registry::StubRegistry;

    #[test]
    fn empty_pack_round_trips() {
        let pack = CardPack::new("test", vec![]);
        assert_eq!(pack.manifest.pack_version, PACK_FORMAT_VERSION);
        let bytes = pack.to_bytes().unwrap();
        let reparsed = CardPack::from_bytes(&bytes).unwrap();
        assert_eq!(reparsed.manifest.pack_id, "test");
        assert_eq!(reparsed.cards.len(), 0);
    }

    #[test]
    fn pack_with_one_card_round_trips() {
        let card = CompiledCard {
            card: "X-1".into(),
            name: "Test".into(),
            kind: CompiledCardKind::Option,
            level: None,
            color: vec![CompiledColor::Red],
            cost: Some(0),
            dp: None,
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            digixros_aliases: vec![],
            also_treated_as: vec![],
            dual: None,
            use_requirement: None,
            alt_paths: vec![],
            effects: vec![],
        };
        let pack = CardPack::new("test", vec![card.clone()]);
        let bytes = pack.to_bytes().unwrap();
        let reparsed = CardPack::from_bytes(&bytes).unwrap();
        assert_eq!(reparsed.cards, vec![card]);
    }

    #[test]
    fn pack_metadata_collects_required_raw_rust_fns() {
        let card = CompiledCard {
            card: "X-RAW".into(),
            name: "Raw".into(),
            kind: CompiledCardKind::Digimon,
            level: Some(3),
            color: vec![CompiledColor::Red],
            cost: Some(3),
            dp: Some(2000),
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            digixros_aliases: vec![],
            also_treated_as: vec![],
            dual: Some(CompiledDual {
                digimon: CompiledDualDigimon {
                    level: 3,
                    dp: 2000,
                    colors: vec![CompiledColor::Red],
                    traits: vec![],
                    effect_text: String::new(),
                    inherited_text: String::new(),
                },
                option: CompiledDualOption {
                    use_cost: 3,
                    colors: vec![CompiledColor::Red],
                    effect_text: String::new(),
                    security_text: String::new(),
                    keywords: vec![],
                    use_requirement: Some(Box::new(CompiledPredicate {
                        any_field_permanent: Some(Box::new(CompiledExistential {
                            of: CompiledPlayerRef::You,
                            predicate: CompiledPredicate {
                                dp_gte: Some(CompiledDpConstraint::Formula(
                                    CompiledFormula::RawRust(
                                        "dual_use_req_nested_formula_fn".into(),
                                    ),
                                )),
                                ..Default::default()
                            },
                        })),
                        ..Default::default()
                    })),
                },
            }),
            use_requirement: Some(CompiledPredicate {
                dp_lte: Some(CompiledDpConstraint::Formula(CompiledFormula::RawRust(
                    "use_req_formula_fn".into(),
                ))),
                ..Default::default()
            }),
            alt_paths: vec![],
            effects: vec![
                CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
                    fn_name: "whole_clause_fn".into(),
                    triggers: vec![],
                    scope: CompiledScope::FaceUp,
                    summary: None,
                    summary_key: None,
                }),
                CompiledClause::Triggered(CompiledTriggeredClause {
                    when: vec![CompiledTiming::OnPlay],
                    scope: CompiledScope::FaceUp,
                    active_when: None,
                    condition: None,
                    optional: false,
                    once_per_turn: false,
                    max_per_turn: None,
                    process: vec![CompiledStep::RawRust {
                        fn_name: "step_fn".into(),
                        consumes: vec![],
                        binds: vec![],
                    }],
                    summary: None,
                    summary_key: None,
                }),
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                    scope: CompiledScope::FaceUp,
                    active_when: None,
                    reduction_timing: None,
                    when_playing_this: true,
                    when_any_ally_played: None,
                    when_any_ally_digivolves_into: None,
                    condition: None,
                    optional: false,
                    once_per_turn: false,
                    amount: None,
                    amount_fn: Some(CompiledFormula::Max(vec![
                        CompiledFormula::RawRust("formula_fn".into()),
                        CompiledFormula::BasePerDelta {
                            base: 0,
                            per: CompiledPerSelector::FilteredCardCountInZoneScoped {
                                zone: CompiledZone::Trash,
                                of: CompiledPlayerRef::Any,
                                filter: Box::new(CompiledPredicate {
                                    dp_lte: Some(CompiledDpConstraint::Formula(
                                        CompiledFormula::RawRust(
                                            "filtered_count_formula_fn".into(),
                                        ),
                                    )),
                                    ..Default::default()
                                }),
                            },
                            delta: 1,
                        },
                    ])),
                    pay_cost: vec![],
                    summary: None,
                    summary_key: None,
                }),
            ],
        };

        let pack = CardPack::from_compiled_cards("raw-pack", vec![card]);
        assert_eq!(
            pack.manifest.required_raw_rust_fns,
            vec![
                "dual_use_req_nested_formula_fn",
                "filtered_count_formula_fn",
                "formula_fn",
                "step_fn",
                "use_req_formula_fn",
                "whole_clause_fn"
            ]
        );
    }

    #[test]
    fn missing_required_raw_rust_fns_reports_manifest_gaps() {
        let pack = CardPack::new_with_required_raw_rust_fns(
            "raw-pack",
            vec![],
            ["present_fn", "missing_fn"],
        );
        let registry = StubRegistry::with(["present_fn"]);
        assert_eq!(
            missing_required_raw_rust_fns(&pack.manifest, &registry),
            vec!["missing_fn"]
        );
    }
}
