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
use crate::formula::{AggregateSelector, FormulaSpec};
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
    pub level_eq_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_matches_aggregate: Option<LevelAggregatePredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_is: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_only: Option<Vec<ColorSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_matches_any_field_digimon: Option<PlayerRefSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_matches_binding: Option<String>,
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
    /// Card-subject leaf: true when NO battle-area Digimon belonging to the
    /// scoped player shares the candidate card's name. Models the printed
    /// "This effect can't play cards with the same names as any of your
    /// Digimon" exclusion on the Jesmon family (BT23-013) — applied as a
    /// filter on a `select_union_zone` (hand+trash) play candidate set so
    /// the in-play names are masked out, never auto-picked.
    /// G-UNION-HAND-TRASH-NAME-EXCLUSION (Phase 2 Track J Task S2.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_not_shared_by_field_digimon: Option<PlayerRefSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_number_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_cost_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_cost_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_digivolve_from_source: Option<bool>,

    // Leaf — permanent-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_eq: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_inherited: Option<Box<PredicateSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_suspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unsuspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_keyword: Option<String>,
    /// Phase 2 Track F (G-DSL-HAS-ON-DELETION-EFFECT) — true when the
    /// permanent's top card (or any card in its digivolution stack) has a
    /// triggered effect with `EffectTiming::OnDeletion` either via a
    /// compiled DSL clause or a hand-written `CardEffect` impl. Used by
    /// EX1-021 MetalGarurumon's "[When Attacking] return 1 opponent
    /// Digimon **that has an [On Deletion] effect** to the bottom of
    /// deck." DCGO `permanent.HasOnDeletionEffect`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_on_deletion_effect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_color_count_gte: Option<u8>,

    // Leaf — zone / owner
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub zone: Vec<Zone>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "of")]
    pub owner: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of_permanent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_in_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_owner: Option<BindingOwnerPredicate>,

    // Leaf — source-relative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_tamer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_unsuspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_permanent_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_digivolution_contains_name: Option<String>,

    // Leaf — global / observer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_count_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_count_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent_security_count_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent_security_count_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_turn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents_turn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_turns: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_hatch: Option<PlayerRef>,
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
    pub event_target_owner: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_permanent_is_source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_is_effect_initiated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_is_player: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_was_self: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_target_change_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_cause: Option<EventCauseSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_permanent_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trashed_source_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trashed_source_card_id_is: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_exists: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "binding_is_present")]
    pub binding_present: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "binding_is_none")]
    pub binding_absent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_suspended_any_own_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "any_returned_card")]
    pub effect_returned_any_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_deleted_any_own_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_deleted_any_opponent_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_played_any_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_digivolved_any_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_added_any_card_to_hand: Option<bool>,

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

    /// BeforePayCost target card predicate. When present, the inner
    /// predicate is evaluated against the card whose cost is currently
    /// being computed (`cost_target_card` on the effect read context),
    /// treated as a `Card` subject. Fails when no cost target is active
    /// (i.e., outside `BeforePayCost` cost-calc dispatch). Use the full
    /// card-shape vocabulary inside: `trait_has`, `color_is`,
    /// `name_contains`, `level_eq`/`_lte`/`_gte`, `kind`, etc.
    ///
    /// Example: a cost-reduction clause that fires only when the card
    /// being digivolved into has the [Free] trait:
    ///
    /// ```yaml
    /// active_when:
    ///   your_turn: true
    ///   cost_target: { trait_has: Free }
    /// ```
    ///
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_target: Option<Box<PredicateSpec>>,

    /// True when the effect's `source_permanent` is the (or one of the)
    /// permanent(s) being digivolved by the action whose cost is being
    /// computed. Use to gate "When THIS Digimon would digivolve into …"
    /// printed semantics so the observer / cost reducer only fires when
    /// its carrier permanent is actually the digivolution target. Single
    /// entry for normal digivolve; both DNA materials for DNA digivolve.
    /// Always false outside cost-calc dispatch and for effects whose
    /// `source_permanent` is `None`.
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_cost_target_permanent: Option<bool>,

    /// Captures unrecognized fields for controlled extension. Validator
    /// (Task 12) checks this for typos in inline predicate positions.
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: IndexMap<String, serde_yml::Value>,
}

impl PredicateSpec {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingOwnerPredicate {
    pub binding: String,
    pub of: PlayerRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum DpConstraint {
    Literal(i32),
    Formula(FormulaSpec),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum DpConstraintDeserialize {
    Literal(i32),
    WrappedFormula { formula: FormulaSpec },
    Formula(FormulaSpec),
}

impl<'de> Deserialize<'de> for DpConstraint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = DpConstraintDeserialize::deserialize(deserializer)?;
        Ok(match helper {
            DpConstraintDeserialize::Literal(n) => Self::Literal(n),
            DpConstraintDeserialize::WrappedFormula { formula }
            | DpConstraintDeserialize::Formula(formula) => Self::Formula(formula),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementCauseSpec {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
    Overclock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventCauseSpec {
    BattleDeletion,
    EffectDeletion,
    OwnEffect,
    OpponentEffect,
    Overclock,
    Return,
    DeckBottom,
    SecurityPlacement,
    SecurityRemoval,
    Cost,
    Rule,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LevelAggregatePredicate {
    pub selector: AggregateSelector,
    #[serde(default = "default_level_aggregate_of")]
    pub of: PlayerRef,
}

fn default_level_aggregate_of() -> PlayerRef {
    PlayerRef::You
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CountAggregate {
    pub filter: Box<PredicateSpec>,
    pub n: DpConstraint,
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
