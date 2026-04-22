//! Alternate entry paths — digivolve / DNA / DigiXros / Burst / Hybrid /
//! App Fusion / Activated Digivolve. Spec §3.3.

use serde::{Deserialize, Serialize};

use crate::dsl::formula::FormulaSpec;
use crate::dsl::predicate::PredicateSpec;
use crate::dsl::step::StepSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltPathSpec {
    pub kind: AltPathKind,

    /// For digivolve / activated_digivolve / burst_digivolve / assembly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<PredicateSpec>,

    /// For dna_digivolve / digixros / app_fusion / assembly.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<MaterialSpec>,

    /// Memory cost — literal or formula. Optional only for `cost_reduction`-driven paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostSpec>,

    /// DNA stacks both parents under the evolved card, unsuspended.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stacks_unsuspended: bool,

    /// activated_digivolve — ignore printed digivolution requirements.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,

    /// Identity override (X-Antibody).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_treated_as: Option<String>,

    /// Extra cost steps paid before the path resolves (e.g. "return Yoshino to hand").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_cost: Option<Vec<StepSpec>>,

    /// Burst-digivolve: run at the end of the burst turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_burst_turn_end: Option<Vec<StepSpec>>,

    /// DigiXros `[Hand] [Counter] <Blast Digivolve>` marker form.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub marker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltPathKind {
    Digivolve,
    DnaDigivolve,
    #[serde(rename = "digixros")]
    DigiXros,
    BurstDigivolve,
    AppFusion,
    Assembly,
    ActivatedDigivolve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CostSpec {
    Literal(i32),
    /// `cost: { formula: { base: N, per: "...", delta: M } }`
    Formula(FormulaCost),
}

/// Wraps `FormulaSpec` under the `formula:` YAML key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaCost {
    pub formula: FormulaSpec,
}

/// A material entry in a multi-material alt-path.
///
/// Two YAML forms are supported:
/// - **Inline**: predicate fields directly on the map, e.g. `{ level_eq: 6, name_contains: Greymon }`
/// - **Wrapped**: a `filter:` key holding a `PredicateSpec`, e.g. `{ filter: { any_of: [...] }, repeat: unbounded }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialSpec {
    /// Explicit filter wrapper — used when the predicate is complex (e.g. `any_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<PredicateSpec>,

    /// Inline predicate fields for simple cases. Flattened so fields like `level_eq`,
    /// `name_contains`, `name_is`, `trait_has` may appear directly on the material map.
    #[serde(flatten)]
    pub inline_filter: PredicateSpec,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatSpec>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_by: Option<DistinctBy>,

    /// Zones the material may come from (digixros cross-zone).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<crate::dsl::predicate::Zone>,

    /// Assembly: materials go under the evolved card.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stack_under: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepeatSpec {
    Keyword(RepeatKeyword),
    Range { min: u8, max: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatKeyword {
    Unbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistinctBy {
    CardNumber,
    Level,
    Name,
}
