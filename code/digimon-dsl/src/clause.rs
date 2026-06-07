//! Clause types — triggered (with `when:` + `process:`) vs declarative
//! (with `kind:` discriminator). Spec §3.5.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::predicate::PredicateSpec;
use crate::step::{BindingRef, StepSpec};

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
    OnBlock,
    OnAllyAttack,
    OnOpponentAttack,
    OnDeletion,
    OnAnyDeletion,
    OnEnterFieldAnyone,
    OnAnyDigimonPlayed,
    OnAllyPlayed,
    OnLeaveField,
    OnSuspend,
    OnUnsuspend,
    OnHatch,
    OnMove,
    OnDigivolve,
    OnDnaDigivolve,
    #[serde(rename = "on_digixros")]
    OnDigixros,
    OnOpponentSecurityRemoved,
    OnOwnSecurityRemoved,
    OnDigivolutionCardTrashed,
    OnSecurityCheck,
    OnCheckFaceUpSecurity,
    OnLoseSecurity,
    OnDiscardSecurity,
    OnSecurity,
    OnOptionPlaced,
    OnOptionTrashed,
    OnPlaceSecurity,
    OnAddedToSecurity,
    Main,
    StartOfYourTurn,
    StartOfOpponentsTurn,
    StartOfYourMainPhase,
    EndOfYourTurn,
    EndOfOpponentsTurn,
    EndOfYourNextTurn,
    EndOfOpponentsNextTurn,
    UntilNextUnsuspend,
    OnAttackTargetChange,
    MainFromHand,
    MainOnField,
    MainFromTrash,
    Counter,
    BeforePayCost,
    /// Sibling of `before_pay_cost` for observer-style triggered bodies
    /// (e.g. "[Your Turn] When this Digimon would DNA digivolve into a
    /// green Digimon card, gain 1 memory."). Fires at the same dispatch
    /// point as `before_pay_cost` but runs the clause's `process:` body
    /// instead of accumulating cost reduction. Pair with
    /// `cost_target: { ... }` predicates inside `active_when:` to gate on
    /// the digivolve-target card's traits/colors/level/name.
    /// G-BEFORE-PAY-COST-GAIN-MEMORY (Phase 2 Track H closure).
    BeforePayCostObserve,
    Delayed,
    /// DigiLink Shape-B: "when this Digimon gets linked" (`when: when_linked`).
    /// Use on a `scope: linked` effect; lowers to `OnLink` + a self-filter.
    WhenLinked,
    /// DigiLink host-side: "[When Linked] when a card gets linked **to this
    /// Digimon**" (`when: when_card_linked_to_this`). The effect lives on the
    /// HOST (a face-up `scope`), not on the linked card. Lowers to `OnLink`
    /// + a host self-filter (`event_permanent == source_permanent`) so it
    /// fires once for the host the card actually attached to and not for a
    /// sibling host. Mirrors DCGO `CardEffectCommons.CanTriggerWhenLinked`.
    WhenCardLinkedToThis,
    /// DigiLink host-side pre-link **replacement**: "when a card **would** link
    /// **to this Digimon**" (`when: when_would_link_to_this`). The effect lives
    /// on the HOST (a face-up `scope`). Lowers to a `WhenWouldLink` REPLACEMENT
    /// effect (not a triggered observer) + a host self-filter
    /// (`pending_link_host() == source_permanent`) so it fires only while the
    /// linking card is attaching to THIS permanent. Pair with an `optional`
    /// clause + a `reduce_link_cost` step to express "you may reduce the cost"
    /// (Gap 5 — BT25-004 Tapmon / BT25-045 Onmon). Filter the would-link card's
    /// traits via `active_when: { would_link_card_trait_any_of: [...] }`.
    WhenWouldLinkToThis,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ClauseScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
    Linked,
    Security,
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
    LinkRequirement,
    LinkCondition,
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
    LinkRequirement(LinkRequirementBody),
    LinkCondition(LinkRequirementBody),
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
            DeclarativeKind::Aura => TypedDeclarativeBody::Aura(serde_yml::from_value(value)?),
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
            DeclarativeKind::Delay => TypedDeclarativeBody::Delay(serde_yml::from_value(value)?),
            DeclarativeKind::LinkRequirement => {
                TypedDeclarativeBody::LinkRequirement(serde_yml::from_value(value)?)
            }
            DeclarativeKind::LinkCondition => {
                TypedDeclarativeBody::LinkCondition(serde_yml::from_value(value)?)
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_player: Option<crate::common::PlayerRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp_modifier: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp_modifier_fn: Option<crate::formula::FormulaSpec>,
    /// Flat `Security A. ±N` grant. Track H §1 — `AuraGrant::SecurityAttack(i32)`.
    /// Installs `ModifierType::SecurityAttackChange` carrying the literal
    /// delta on each matching target. Use this for printed text like
    /// "your Olympos XII Digimon get Security Attack +1"; use
    /// `security_attack_fn` only when the value is computed dynamically
    /// (formula over board state).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_attack_fn: Option<crate::formula::FormulaSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_keyword: Option<GrantKeywordValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<String>,
    /// Scalar value for the named `modifier` grant (the modifier's `value`
    /// field). Required for scalar modifiers like `ChangeLinkMax` ("Link +N")
    /// and `ChangeLinkCost`; defaults to `0` when omitted (boolean / flag
    /// modifiers ignore it). Fixes G-ENGINE-AURA-GRANT-LINK-MAX — the aura
    /// path previously installed every named modifier with a hardcoded `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier_value: Option<i32>,
    /// Track H §4 — install-once continuous gate. When set, the aura's
    /// modifier installs with `Expiry::UntilCondition` carrying this
    /// predicate. The UntilCondition controller (PR #458) evicts the
    /// modifier as soon as the predicate flips false; per the
    /// printed-semantics rule, `false → true` does NOT re-install.
    /// Distinct from `active_when`, which gates per-tick re-installation
    /// and is symmetric. Use `while_condition` for printed text like
    /// "while opponent has no unsuspended Digimon, this Digimon gains X"
    /// where the eviction is final once it fires. Currently lowers for
    /// self-auras with `dp_modifier`, `security_attack`, or named
    /// `modifier` grants (keyword grants TBD — `KeywordEntry` lacks an
    /// `until_condition` field; tracked separately).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub while_condition: Option<PredicateSpec>,

    /// PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA: when `true`, this aura's
    /// `dp_modifier` is contributed to the attacker's `security_dp_adjustment`
    /// during a security battle (the printed text "all of your opponent's
    /// security Digimon get -3000 DP" on cards like ST19-03 / EX7-024).
    /// Lowers to `EffectBuilder::applies_to_opponent_security_dp()`. The
    /// aura must be inherited so the flag rides under the attacker's
    /// digivolution stack and surfaces during the security check. Use with
    /// `scope: inherited` + `active_when: { your_turn: true }` to match the
    /// printed turn gate. Distinct from a battle-area DP aura — only the
    /// security battle context consults this flag (`combat.rs:260`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to_opponent_security_dp: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to_own_security_dp: Option<bool>,
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
    /// Digivolution-target-keyed cost-reduction trigger. When set, the
    /// reduction fires only for a DIGIVOLVE cost (not a play) whose target
    /// card — the card being digivolved INTO — matches this predicate. The
    /// predicate is evaluated against the digivolve target as a `Card`
    /// subject, so the full card-shape vocabulary (`name_contains`,
    /// `trait_has`, `any_of`, ...) applies. Models card text of the form
    /// "When one of your Digimon would digivolve into a [Name] card, ...".
    /// Used by BT5-092 (Nokia Shiramine). `G-COST-REDUCTION-DIGIVOLVE-INTO`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_any_ally_digivolves_into: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub once_per_turn: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ReplacementCostBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choose: Option<ReplacementChooseBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<ReplacementOutcome>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "then")]
    pub then_steps: Vec<ReplacementThenStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub process: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplacementCostBody {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub delay_self: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplacementChooseBody {
    #[serde(rename = "from")]
    pub from_zone: ReplacementChooseFrom,
    pub card_filter: PredicateSpec,
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementChooseFrom {
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementOutcome {
    Prevent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplacementThenStep {
    pub digivolve_without_cost: ReplacementDigivolveWithoutCostBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplacementDigivolveWithoutCostBody {
    pub target: BindingRef,
    pub card: BindingRef,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overclock_cost_filter: Option<PredicateSpec>,
}

/// Body for `kind: delay`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DelayBody {
    pub trigger: Timing,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_when: Option<PredicateSpec>,
    pub process: Vec<StepSpec>,
}

/// Body for `kind: link_requirement` on Link Options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkRequirementBody {
    pub cost: u16,
    pub filter: PredicateSpec,
}

/// Body for `kind: flood_gate` — applies a blanket modifier to matching
/// permanents (e.g. `CannotDigivolve`, `CannotAttack`) or to referenced
/// players (e.g. `CannotPlayDigimonByEffect`, `CannotReducePlayCost`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FloodGateBody {
    pub modifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_player: Option<crate::common::PlayerRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
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
