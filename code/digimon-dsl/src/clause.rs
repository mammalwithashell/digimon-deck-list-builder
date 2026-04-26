//! Clause types — triggered (with `when:` + `process:`) vs declarative
//! (with `kind:` discriminator). Spec §3.5.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::predicate::PredicateSpec;
use crate::step::StepSpec;

/// A clause is either triggered or declarative. Untagged serde enum —
/// presence of `when:` ⇒ triggered; presence of `kind:` ⇒ declarative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ClauseSpec {
    Triggered(TriggeredClause),
    Declarative(DeclarativeClause),
}

impl ClauseSpec {
    pub fn as_triggered(&self) -> Option<&TriggeredClause> {
        match self {
            ClauseSpec::Triggered(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_declarative(&self) -> Option<&DeclarativeClause> {
        match self {
            ClauseSpec::Declarative(d) => Some(d),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TriggeredClause {
    pub when: TimingSet,

    #[serde(default, skip_serializing_if = "ClauseScope::is_face_up")]
    pub scope: ClauseScope,

    /// Optional short effect summary displayed when the clause activates.
    /// Authored in `en-US`; used as the canonical localization key when
    /// translated. See spec §7b.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Explicit localization key override. If absent, derived positionally
    /// as `<card_id>.clause[<index>].summary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_key: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_when: Option<PredicateSpec>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PredicateSpec>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub once_per_turn: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_per_turn: Option<u8>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub process: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum TimingSet {
    Single(Timing),
    Multi(Vec<Timing>),
}

/// Every value allowed in `when:`. Maps 1:1 to a variant of
/// `crate::enums::EffectTiming` at lowering time (Phase 2). Spec §3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    OnPlay,
    WhenDigivolving,
    WhenAttacking,
    EndOfAttack,
    EndOfBattle,
    OnAttack,
    OnDeletion,
    OnAnyDeletion,
    OnEnterFieldAnyone,
    OnAllyPlayed,
    OnLeaveField,
    OnSuspend,
    OnUnsuspend,
    OnHatch,
    OnDigivolve,
    OnDnaDigivolve,
    #[serde(rename = "on_digixros")]
    OnDigixros,
    OnOpponentSecurityRemoved,
    OnDigivolutionCardTrashed,
    OnSecurityCheck,
    OnLoseSecurity,
    OnSecurity,
    OnOptionPlaced,
    StartOfYourTurn,
    StartOfOpponentsTurn,
    StartOfYourMainPhase,
    EndOfYourTurn,
    EndOfOpponentsTurn,
    OnAttackTargetChange,
    MainFromHand,
    MainOnField,
    MainFromTrash,
    Counter,
    BeforePayCost,
    Delayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClauseScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
}

impl ClauseScope {
    pub(crate) fn is_face_up(&self) -> bool {
        matches!(self, ClauseScope::FaceUp)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
// NOTE: DeclarativeClause deliberately omits `deny_unknown_fields` because the
// free-form `body` IndexMap (via `#[serde(flatten)]`) absorbs per-kind fields
// whose schema is enforced by Task 9's `typed_body()`. Serde does not allow
// `deny_unknown_fields` together with `flatten` on the same struct.
pub struct DeclarativeClause {
    pub kind: DeclarativeKind,

    #[serde(default, skip_serializing_if = "ClauseScope::is_face_up")]
    pub scope: ClauseScope,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_when: Option<PredicateSpec>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_key: Option<String>,

    /// Free-form body keyed by clause-kind — validated in Task 9 via `typed_body()`.
    /// Storing as `IndexMap<String, serde_yml::Value>` preserves key order
    /// for the pretty-printer.
    #[serde(flatten)]
    #[schemars(skip)]
    pub body: IndexMap<String, serde_yml::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeclarativeKind {
    Aura,
    CostReduction,
    Replacement,
    Partition,
    AceOverflow,
    GrantKeyword,
    Delay,
    FloodGate,
    AltPathRegistration,
    RawRust,
}

// ---------------------------------------------------------------------------
// Task 9 — typed per-kind declarative-clause bodies
// ---------------------------------------------------------------------------

/// The typed body variant for a [`DeclarativeClause`], obtained via
/// [`DeclarativeClause::typed_body`]. One enum arm per [`DeclarativeKind`].
#[derive(Debug, Clone, PartialEq)]
pub enum TypedDeclarativeBody {
    Aura(AuraBody),
    CostReduction(CostReductionBody),
    Replacement(ReplacementBody),
    Partition(PartitionBody),
    AceOverflow(AceOverflowBody),
    GrantKeyword(GrantKeywordBody),
    Delay(DelayBody),
    FloodGate(FloodGateBody),
    AltPathRegistration(AltPathRegistrationBody),
    RawRust(RawRustClauseBody),
}

impl DeclarativeClause {
    /// Deserialize the free-form `body` map into the typed variant matching
    /// `self.kind`. Returns `Err` if the body does not match the kind's schema.
    pub fn typed_body(&self) -> Result<TypedDeclarativeBody, serde_yml::Error> {
        use serde_yml::Value;

        let value = Value::Mapping(
            self.body
                .iter()
                .map(|(k, v)| (Value::String(k.clone()), v.clone()))
                .collect(),
        );

        Ok(match self.kind {
            DeclarativeKind::Aura => {
                TypedDeclarativeBody::Aura(serde_yml::from_value(value)?)
            }
            DeclarativeKind::CostReduction => {
                TypedDeclarativeBody::CostReduction(serde_yml::from_value(value)?)
            }
            DeclarativeKind::Replacement => {
                TypedDeclarativeBody::Replacement(serde_yml::from_value(value)?)
            }
            DeclarativeKind::Partition => {
                TypedDeclarativeBody::Partition(serde_yml::from_value(value)?)
            }
            DeclarativeKind::AceOverflow => {
                TypedDeclarativeBody::AceOverflow(serde_yml::from_value(value)?)
            }
            DeclarativeKind::GrantKeyword => {
                TypedDeclarativeBody::GrantKeyword(serde_yml::from_value(value)?)
            }
            DeclarativeKind::Delay => {
                TypedDeclarativeBody::Delay(serde_yml::from_value(value)?)
            }
            DeclarativeKind::FloodGate => {
                TypedDeclarativeBody::FloodGate(serde_yml::from_value(value)?)
            }
            DeclarativeKind::AltPathRegistration => {
                TypedDeclarativeBody::AltPathRegistration(serde_yml::from_value(value)?)
            }
            DeclarativeKind::RawRust => {
                TypedDeclarativeBody::RawRust(serde_yml::from_value(value)?)
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Per-kind body structs
// ---------------------------------------------------------------------------

/// Body for `kind: aura` — static continuous effect applied to matching
/// permanents while the source is on field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuraBody {
    pub target: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp_modifier: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_keyword: Option<GrantKeywordValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<String>,
}

/// Inline keyword grant used inside [`AuraBody`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordValue {
    pub keyword: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

/// Body for `kind: cost_reduction`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostReductionBody {
    /// Cost-timing discriminator (e.g., `before_pay_cost`). NOT the
    /// clause's zone scope — that lives on `DeclarativeClause.scope`.
    /// Optional because most cards don't need it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduction_timing: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub when_playing_this: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_any_ally_played: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub once_per_turn: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_fn: Option<crate::formula::FormulaSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pay_cost: Option<Vec<StepSpec>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub unlocks: Vec<IndexMap<String, serde_yml::Value>>,
}

/// Body for `kind: replacement`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplacementBody {
    pub trigger: String,
    pub process: Vec<StepSpec>,
}

/// Body for `kind: partition` — declares which digivolution-source cards form
/// this permanent's digivolution stack partition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartitionBody {
    pub sources: Vec<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_cause: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub process: Vec<StepSpec>,
}

/// Body for `kind: ace_overflow`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AceOverflowBody {
    pub value: i32,
}

/// Body for `kind: grant_keyword` — statically grants a keyword to this card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordBody {
    pub keyword: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

/// Body for `kind: delay`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DelayBody {
    pub trigger: Timing,
    pub process: Vec<StepSpec>,
}

/// Body for `kind: flood_gate` — applies a blanket modifier to matching
/// permanents (e.g. `CannotDigivolve`, `CannotAttack`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FloodGateBody {
    pub modifier: String,
    pub target: PredicateSpec,
}

/// Body for `kind: alt_path_registration`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AltPathRegistrationBody {
    pub trigger: Timing,
    #[schemars(skip)]
    pub registers: IndexMap<String, serde_yml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to: Option<PredicateSpec>,
}

/// Body for `kind: raw_rust` — escape hatch pointing at a hand-written Rust
/// function for effects that cannot be expressed declaratively yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawRustClauseBody {
    #[serde(rename = "fn")]
    pub fn_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<Timing>,
}
