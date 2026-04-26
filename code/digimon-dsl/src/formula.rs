//! Formula primitives for scalar computations in predicates and clauses.
//! Spec §3.10.

use serde::{Deserialize, Serialize};
use serde::ser::SerializeMap;

use crate::common::PlayerRef;
use crate::predicate::Zone;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum FormulaSpec {
    Literal(i32),
    BasePerDelta {
        base: i32,
        per: PerSelector,
        delta: i32,
    },
    Compound(CompoundFormula),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CardCountInZoneSpec {
    pub zone: Zone,
    pub of: PlayerRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    DigivolutionColorCount,
    CardCountInZone(CardCountInZoneSpec),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AggregateSelector {
    LowestDp,
    HighestDp,
    LowestLevel,
    HighestLevel,
}
