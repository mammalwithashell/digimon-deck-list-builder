//! Formula primitives for scalar computations in predicates and clauses.
//! Spec §3.10.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::common::PlayerRef;
use crate::predicate::{PredicateSpec, Zone};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum FormulaSpec {
    Literal(i32),
    BasePerDelta {
        base: i32,
        per: PerSelector,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bucket: Option<u32>,
        delta: i32,
    },
    BindingDp {
        binding_dp: String,
    },
    BindingPlayCost {
        binding_play_cost: String,
    },
    BindingValue {
        binding_value: String,
    },
    /// Effective DP of the effect's `source_permanent` (the carrier of
    /// the running effect). YAML form: `{ source_dp: {} }`. Unlike
    /// `binding_dp`, which reads a named `bind_as` binding, this reads
    /// `ctx.source_permanent` directly. Used by P-182's [When
    /// Digivolving] "delete 1 opp Digimon with as much or less DP as
    /// this Digimon". G-FORMULA-SOURCE-DP.
    SourceDp {
        source_dp: SourceDpSpec,
    },
    /// Number of digivolution *cards* (materials beneath the top card) on
    /// the effect's `source_permanent`. YAML form: `{ source_material_count:
    /// {} }`. Sibling of `source_dp`. Used by AD1-025's [On Play][When
    /// Digivolving] "return all opponent Digimon with as many or fewer
    /// digivolution cards as this Digimon" — the candidate's
    /// `materials_count_lte` is compared against this. G-FORMULA-SOURCE-MATERIAL-COUNT.
    SourceMaterialCount {
        source_material_count: SourceDpSpec,
    },
    /// Level of the current trigger's EVENT card (the card that caused the
    /// observed event — e.g. the just-played Digimon for an
    /// `on_any_digimon_played` observer). YAML form:
    /// `{ event_target_level: {} }`. Card text shape: "you may play 1
    /// purple Digimon card with a level LESS THAN OR EQUAL TO IT from your
    /// trash" (EX5-060 Dragomon clause 2 — DCGO reads
    /// `permanent.LevelJustAfterPlayed`). Evaluates to 0 when there is no
    /// trigger context or the event card has no level (filters comparing
    /// against it then match nothing). G-EVENT-PLAYED-LEVEL-FORMULA.
    EventTargetLevel {
        event_target_level: SourceDpSpec,
    },
    /// Number of distinct colors represented by source cards beneath the effect
    /// carrier's top card. YAML form: `{ source_color_count: {} }`.
    /// This is source-relative, unlike `digivolution_color_count`, whose target
    /// is the permanent currently being measured.
    SourceColorCount {
        source_color_count: SourceDpSpec,
    },
    /// Number of source cards beneath a target permanent's top card, optionally
    /// filtered by card predicates. YAML form:
    /// `{ source_stack_count: { target: source, filter: { level_eq: 6 } } }`.
    SourceStackCount {
        source_stack_count: SourceStackDpSumSpec,
    },
    SourceStackDpSum {
        source_stack_dp_sum: SourceStackDpSumSpec,
    },
    Compound(CompoundFormula),
}

/// Empty payload marker for source-relative scalar formulas. Exists so the
/// untagged `FormulaSpec` variants have a distinguishing map key at
/// deserialization. YAML: `{ source_dp: {} }`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceDpSpec {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceStackDpSumSpec {
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Box<PredicateSpec>>,
}

