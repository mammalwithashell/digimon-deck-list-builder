//! Alternate entry paths — digivolve / DNA / DigiXros / Burst / Hybrid /
//! App Fusion / Activated Digivolve. Spec §3.3.

use serde::{Deserialize, Serialize};

use crate::formula::FormulaSpec;
use crate::predicate::PredicateSpec;
use crate::step::StepSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

    /// Activation gate evaluated on top of the existing source-filter /
    /// extra-cost gates. Used by activated-digivolve clauses with
    /// printed conditions like "If you have [Owen Dreadnought], …"
    /// where the alt-path must check arbitrary game state, not just
    /// material availability. Closes G-ALT-PATH-CONDITION (BT24-016).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PredicateSpec>,

    /// Phase 2 Track F (G-ALT-PATH-DIRECTION-INTO) — flips the
    /// AltPathSpec semantic. The default (`From`) keeps the legacy
    /// reading: the alt-path lives on the **destination** card and
    /// `from:` filters the candidate source. The `Into` variant flips
    /// it: the alt-path lives on the **source** card (carrier
    /// permanent) and `from:` filters the candidate destination
    /// (typically a hand card). Used by ST20-10 Agumon's
    /// "[Your Turn] this Digimon can digivolve into [WarGreymon] in
    /// the hand for cost 4" shape. DCGO models both directions via the
    /// same `AddSelfDigivolutionRequirementStaticEffect`; the DSL
    /// surface needed an explicit flag to disambiguate. Defaults to
    /// `From` for back-compat with existing YAML.
    #[serde(default, skip_serializing_if = "is_default_direction")]
    pub direction: AltPathDirection,
}

fn is_default_direction(d: &AltPathDirection) -> bool {
    matches!(d, AltPathDirection::From)
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AltPathDirection {
    /// Legacy direction: the alt-path is registered on the
    /// destination card; `from:` filters the source permanent /
    /// hand-card candidate.
    #[default]
    From,
    /// New direction (Phase 2 Track F): the alt-path is registered on
    /// the source card; `from:` filters the destination hand-card
    /// candidate. ST20-10-shape "this card may digivolve into X".
    Into,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AltPathKind {
    Digivolve,
    DnaDigivolve,
    BlastDnaDigivolve,
    #[serde(rename = "digixros")]
    DigiXros,
    BurstDigivolve,
    AppFusion,
    Assembly,
    ActivatedDigivolve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CostSpec {
    Literal(i32),
    /// `cost: { formula: { base: N, per: "...", delta: M } }`
    Formula(FormulaCost),
}

/// Wraps `FormulaSpec` under the `formula:` YAML key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FormulaCost {
    pub formula: FormulaSpec,
}

/// A material entry in a multi-material alt-path.
///
/// Two YAML forms are supported:
/// - **Inline**: predicate fields directly on the map, e.g. `{ level_eq: 6, name_contains: Greymon }`
/// - **Wrapped**: a `filter:` key holding a `PredicateSpec`, e.g. `{ filter: { any_of: [...] }, repeat: unbounded }`
// NOTE: MaterialSpec deliberately omits `#[serde(deny_unknown_fields)]` because
// serde does not permit combining it with `#[serde(flatten)]`. As a consequence,
// typos in inline-predicate fields (e.g. `levle_eq: 6`) are silently dropped
// rather than raising a parse error. The semantic validator (Task 12) will need
// a pass over inline-material predicate fields to catch this — tracked as part
// of Task 7 when `PredicateSpec` is fleshed out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    pub zones: Vec<crate::predicate::Zone>,

    /// Assembly: materials go under the evolved card.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stack_under: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum RepeatSpec {
    Keyword(RepeatKeyword),
    Range { min: u8, max: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepeatKeyword {
    Unbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DistinctBy {
    CardNumber,
    Level,
    Name,
}
