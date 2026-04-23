//! Compiled card IR — rkyv-friendly mirror of `CardSpec` used as the
//! on-disk / in-memory format for distributed card packs.
//!
//! Phase 1b Task 2: top-level types + stubs for nested. Task 3 populates
//! the stubs. Task 4 adds the lowering pass (CardSpec → CompiledCard).
//!
//! # rkyv design note
//!
//! rkyv 0.7 cannot derive `Archive` for self-referential types (e.g. a type
//! that transitively contains `Vec<Self>`).  `CompiledCard`'s outermost
//! fields — card number, name, level, cost, DP, traits, etc. — are
//! non-recursive and are zero-copy archived normally.  The recursive effect
//! tree (`CompiledClause` / `CompiledStep` / `CompiledPredicate` /
//! `CompiledFormula`) and the alt-path list are stored as `serde_json`-
//! encoded byte blobs (`effects_blob` / `alt_paths_blob`) within the
//! rkyv-archived `CompiledCard`.  The typed accessor helpers
//! `CompiledCard::effects()` and `CompiledCard::alt_paths()` decode them on
//! demand.  This keeps zero-copy for the hot-path metadata look-ups while
//! avoiding the rkyv recursive-type limitation.

use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};
use serde::{Serialize, Deserialize};

// ── Top-level card ──────────────────────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledCard {
    pub card: String,
    pub name: String,
    pub kind: CompiledCardKind,
    pub level: Option<u8>,
    pub color: Vec<CompiledColor>,
    pub cost: Option<i32>,
    pub dp: Option<i32>,
    pub traits: Vec<String>,
    pub form: Option<String>,
    pub attribute: Option<String>,
    pub ace_overflow: Option<i32>,
    /// Encoded `Option<CompiledIdentity>` — serde_json bytes.
    pub identity_blob: Vec<u8>,
    /// Encoded `Vec<CompiledAltPath>` — serde_json bytes.
    pub alt_paths_blob: Vec<u8>,
    /// Encoded `Vec<CompiledClause>` — serde_json bytes.
    pub effects_blob: Vec<u8>,
}

impl CompiledCard {
    /// Decode the identity field.  Returns `None` if identity is absent or
    /// decoding fails (should not happen for well-formed packs).
    pub fn identity(&self) -> Option<CompiledIdentity> {
        serde_json::from_slice(&self.identity_blob).ok().flatten()
    }

    /// Decode the alt-paths list.
    pub fn alt_paths(&self) -> Vec<CompiledAltPath> {
        serde_json::from_slice(&self.alt_paths_blob).unwrap_or_default()
    }

