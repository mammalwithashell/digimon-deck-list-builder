//! Filter / predicate tree. Spec §3.8.
//!
//! `PredicateSpec` is a flat struct where every leaf predicate is an
//! `Option<_>` field and compound forms (`all_of` / `any_of` / `none_of`
//! / `not`) are sibling fields. At evaluation time (Phase 2) every
//! present field contributes an AND-joined constraint.
//!
//! NOTE: this struct deliberately does NOT set `deny_unknown_fields`
//! because it is flattened into several call-sites (MaterialSpec, field
//! predicates). Typos in leaf-predicate fields are silently dropped at
//! parse time; the semantic validator (Task 12) must re-check.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::common::PlayerRef;
use crate::formula::FormulaSpec;
use crate::spec::{CardKind, ColorSpec};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct PredicateSpec {
    // Leaf — card/permanent identity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<CardKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_eq: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_lte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_gte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_is: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_only: Option<Vec<ColorSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_matches_any_field_digimon: Option<PlayerRefSelector>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "trait",
        alias = "subject_trait"
    )]
    pub trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_number_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_cost_lte: Option<i32>,

    // Leaf — permanent-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_eq: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size_lte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size_gte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_lte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_gte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_inherited: Option<Box<PredicateSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_suspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unsuspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_keyword: Option<String>,

    // Leaf — zone / owner
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub zone: Vec<Zone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of_permanent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_in_binding: Option<String>,

    // Leaf — source-relative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_tamer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_permanent_trait_has: Option<String>,

    // Leaf — global / observer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_lte: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_gte: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_count_lte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_count_gte: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_turn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents_turn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_turns: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_breeding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_field: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dna_origin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_kind: Option<CardKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_cause: Option<ReplacementCauseSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_source_is_opponent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_subject_is_mine: Option<bool>,

    // Binding comparisons
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Vec<serde_json::Value>>")]
    pub equals: Option<Vec<serde_yml::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Vec<serde_json::Value>>")]
    pub not_equals: Option<Vec<serde_yml::Value>>,

    // Count aggregates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_lte: Option<CountAggregate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_gte: Option<CountAggregate>,

    // Existential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_permanent: Option<Box<ExistentialPredicate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_field_permanent: Option<Box<ExistentialPredicate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_permanent: Option<Box<ExistentialPredicate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_permanents: Option<Box<ExistentialPredicate>>,

    // Compound
    #[serde(skip_serializing_if = "Vec::is_empty", alias = "all")]
    pub all_of: Vec<PredicateSpec>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<PredicateSpec>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub none_of: Vec<PredicateSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<PredicateSpec>>,

    // Misc contextual predicates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_alt_path: Option<String>,

    /// Captures unrecognized fields for controlled extension. Validator
    /// (Task 12) checks this for typos in inline predicate positions.
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: IndexMap<String, serde_yml::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum DpConstraint {
    Literal(i32),
    Formula(FormulaSpec),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementCauseSpec {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum PlayerRefSelector {
    Player(PlayerRef),
    Scoped { of: PlayerRef },
}

impl PlayerRefSelector {
    pub fn player(self) -> PlayerRef {
        match self {
            Self::Player(player) => player,
            Self::Scoped { of } => of,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CountAggregate {
    pub filter: Box<PredicateSpec>,
    pub n: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct ExistentialPredicate {
    pub of: PlayerRef,
    #[serde(flatten)]
    pub predicate: PredicateSpec,
}

impl Default for ExistentialPredicate {
    fn default() -> Self {
        Self {
            of: PlayerRef::You,
            predicate: PredicateSpec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    Deck,
    Trash,
    BattleArea,
    Security,
    Breeding,
    Reveal,
    #[serde(rename = "digi_egg_deck")]
    DigiEggDeck,
    Material,
}
