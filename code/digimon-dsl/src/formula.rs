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
    SourceStackDpSum {
        source_stack_dp_sum: SourceStackDpSumSpec,
    },
    Compound(CompoundFormula),
}

/// Empty payload marker for the `source_dp` / `source_material_count`
/// formulas. Exists so the untagged `FormulaSpec` variant has a
/// distinguishing map key at deserialization. YAML: `{ source_dp: {} }`.
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
    SameLevelPairsInSources,
    SharedTrashCount {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bucket: Option<u32>,
    },
    CardCountInZone(CardCountInZoneSpec),
    DistinctColorsCount(CardCountInZoneSpec),
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