#[derive(Debug, Clone, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompoundFormula {
    FloorDiv(Vec<FormulaSpec>),
    Max(Vec<FormulaSpec>),
    Min(Vec<FormulaSpec>),
    /// Left-associative subtraction: `subtract: [a, b]` → `a - b`. YAML:
    /// `{ subtract: [ {..}, {..} ] }`. G-DSL-FORMULA-SUBTRACT.
    Subtract(Vec<FormulaSpec>),
    Aggregate(AggregateSelector),
    AggregateScoped(AggregateFormulaSpec),
    RawRust(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AggregateFormulaSpec {
    pub selector: AggregateSelector,
    #[serde(default = "default_aggregate_scope")]
    pub scope: PlayerRef,
}

fn default_aggregate_scope() -> PlayerRef {
    PlayerRef::You
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CompoundFormulaDeserialize {
    FloorDiv(Vec<FormulaSpec>),
    Max(Vec<FormulaSpec>),
    Min(Vec<FormulaSpec>),
    Subtract(Vec<FormulaSpec>),
    Aggregate(AggregateFormulaPayload),
    AggregateScoped(AggregateFormulaSpec),
    RawRust(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum AggregateFormulaPayload {
    Legacy(AggregateSelector),
    Scoped(AggregateFormulaSpec),
}

impl<'de> Deserialize<'de> for CompoundFormula {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = CompoundFormulaDeserialize::deserialize(deserializer)?;
        Ok(match helper {
            CompoundFormulaDeserialize::FloorDiv(v) => Self::FloorDiv(v),
            CompoundFormulaDeserialize::Max(v) => Self::Max(v),
            CompoundFormulaDeserialize::Min(v) => Self::Min(v),
            CompoundFormulaDeserialize::Subtract(v) => Self::Subtract(v),
            CompoundFormulaDeserialize::Aggregate(AggregateFormulaPayload::Legacy(selector)) => {
                Self::Aggregate(selector)
            }
            CompoundFormulaDeserialize::Aggregate(AggregateFormulaPayload::Scoped(spec)) => {
                Self::AggregateScoped(spec)
            }
            CompoundFormulaDeserialize::AggregateScoped(spec) => Self::AggregateScoped(spec),
            CompoundFormulaDeserialize::RawRust(s) => Self::RawRust(s),
        })
    }
}

impl Serialize for CompoundFormula {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::FloorDiv(v) => map.serialize_entry("floor_div", v)?,
            Self::Max(v) => map.serialize_entry("max", v)?,
            Self::Min(v) => map.serialize_entry("min", v)?,
            Self::Subtract(v) => map.serialize_entry("subtract", v)?,
            Self::Aggregate(selector) => map.serialize_entry("aggregate", selector)?,
            Self::AggregateScoped(spec) => map.serialize_entry("aggregate", spec)?,
            Self::RawRust(name) => map.serialize_entry("raw_rust", name)?,
        }
        map.end()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CardCountInZoneSpec {
    pub zone: Zone,
    pub of: PlayerRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Box<crate::predicate::PredicateSpec>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    SuspendedCount {
        of: PlayerRef,
        /// When true, the effect's own source permanent is excluded from the
        /// count — DCGO `permanent != card.PermanentOfThisCard()`, used by
        /// EX8-074's "for each *other* suspended Digimon". Omitted → false.
        /// Only suspended *Digimon* are counted regardless of this flag.
        #[serde(default)]
        exclude_source: bool,
    },
    DigivolutionColorCount,
    SourceColorCount,
    SameLevelPairsInSources,
    SharedTrashCount {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bucket: Option<u32>,
    },
    CardCountInZone(CardCountInZoneSpec),
    DistinctColorsCount(CardCountInZoneSpec),
    /// Count of the effect carrier's *own* digivolution sources (the cards
    /// beneath its top card) that match `filter`. YAML form:
    /// `source_stack_count: { filter: { any_of: [...] } }`. Composes inside a
    /// `base_per_delta` so a card can scale a numeric (e.g. a DP cap) by the
    /// number of its sources matching a trait — EX3-014 Dorbickmon's "for each
    /// card with [Dragon]/[saur]/[Ceratopsian] in this Digimon's digivolution
    /// cards, add 2000 to the maximum DP". Unlike the top-level `source_stack_count`
    /// FormulaSpec (which yields a raw count and cannot be offset/scaled), this
    /// is a `per` selector. Sources are always those of `ctx.source_permanent`.
    /// G-DSL-PER-SOURCE-STACK-COUNT-FILTERED.
    SourceStackCount(SourceStackCountSpec),
    /// A player's memory-gauge value as a scalar count, so a `base_per_delta`
    /// can scale a numeric by memory. `of: you` reads the controller's signed
    /// memory (the gauge when it is their turn, negated otherwise); `of:
    /// opponent` reads the opponent's, clamped at 0 (DCGO `Math.Max(0,
    /// Enemy.MemoryForPlayer)`). YAML: `per: { player_memory: { of: opponent } }`.
    /// Drives BT25-086 Dan Yuki's "[for each memory your opponent has]"
    /// +1000-DP grant: `base_per_delta { base: 0, per: { player_memory: { of:
    /// opponent } }, delta: 1000 }`. G-DSL-FORMULA-PLAYER-MEMORY (driver
    /// BT25-086 / G-DSL-FORMULA-OPPONENT-MEMORY).
    PlayerMemory { of: PlayerRef },
    /// Total number of **link cards** across every one of `of`'s battle-area
    /// Digimon — `Σ over of.battle_area Digimon of permanent.linked_cards.len()`.
    /// YAML form: `{ own_link_card_count: { of: you } }`. Drives BT25-075
    /// Vulcanusmon's "for each of your link cards, ＜De-Digivolve 1＞ all of your
    /// opponent's Digimon" — DCGO reads
    /// `card.Owner.GetBattleAreaDigimons().Map(p => p.LinkedCards).Flat().Count()`.
    /// Composes inside `base_per_delta`. G-DSL-FORMULA-OWN-LINK-CARD-COUNT.
    OwnLinkCardCount { of: PlayerRef },
    /// Number of link cards on the effect carrier's OWN permanent
    /// (`ctx.source_permanent.linked_cards.len()`). YAML form:
    /// `source_link_card_count`. The per-host sibling of `own_link_card_count`.
    /// G-DSL-LINK-N-CARDS-PER-HOST (formula facet).
    SourceLinkCardCount,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceStackCountSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Box<crate::predicate::PredicateSpec>>,
}

impl Serialize for PerSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::MaterialCount => serializer.serialize_str("material_count"),
            Self::StackSize => serializer.serialize_str("stack_size"),
            Self::AllyCount => serializer.serialize_str("ally_count"),
            Self::SuspendedCount { of, exclude_source } => {
                let mut outer = serializer.serialize_map(Some(1))?;
                #[derive(Serialize)]
                struct SuspendedPayload {
                    of: PlayerRef,
                    #[serde(skip_serializing_if = "std::ops::Not::not")]
                    exclude_source: bool,
                }
                outer.serialize_entry(
                    "suspended_count",
                    &SuspendedPayload {
                        of: *of,
                        exclude_source: *exclude_source,
                    },
                )?;
                outer.end()
            }
            Self::DigivolutionColorCount => serializer.serialize_str("digivolution_color_count"),
            Self::SourceColorCount => serializer.serialize_str("source_color_count"),
            Self::SameLevelPairsInSources => {
                serializer.serialize_str("same_level_pairs_in_sources")
            }
            Self::SharedTrashCount { bucket } => {
                let mut outer = serializer.serialize_map(Some(1))?;
                #[derive(Serialize)]
                struct SharedTrashPayload {
                    #[serde(skip_serializing_if = "Option::is_none")]
                    bucket: Option<u32>,
                }
                outer.serialize_entry(
                    "shared_trash_count",
                    &SharedTrashPayload { bucket: *bucket },
                )?;
                outer.end()
            }
            Self::CardCountInZone(spec) => {
                let mut outer = serializer.serialize_map(Some(1))?;
                outer.serialize_entry("card_count_in_zone", spec)?;
                outer.end()
            }
            Self::DistinctColorsCount(spec) => {
                let mut outer = serializer.serialize_map(Some(1))?;
                outer.serialize_entry("distinct_colors_count", spec)?;
                outer.end()
            }
            Self::SourceStackCount(spec) => {
                let mut outer = serializer.serialize_map(Some(1))?;
                outer.serialize_entry("source_stack_count", spec)?;
                outer.end()
            }
            Self::PlayerMemory { of } => {
                let mut outer = serializer.serialize_map(Some(1))?;
                #[derive(Serialize)]
                struct PlayerMemoryPayload {
                    of: PlayerRef,
                }
                outer.serialize_entry("player_memory", &PlayerMemoryPayload { of: *of })?;
                outer.end()
            }
            Self::OwnLinkCardCount { of } => {
                let mut outer = serializer.serialize_map(Some(1))?;
                #[derive(Serialize)]
                struct OwnLinkPayload {
                    of: PlayerRef,
                }
                outer.serialize_entry("own_link_card_count", &OwnLinkPayload { of: *of })?;
                outer.end()
            }
            Self::SourceLinkCardCount => serializer.serialize_str("source_link_card_count"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AggregateSelector {
    LowestDp,
    HighestDp,
    LowestLevel,
    HighestLevel,
    FewestMaterials,
    /// Lowest printed play cost among the scope's **Digimon** (Tamers are
    /// excluded — DCGO `IsMinCost(.., IsDigimonOnly: true)`). Drives BT9-112's
    /// "delete all opponent Digimon with the lowest play cost". The DP/Level
    /// aggregates are already Digimon-only since Tamers lack DP/level.
    /// G-PLAY-COST-AGGREGATE.
    LowestPlayCost,
}
