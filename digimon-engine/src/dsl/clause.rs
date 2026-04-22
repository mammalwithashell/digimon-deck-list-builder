//! Clause types — triggered (with `when:` + `process:`) vs declarative
//! (with `kind:` discriminator). Spec §3.5.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::dsl::predicate::PredicateSpec;
use crate::dsl::step::StepSpec;

/// A clause is either triggered or declarative. Untagged serde enum —
/// presence of `when:` ⇒ triggered; presence of `kind:` ⇒ declarative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggeredClause {
    pub when: TimingSet,

    #[serde(default)]
    pub scope: ClauseScope,

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimingSet {
    Single(Timing),
    Multi(Vec<Timing>),
}

/// Every value allowed in `when:`. Maps 1:1 to a variant of
/// `crate::enums::EffectTiming` at lowering time (Phase 2). Spec §3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClauseScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// NOTE: DeclarativeClause deliberately omits `deny_unknown_fields` because the
// free-form `body` IndexMap (via `#[serde(flatten)]`) absorbs per-kind fields
// whose schema is enforced by Task 9's `typed_body()`. Serde does not allow
// `deny_unknown_fields` together with `flatten` on the same struct.
pub struct DeclarativeClause {
    pub kind: DeclarativeKind,

    #[serde(default)]
    pub scope: ClauseScope,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_when: Option<PredicateSpec>,

    /// Free-form body keyed by clause-kind — validated in Task 9 via `typed_body()`.
    /// Storing as `IndexMap<String, serde_yml::Value>` preserves key order
    /// for the pretty-printer.
    #[serde(flatten)]
    pub body: IndexMap<String, serde_yml::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