    /// Decode the effect clause list.
    pub fn effects(&self) -> Vec<CompiledClause> {
        serde_json::from_slice(&self.effects_blob).unwrap_or_default()
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledCardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledColor {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
}

// ── Identity ────────────────────────────────────────────────────────

/// Non-recursive — safe to derive rkyv Archive.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledIdentity {
    pub name_aliases: Vec<CompiledNameAlias>,
}

/// Non-recursive — safe to derive rkyv Archive.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledNameAlias {
    pub treat_as: String,
    pub zone: Vec<CompiledZone>,
    pub has_inherited_card_number: Option<String>,
    pub has_inherited_name: Option<String>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledZone {
    Hand,
    Deck,
    Trash,
    BattleArea,
    Security,
    Breeding,
    Reveal,
    DigiEggDeck,
    Material,
}

// ── Alt-paths ───────────────────────────────────────────────────────
// All alt-path types are recursive (they embed CompiledPredicate /
// CompiledStep) so they use serde only, no rkyv Archive derive.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledAltPath {
    pub kind: CompiledAltPathKind,
    pub from: Option<Box<CompiledPredicate>>,
    pub materials: Vec<CompiledMaterial>,
    pub cost: Option<CompiledCost>,
    pub stacks_unsuspended: bool,
    pub ignore_requirements: bool,
    pub source_treated_as: Option<String>,
    pub extra_cost: Vec<CompiledStep>,
    pub on_burst_turn_end: Vec<CompiledStep>,
    pub marker: bool,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledAltPathKind {
    Digivolve,
    DnaDigivolve,
    DigiXros,
    BurstDigivolve,
    AppFusion,
    Assembly,
    ActivatedDigivolve,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledMaterial {
    pub filter: CompiledPredicate,
    pub repeat: Option<CompiledRepeat>,
    pub distinct_by: Option<CompiledDistinctBy>,
    pub zones: Vec<CompiledZone>,
    pub stack_under: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledRepeat {
    Unbounded,
    Range { min: u8, max: u8 },
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledDistinctBy {
    CardNumber,
    Level,
    Name,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledCost {
    Literal(i32),
    Formula(CompiledFormula),
}

// ── Predicate tree ──────────────────────────────────────────────────
// Recursive (contains Vec<CompiledPredicate>, Box<CompiledPredicate>).
// serde only.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct CompiledPredicate {
    pub kind: Option<CompiledCardKind>,
    pub level_eq: Option<u8>,
    pub level_lte: Option<u8>,
    pub level_gte: Option<u8>,
    pub color_is: Option<CompiledColor>,
    pub color_only: Option<Vec<CompiledColor>>,
    pub trait_has: Option<String>,
    pub form_is: Option<String>,
    pub attribute_is: Option<String>,
    pub name_is: Option<String>,
    pub name_contains: Option<String>,
    pub name_in: Option<Vec<String>>,
    pub card_number_is: Option<String>,
    pub dp_eq: Option<CompiledDpConstraint>,
    pub dp_lte: Option<CompiledDpConstraint>,
    pub dp_gte: Option<CompiledDpConstraint>,
    pub stack_size_lte: Option<u8>,
    pub stack_size_gte: Option<u8>,
    pub materials_count_lte: Option<u8>,
    pub materials_count_gte: Option<u8>,
    pub has_inherited: Option<Box<CompiledPredicate>>,
    pub is_suspended: Option<bool>,
    pub is_unsuspended: Option<bool>,
    pub has_keyword: Option<String>,
    pub zone: Vec<CompiledZone>,
    pub owner: Option<CompiledPlayerRef>,
    pub other: Option<bool>,
    pub of_permanent: Option<String>,
    pub source_is_tamer: Option<bool>,
    pub source_name_contains: Option<String>,
    pub source_permanent_trait_has: Option<String>,
    pub memory_lte: Option<i32>,
    pub memory_gte: Option<i32>,
    pub security_count_lte: Option<u8>,
    pub security_count_gte: Option<u8>,
    pub your_turn: Option<bool>,
    pub opponents_turn: Option<bool>,
    pub all_turns: Option<bool>,
    pub in_breeding: Option<bool>,
    pub on_field: Option<bool>,
    pub dna_origin: Option<bool>,
    pub event_target_kind: Option<CompiledCardKind>,
    pub event_target_trait_has: Option<String>,
    pub event_card_trait_has: Option<String>,
    pub equals: Option<Vec<CompiledBindingCompare>>,
    pub not_equals: Option<Vec<CompiledBindingCompare>>,
    pub count_lte: Option<CompiledCountAggregate>,
    pub count_gte: Option<CompiledCountAggregate>,
    pub any_permanent: Option<Box<CompiledExistential>>,
    pub no_permanent: Option<Box<CompiledExistential>>,
    pub all_permanents: Option<Box<CompiledExistential>>,
    pub all_of: Vec<CompiledPredicate>,
    pub any_of: Vec<CompiledPredicate>,
    pub none_of: Vec<CompiledPredicate>,
    pub not: Option<Box<CompiledPredicate>>,
    pub has_alt_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledDpConstraint {
    Literal(i32),
    Formula(CompiledFormula),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledBindingCompare {
    Binding(String),
    Literal(i64),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledCountAggregate {
    pub filter: Box<CompiledPredicate>,
    pub n: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledExistential {
    pub of: CompiledPlayerRef,
    pub predicate: CompiledPredicate,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledPlayerRef {
    You,
    Opponent,
    Any,
    Active,
}

// ── Formulas ────────────────────────────────────────────────────────
// Recursive (Vec<CompiledFormula>).  serde only.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledFormula {
    Literal(i32),
    BasePerDelta {
        base: i32,
        per: CompiledPerSelector,
        delta: i32,
    },
    FloorDiv(Vec<CompiledFormula>),
    Max(Vec<CompiledFormula>),
    Min(Vec<CompiledFormula>),
    Aggregate(CompiledAggregateSelector),
    RawRust(String),
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledPerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    DigivolutionColorCount,
    CardCountInZone,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledAggregateSelector {
    LowestDp,
    HighestDp,
    LowestLevel,
    HighestLevel,
}

// ── Clauses ─────────────────────────────────────────────────────────
// Recursive (contain Vec<CompiledStep>).  serde only.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledClause {
    Triggered(CompiledTriggeredClause),
    Declarative(CompiledDeclarativeClause),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledTriggeredClause {
    pub when: Vec<CompiledTiming>,
    pub scope: CompiledScope,
    pub active_when: Option<CompiledPredicate>,
    pub condition: Option<CompiledPredicate>,
    pub optional: bool,
    pub once_per_turn: bool,
    pub max_per_turn: Option<u8>,
    pub process: Vec<CompiledStep>,
    pub summary: Option<String>,
    pub summary_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledDeclarativeClause {
    Aura {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        target: CompiledPredicate,
        dp_modifier: Option<i32>,
        grant_keyword: Option<CompiledGrantKeywordValue>,
        modifier: Option<String>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    CostReduction {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        reduction_timing: Option<String>,
        when_playing_this: bool,
        when_any_ally_played: Option<CompiledPredicate>,
        condition: Option<CompiledPredicate>,
        once_per_turn: bool,
        amount: Option<i32>,
        amount_fn: Option<CompiledFormula>,
        pay_cost: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Replacement {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        trigger: String,
        process: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Partition {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        sources: Vec<CompiledPredicate>,
        exclude_cause: Vec<String>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    AceOverflow {
        value: i32,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    GrantKeyword {
        keyword: String,
        value: Option<i32>,
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Delay {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        trigger: CompiledTiming,
        process: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    FloodGate {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        modifier: String,
        target: CompiledPredicate,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    AltPathRegistration {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        trigger: CompiledTiming,
        applies_to: Option<CompiledPredicate>,
        registers: CompiledAltPath,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    RawRust {
        fn_name: String,
        triggers: Vec<CompiledTiming>,
        scope: CompiledScope,
        summary: Option<String>,
        summary_key: Option<String>,
    },
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[archive(check_bytes)]
pub enum CompiledScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledTiming {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledGrantKeywordValue {
    pub keyword: String,
    pub value: Option<i32>,
}

// ── Steps ───────────────────────────────────────────────────────────
// Recursive (Vec<CompiledStep>).  serde only.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledStep {
    GainMemory(i32),
    LoseMemory(i32),
    SetMemory(i32),
    Draw { of: CompiledPlayerRef, count: u8 },
    TrashFromTop { of: CompiledPlayerRef, count: u8 },
    AddToHandFromDeck { of: CompiledPlayerRef, card: CompiledBindingRef },
    AddToHandFromTrash { of: CompiledPlayerRef, card: CompiledBindingRef },
    AddToHandFromReveal { of: CompiledPlayerRef, card: CompiledBindingRef },
    TrashFromHandByIndex { of: CompiledPlayerRef, hand_index: CompiledBindingRef },
    TrashFromReveal { of: CompiledPlayerRef, card: CompiledBindingRef },
    ReturnToDeckFromReveal { of: CompiledPlayerRef, card: CompiledBindingRef, position: CompiledStackPosition },
    ShuffleDeck { of: CompiledPlayerRef },
    RevealTopDeck { of: CompiledPlayerRef, count: u8, zone: Option<CompiledZone>, bind_as: Option<String> },
    PlaceRemainderOnDeck { of: CompiledPlayerRef, position: CompiledStackPosition },
    DeletePermanent { target: CompiledBindingRef },
    ReturnToHand { target: CompiledBindingRef },
    ReturnToDeck { target: CompiledBindingRef, position: CompiledStackPosition, include_sources: bool },
    Suspend { target: CompiledBindingRef },
    Unsuspend { target: CompiledBindingRef },
    DeDigivolve { target: CompiledBindingRef, amount: Option<u8>, stop_at_level: Option<u8> },
    PlaceOnSecurity { of: CompiledPlayerRef, source: CompiledBindingRef, position: CompiledStackPosition, face_up: bool },
    PlayToken { controller: CompiledPlayerRef, token_name: String },
    PlaceAsBottomSource { source: CompiledBindingRef, target: CompiledBindingRef },
    TrashTopSource { target: CompiledBindingRef },
    Hatch { of: CompiledPlayerRef },
    PlayFromHand { of: CompiledPlayerRef, hand_index: CompiledBindingRef, cost_delta: Option<CompiledCostDelta> },
    PlayFromHandFree { of: CompiledPlayerRef, hand_index: CompiledBindingRef },
    PlayFromTrash { of: CompiledPlayerRef, trash_index: CompiledBindingRef, cost_delta: Option<CompiledCostDelta> },
    PlayFromTrashFree { of: CompiledPlayerRef, trash_index: CompiledBindingRef },
    PlayFromSecurity,
    PlayFromMaterials { target: CompiledBindingRef, source_index: CompiledBindingRef, cost_delta: Option<CompiledCostDelta> },
    EffectInitiatedDigivolve { target: CompiledBindingRef, from_hand: CompiledBindingRef, cost: i32, ignore_requirements: bool },
    EffectInitiatedDnaDigivolve { target_a: CompiledBindingRef, target_b: CompiledBindingRef, from_hand: CompiledBindingRef, cost: i32, ignore_requirements: bool },
    TrashTopSecurity { of: CompiledPlayerRef },
    MarkSecurityFaceUp { of: CompiledPlayerRef, card: CompiledBindingRef },
    AddDpModifier { target: CompiledBindingRef, value: i32, expiry: String },
    AddModifier { target: CompiledModifierTarget, modifier: String, value: i32, expiry: String },
    GrantKeyword { target: CompiledBindingRef, keyword: String, expiry: String, value: Option<i32> },
    SelectOwnPermanent { filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectOpponentPermanent { filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectHand { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectTrash { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectMaterial { of_permanent: CompiledBindingRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectReveal { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectSecurity { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectUnionZone { of: CompiledPlayerRef, zones: Vec<CompiledZone>, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectOrderedPermutation { items: CompiledBindingRef, bind_as: Option<String>, prompt: String, prompt_key: Option<String> },
    SelectCountCappedMulti { of: CompiledPlayerRef, zone: CompiledZone, max: u8, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional_zero: bool, distinct_by: Option<CompiledDistinctBy> },
    SelectEffectChoice { labels: Vec<String>, bind_as: Option<String>, prompt: String, prompt_key: Option<String> },
    AsSelectingPlayer { of: CompiledPlayerRef, body: Vec<CompiledStep> },
    If { condition: CompiledPredicate, then: Vec<CompiledStep>, else_branch: Vec<CompiledStep> },
    ForEach { over: CompiledPredicate, bind_as: String, body: Vec<CompiledStep> },
    PerSelected { selection: String, bind_as: String, body: Vec<CompiledStep> },
    ScheduleDelayed { when: CompiledTiming, body: Vec<CompiledStep> },
    Optional(Vec<CompiledStep>),
    RawRust { fn_name: String, consumes: Vec<String>, binds: Vec<String> },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledBindingRef {
    Named(String),
    SelfRef,
    Carrier,
    Source,
    EventTarget,
    EventCard,
    Permanent(String),
    Binding(String),
    OfPermanent(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledModifierTarget {
    Binding(CompiledBindingRef),
    Filter(CompiledPredicate),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompiledCostDelta {
    Free,
    Printed,
    Literal(i32),
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledStackPosition {
    Top,
    Bottom,
    Random,
}
