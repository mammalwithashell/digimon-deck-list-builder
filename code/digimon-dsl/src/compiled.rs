//! Compiled card IR — bincode-friendly mirror of `CardSpec`.
//!
//! Used as the on-disk / in-memory format for distributed card packs.
//! Phase 1b Task 3: full IR with recursive types embedded directly
//! (no zero-copy — bincode does a single-shot deserialize at boot,
//! ~20ms for a 4000-card pack, well inside the 100ms budget).
//!
//! Phase 1c will add the bridge from `CompiledCard` to engine `Effect`
//! closures.

use serde::{Deserialize, Serialize};

// ── Top-level ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub identity: Option<CompiledIdentity>,
    pub digixros_aliases: Vec<String>,
    /// Static identity aliases — see `CardSpec::also_treated_as`.
    /// Honored by generic name-matching predicates in every zone.
    pub also_treated_as: Vec<String>,
    pub dual: Option<CompiledDual>,
    pub use_requirement: Option<CompiledPredicate>,
    pub alt_paths: Vec<CompiledAltPath>,
    pub effects: Vec<CompiledClause>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledCardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
    Dual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledColor {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledDual {
    pub digimon: CompiledDualDigimon,
    pub option: CompiledDualOption,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledDualDigimon {
    pub level: u8,
    pub dp: i32,
    pub colors: Vec<CompiledColor>,
    pub traits: Vec<String>,
    pub effect_text: String,
    pub inherited_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledDualOption {
    pub use_cost: u16,
    pub colors: Vec<CompiledColor>,
    pub effect_text: String,
    pub security_text: String,
    pub keywords: Vec<String>,
    pub use_requirement: Option<Box<CompiledPredicate>>,
}

// ── Identity ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledIdentity {
    pub name_aliases: Vec<CompiledNameAlias>,
    pub source_name_aliases: Vec<CompiledSourceNameAlias>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledNameAlias {
    pub treat_as: String,
    pub zone: Vec<CompiledZone>,
    pub has_inherited_card_number: Option<String>,
    pub has_inherited_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledSourceNameAlias {
    pub level_lte: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Activation gate on top of source/material/extra-cost gates.
    /// Closes G-ALT-PATH-CONDITION (BT24-016).
    #[serde(default)]
    pub condition: Option<Box<CompiledPredicate>>,
    /// Phase 2 Track F (G-ALT-PATH-DIRECTION-INTO) — direction flip.
    /// `From` (default): legacy reading; alt-path is registered on the
    /// destination card and `from:` filters the source candidate.
    /// `Into`: alt-path is registered on the source card and `from:`
    /// filters the destination hand-card candidate. ST20-10 warp-shape.
    #[serde(default)]
    pub direction: CompiledAltPathDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CompiledAltPathDirection {
    #[default]
    From,
    Into,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledAltPathKind {
    Digivolve,
    DnaDigivolve,
    BlastDnaDigivolve,
    DigiXros,
    BurstDigivolve,
    AppFusion,
    Assembly,
    ActivatedDigivolve,
}

impl CompiledAltPathKind {
    /// Canonical snake_case string key for this alt-path variant.
    ///
    /// Stable identifier consumed by:
    /// - `GameEvent::Play.via_alt_path` (surfaces the alt-path through
    ///   which a card entered play, per the `engine-event-emission` spec)
    /// - The `reward-profiles` `key_cards:` `alt_paths` matcher and the
    ///   `play_named_card.via_alt_path` component matcher (YAML-side keys)
    ///
    /// These keys are part of the engine's public surface — renaming a
    /// variant requires updating downstream YAML profiles AND any code
    /// that pattern-matches on the string. Add `#[serde(rename = "...")]`
    /// here if a future change wants a different serde wire format than
    /// the matcher key.
    pub fn as_key(self) -> &'static str {
        // NOTE: strings here MUST match `alt_path_kind_matches` in
        // `code/digimon-engine/src/dsl_cards/predicate.rs` to keep the
        // engine's single source of truth for alt-path identifiers.
        // Two divergences from a "pure snake_case" reading exist for
        // historical-predicate-matching reasons:
        //   • BlastDnaDigivolve → "blast_dna_digivolve" (not "blast_dna")
        //   • DigiXros → "digixros" (not "digi_xros")
        // Update both sites in lockstep if a future change wants a
        // different scheme.
        match self {
            CompiledAltPathKind::Digivolve => "digivolve",
            CompiledAltPathKind::DnaDigivolve => "dna_digivolve",
            CompiledAltPathKind::BlastDnaDigivolve => "blast_dna_digivolve",
            CompiledAltPathKind::DigiXros => "digixros",
            CompiledAltPathKind::BurstDigivolve => "burst_digivolve",
            CompiledAltPathKind::AppFusion => "app_fusion",
            CompiledAltPathKind::Assembly => "assembly",
            CompiledAltPathKind::ActivatedDigivolve => "activated_digivolve",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledMaterial {
    pub filter: CompiledPredicate,
    pub repeat: Option<CompiledRepeat>,
    pub distinct_by: Option<CompiledDistinctBy>,
    pub zones: Vec<CompiledZone>,
    pub stack_under: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledRepeat {
    Unbounded,
    Range { min: u8, max: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledDistinctBy {
    CardNumber,
    Level,
    Name,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledCost {
    Literal(i32),
    Formula(CompiledFormula),
}

// ── Predicate tree ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CompiledPredicate {
    pub kind: Option<CompiledCardKind>,
    pub level_eq: Option<u8>,
    pub level_eq_binding: Option<String>,
    pub level_lte: Option<CompiledDpConstraint>,
    pub level_gte: Option<CompiledDpConstraint>,
    pub color_is: Option<CompiledColor>,
    pub color_only: Option<Vec<CompiledColor>>,
    pub color_matches_any_field_digimon: Option<CompiledPlayerRef>,
    pub color_matches_binding: Option<String>,
    pub trait_has: Option<String>,
    pub form_is: Option<String>,
    pub attribute_is: Option<String>,
    pub name_is: Option<String>,
    pub name_contains: Option<String>,
    /// Case-insensitive substring scan against the candidate card's
    /// concatenated printed text (`effect_text` + `inherited_text` +
    /// `security_text`). G-DSL-PREDICATE-TEXT-CONTAINS.
    pub effect_text_contains: Option<String>,
    pub name_in: Option<Vec<String>>,
    /// G-UNION-HAND-TRASH-NAME-EXCLUSION (Phase 2 Track J Task S2.2) —
    /// card-subject leaf: true when no battle-area Digimon of the scoped
    /// player shares the candidate card's name.
    pub name_not_shared_by_field_digimon: Option<CompiledPlayerRef>,
    /// Card-subject leaf: true when no battle-area Tamer of the scoped
    /// player shares the candidate card's name.
    pub name_not_shared_by_field_tamer: Option<CompiledPlayerRef>,
    pub card_number_is: Option<String>,
    pub play_cost_lte: Option<CompiledDpConstraint>,
    pub play_cost_gte: Option<CompiledDpConstraint>,
    pub can_digivolve_from_source: Option<bool>,
    pub dp_eq: Option<CompiledDpConstraint>,
    pub dp_lte: Option<CompiledDpConstraint>,
    pub dp_gte: Option<CompiledDpConstraint>,
    pub stack_size_lte: Option<CompiledDpConstraint>,
    pub stack_size_gte: Option<CompiledDpConstraint>,
    pub materials_count_lte: Option<CompiledDpConstraint>,
    pub materials_count_gte: Option<CompiledDpConstraint>,
    pub has_inherited: Option<Box<CompiledPredicate>>,
    pub is_suspended: Option<bool>,
    pub is_unsuspended: Option<bool>,
    pub has_keyword: Option<String>,
    /// Permanent-subject leaf for printed/granted/temporary Security A.
    /// deltas. Used by Venusmon-style text that cares about "with
    /// <Security A.>" rather than a specific keyword spelling.
    pub has_security_attack_change: Option<bool>,
    /// Phase 2 Track F (G-DSL-HAS-ON-DELETION-EFFECT) — true if the
    /// candidate permanent carries any `EffectTiming::OnDeletion`-timed
    /// triggered effect via a compiled DSL clause or a hand-written
    /// `CardEffect` impl. EX1-021 MetalGarurumon's [When Attacking] arm
    /// gates target selection on this predicate.
    pub has_on_deletion_effect: Option<bool>,
    pub self_color_count_gte: Option<u8>,
    pub has_face_down_source: Option<bool>,
    /// True when the observer's battle-area Tamers collectively have at
    /// least N distinct colors. G-DSL-DISTINCT-TAMER-COLORS.
    pub distinct_tamer_colors_gte: Option<u8>,
    /// Battle-context leaf: true when the effect's carrier is battling an
    /// opposing Digimon with zero digivolution source cards.
    pub battle_opponent_no_sources: Option<bool>,
    pub zone: Vec<CompiledZone>,
    pub owner: Option<CompiledPlayerRef>,
    pub other: Option<bool>,
    pub of_permanent: Option<String>,
    pub not_in_binding: Option<String>,
    pub binding_owner: Option<CompiledBindingOwnerPredicate>,
    pub source_is_tamer: Option<bool>,
    pub source_is_unsuspended: Option<bool>,
    pub source_name_contains: Option<String>,
    pub source_permanent_trait_has: Option<String>,
    pub is_face_down: Option<bool>,
    pub is_bottom_source: Option<bool>,
    pub host_kind_is: Option<CompiledCardKind>,
    /// Case-insensitive substring match against the carrier permanent's
    /// printed rules text (effect_text + inherited_text + security_text of
    /// the top card). Fails when the subject is not a permanent. PUPPETS-G025.
    pub rules_text_contains: Option<String>,
    pub memory_lte: Option<CompiledDpConstraint>,
    pub memory_gte: Option<CompiledDpConstraint>,
    pub security_count_lte: Option<CompiledDpConstraint>,
    pub security_count_gte: Option<CompiledDpConstraint>,
    pub opponent_security_count_lte: Option<CompiledDpConstraint>,
    pub opponent_security_count_gte: Option<CompiledDpConstraint>,
    pub face_up_security_count_lte: Option<CompiledDpConstraint>,
    pub face_up_security_count_gte: Option<CompiledDpConstraint>,
    /// True when the named player has no face-up security card matching the
    /// identity filter. G-PRED-NO-FACE-UP-SECURITY-NAMED.
    pub no_face_up_security_named: Option<CompiledFaceUpSecurityNamed>,
    /// True when the named list-typed binding holds exactly `1` (count)
    /// entries. `(binding_name, n)`. G-DSL-BINDING-COUNT-EQ.
    pub binding_count_eq: Option<(String, u8)>,
    pub your_turn: Option<bool>,
    pub opponents_turn: Option<bool>,
    pub all_turns: Option<bool>,
    pub can_hatch: Option<CompiledPlayerRef>,
    pub digimon_attacked_this_turn: Option<CompiledPlayerRef>,
    pub in_breeding: Option<bool>,
    pub on_field: Option<bool>,
    pub dna_origin: Option<bool>,
    pub event_target_kind: Option<CompiledCardKind>,
    pub event_target_trait_has: Option<String>,
    pub event_target_level_eq: Option<u8>,
    pub event_target_level_lte: Option<CompiledDpConstraint>,
    pub event_target_level_gte: Option<CompiledDpConstraint>,
    pub event_target_dp_eq: Option<CompiledDpConstraint>,
    pub event_target_dp_lte: Option<CompiledDpConstraint>,
    pub event_target_dp_gte: Option<CompiledDpConstraint>,
    /// Case-insensitive substring scan against the event-target
    /// permanent's card name. G-EVENT-TARGET-NAME-CONTAINS.
    pub event_target_name_contains: Option<String>,
    pub event_target_is_player: Option<bool>,
    pub event_target_is_source: Option<bool>,
    pub event_target_was_self: Option<bool>,
    pub attack_target_change_reason: Option<String>,
    pub attacker_trait_has: Option<String>,
    pub event_card_trait_has: Option<String>,
    pub event_card_name_contains: Option<String>,
    pub event_card_level_eq: Option<u8>,
    pub event_card_level_gte: Option<CompiledDpConstraint>,
    /// Every color of the triggering event card must be within this set.
    pub event_card_color_only: Option<Vec<CompiledColor>>,
    /// The triggering event card must have at least one of these colors
    /// (intersection / "has" semantics). G-EVENT-CARD-COLOR-IS.
    pub event_card_color_has: Option<Vec<CompiledColor>>,
    /// The triggering event card must have exactly this many distinct colors.
    pub event_card_color_count: Option<u8>,
    pub event_permanent_is_source: Option<bool>,
    pub source_deleted_battle_opponent: Option<bool>,
    pub event_host_permanent_is_source: Option<bool>,
    pub event_is_effect_initiated: Option<bool>,
    pub event_target_same_level_as_previous: Option<bool>,
    pub event_cause: Option<CompiledEventCause>,
    pub replacement_cause: Option<CompiledReplacementCause>,
    pub replacement_source_is_opponent: Option<bool>,
    pub replacement_subject_is_mine: Option<bool>,
    pub equals: Option<Vec<CompiledBindingCompare>>,
    pub not_equals: Option<Vec<CompiledBindingCompare>>,
    pub binding_exists: Option<String>,
    pub binding_present: Option<String>,
    pub binding_absent: Option<String>,
    pub effect_suspended_any_own_digimon: Option<bool>,
    /// Opponent-side sibling of `effect_suspended_any_own_digimon`.
    /// G-DSL-EFFECT-SUSPENDED-RESULT.
    pub effect_suspended_any_opponent_digimon: Option<bool>,
    pub effect_returned_any_card: Option<bool>,
    /// Filtered variant of `effect_returned_any_card`. When present, the inner
    /// predicate is evaluated as a `Card` subject against each returned card
    /// identity in the per-effect result log. G-ANY-RETURNED-CARD-PREDICATE.
    pub returned_card_matching: Option<Box<CompiledPredicate>>,
    pub effect_deleted_any_own_digimon: Option<bool>,
    pub effect_deleted_any_opponent_digimon: Option<bool>,
    pub effect_played_any_digimon: Option<bool>,
    pub effect_digivolved_any_digimon: Option<bool>,
    pub effect_added_any_card_to_hand: Option<bool>,
    pub count_lte: Option<CompiledCountAggregate>,
    pub count_gte: Option<CompiledCountAggregate>,
    pub any_permanent: Option<Box<CompiledExistential>>,
    pub any_field_permanent: Option<Box<CompiledExistential>>,
    pub no_permanent: Option<Box<CompiledExistential>>,
    pub all_permanents: Option<Box<CompiledExistential>>,
    pub all_of: Vec<CompiledPredicate>,
    pub any_of: Vec<CompiledPredicate>,
    pub none_of: Vec<CompiledPredicate>,
    pub not: Option<Box<CompiledPredicate>>,
    pub has_alt_path: Option<String>,
    pub level_matches_aggregate: Option<(CompiledAggregateSelector, CompiledPlayerRef)>,
    pub materials_count_matches_aggregate: Option<(CompiledAggregateSelector, CompiledPlayerRef)>,
    pub self_digivolution_contains_name: Option<String>,
    /// Like `self_digivolution_contains_name` but scans ONLY the
    /// digivolution source cards beneath the carrier — the carrier's
    /// own top card is excluded. G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY.
    pub self_digivolution_sources_contain_name: Option<String>,
    pub self_digivolution_sources_trait_has: Option<String>,
    pub event_target_owner: Option<CompiledPlayerRef>,
    /// Event-target permanent color-set intersection test.
    /// G-EVENT-TARGET-COLOR.
    pub event_target_color_any_of: Option<Vec<CompiledColor>>,
    pub host_permanent_trait_has: Option<String>,
    pub trashed_source_trait_has: Option<String>,
    pub trashed_source_card_id_is: Option<String>,
    /// BeforePayCost cost-target sub-predicate. Evaluated as a `Card`
    /// subject against the cost target (`cost_target_card` on the read
    /// context). Fails when no cost target is active. Used by
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    pub cost_target: Option<Box<CompiledPredicate>>,
    /// True when the effect's `source_permanent` is one of the
    /// digivolve-target permanents on the read context's
    /// `cost_target_permanents`. Used to gate
    /// "When THIS Digimon would digivolve into ..." printed semantics.
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    pub source_is_cost_target_permanent: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledPermanentProperty {
    Level,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledDpConstraint {
    Literal(i32),
    Formula(CompiledFormula),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledReplacementCause {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
    Overclock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledEventCause {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledBindingCompare {
    Binding(String),
    Literal(i64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledBindingOwnerPredicate {
    pub binding: String,
    pub of: CompiledPlayerRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledCountAggregate {
    pub filter: Box<CompiledPredicate>,
    pub n: CompiledDpConstraint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledExistential {
    pub of: CompiledPlayerRef,
    pub predicate: CompiledPredicate,
}

/// Compiled form of `no_face_up_security_named`. Exactly one of
/// `card_number_is` / `name_is` is populated. G-PRED-NO-FACE-UP-SECURITY-NAMED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledFaceUpSecurityNamed {
    pub of: CompiledPlayerRef,
    pub card_number_is: Option<String>,
    pub name_is: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledPlayerRef {
    You,
    Opponent,
    Any,
    Active,
}

// ── Formulas ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, strum_macros::EnumDiscriminants)]
#[strum_discriminants(derive(strum_macros::EnumIter, Hash))]
#[strum_discriminants(name(CompiledFormulaDiscriminant))]
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
    AggregateScoped {
        selector: CompiledAggregateSelector,
        scope: CompiledPlayerRef,
    },
    BindingDp(String),
    BindingPlayCost(String),
    /// Effective DP of the effect's `source_permanent` (the carrier of
    /// the running effect). Unlike `BindingDp`, which reads a named
    /// `bind_as` binding, this reads `ctx.source_permanent` directly.
    /// Used by P-182's [When Digivolving] "delete 1 opp Digimon with as
    /// much or less DP as this Digimon". G-FORMULA-SOURCE-DP.
    SourceDp,
    /// Digivolution-card count (materials beneath the top card) of the
    /// effect's `source_permanent`. Sibling of `SourceDp`. Used by AD1-025's
    /// [On Play][When Digivolving] bounce-by-digivolution-card-count.
    /// G-FORMULA-SOURCE-MATERIAL-COUNT.
    SourceMaterialCount,
    SourceStackDpSum {
        target: String,
        filter: Option<Box<CompiledPredicate>>,
    },
    RawRust(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledPerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    SuspendedCount {
        of: CompiledPlayerRef,
        /// Exclude the effect's own source permanent from the count.
        #[serde(default)]
        exclude_source: bool,
    },
    DigivolutionColorCount,
    SameLevelPairsInSources,
    SharedTrashCount {
        bucket: Option<u32>,
    },
    CardCountInZone,
    CardCountInZoneScoped {
        zone: CompiledZone,
        of: CompiledPlayerRef,
    },
    FilteredCardCountInZoneScoped {
        zone: CompiledZone,
        of: CompiledPlayerRef,
        filter: Box<CompiledPredicate>,
    },
    DistinctColorsCountScoped {
        zone: CompiledZone,
        of: CompiledPlayerRef,
        filter: Option<Box<CompiledPredicate>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledCountBound {
    Literal(u8),
    Formula(CompiledFormula),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledAggregateSelector {
    LowestDp,
    HighestDp,
    LowestLevel,
    HighestLevel,
    FewestMaterials,
    /// Lowest printed play cost among the candidate set.
    /// G-PLAY-COST-AGGREGATE.
    LowestPlayCost,
}

/// Value carried by `add_dp_modifier` / `add_modifier` steps. Phase 2f2 Task 1
/// generalizes this from a bare `i32` so card text like Susanoomon's
/// "+2000 DP per material on this Digimon" can express the value as a
/// formula. The runtime evaluator (Phase 2f2 Task 2) and step-runner wiring
/// (Task 3) consume this enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledModifierValue {
    Literal(i32),
    Formula(CompiledFormula),
}

// ── Clauses ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledClause {
    Triggered(CompiledTriggeredClause),
    Declarative(CompiledDeclarativeClause),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledDeclarativeClause {
    Aura {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        target: CompiledPredicate,
        target_player: Option<CompiledPlayerRef>,
        dp_modifier: Option<i32>,
        dp_modifier_fn: Option<CompiledFormula>,
        /// Flat `Security A. ±N` grant. Track H §1.
        /// Lowers to `ModifierType::SecurityAttackChange` with the literal
        /// delta as `value`; the security-resolution loop reads the sum
        /// at consult time (`combat.rs:2326`).
        security_attack: Option<i32>,
        security_attack_fn: Option<CompiledFormula>,
        grant_keyword: Option<CompiledGrantKeywordValue>,
        modifier: Option<String>,
        /// Track H §4 — install-once continuous gate. When present, the
        /// lowered `Effect` installs its modifier(s) with
        /// `Expiry::UntilCondition` carrying this predicate. Eviction is
        /// final per PR #458 (`false → true` does not re-install).
        while_condition: Option<CompiledPredicate>,
        /// PUPPETS-G008 — when true, the lowered effect calls
        /// `EffectBuilder::applies_to_opponent_security_dp()` so the
        /// `dp_modifier` rides as an attacker-side security-DP adjustment
        /// during the security battle (rather than as a battle-area aura).
        applies_to_opponent_security_dp: bool,
        applies_to_own_security_dp: bool,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    CostReduction {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        reduction_timing: Option<String>,
        when_playing_this: bool,
        when_any_ally_played: Option<CompiledPredicate>,
        /// Digivolution-target-keyed trigger — fires only for a DIGIVOLVE
        /// cost whose target card matches this predicate.
        /// `G-COST-REDUCTION-DIGIVOLVE-INTO`.
        when_any_ally_digivolves_into: Option<CompiledPredicate>,
        condition: Option<CompiledPredicate>,
        optional: bool,
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
        optional: bool,
        once_per_turn: bool,
        process: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Partition {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        sources: Vec<CompiledPredicate>,
        exclude_cause: Vec<String>,
        process: Vec<CompiledStep>,
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
        overclock_cost_filter: Option<CompiledPredicate>,
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
    LinkRequirement {
        scope: CompiledScope,
        cost: u16,
        filter: CompiledPredicate,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    FloodGate {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        modifier: String,
        target: Option<CompiledPredicate>,
        target_player: Option<CompiledPlayerRef>,
        expiry: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CompiledScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
    Linked,
    Security,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum_macros::EnumIter,
)]
pub enum CompiledTiming {
    OnPlay,
    WhenDigivolving,
    WhenAttacking,
    EndOfAttack,
    EndOfBattle,
    OnAttack,
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
    /// Sibling of `BeforePayCost` for observer-style triggered bodies
    /// (e.g. "When this would DNA digivolve into a green Digimon card,
    /// gain 1 memory."). Lowers to `EffectTiming::BeforePayCostObserve`
    /// and fires its `process` body at the same dispatch point as
    /// `BeforePayCost` cost reducers without coupling to cost-reduction
    /// fields. G-BEFORE-PAY-COST-GAIN-MEMORY (Phase 2 Track H closure).
    BeforePayCostObserve,
    Delayed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledGrantKeywordValue {
    pub keyword: String,
    pub value: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledEffectSourceKind {
    Digimon,
    Tamer,
    Option,
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledEffectController {
    Any,
    Opponent,
    Own,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledFieldSelector {
    LowestDp,
    HighestDp,
    /// Lowest printed play cost among the candidate permanents.
    /// G-PLAY-COST-AGGREGATE.
    LowestPlayCost,
    /// Highest printed play cost among the candidate permanents.
    /// G-HIGHEST-PLAY-COST-SELECTOR.
    HighestPlayCost,
}

// ── Steps ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CompiledAttackCostUpgrade {
    pub dp: i32,
    pub security_attack: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, strum_macros::EnumDiscriminants)]
#[strum_discriminants(derive(strum_macros::EnumIter, Hash))]
#[strum_discriminants(name(CompiledStepDiscriminant))]
pub enum CompiledStep {
    GainMemory(i32),
    LoseMemory(i32),
    SetMemory(i32),
    /// Phase 2 Track F (G-DSL-GAIN-MEMORY-FN): formula-valued memory
    /// mutation. Evaluated at resolution time via `formula_eval` and
    /// fed to the engine's signed `add_memory` helper. Mirror of the
    /// literal `GainMemory(i32)` with runtime-computed magnitude.
    GainMemoryFn {
        formula: CompiledFormula,
    },
    LoseMemoryFn {
        formula: CompiledFormula,
    },
    Draw {
        of: CompiledPlayerRef,
        count: u8,
    },
    TrashFromTop {
        of: CompiledPlayerRef,
        count: u8,
    },
    AddToHandFromDeck {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    AddToHandFromTrash {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    AddToHandFromSecurity {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    /// Play a bound card from the security stack without paying its cost.
    /// G-PLAY-SELECTED-SECURITY-CARD.
    PlaySecurityCard {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    /// Trash a bound card from a player's security stack.
    /// G-TRASH-SELECTED-SECURITY.
    TrashSelectedSecurity {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    AddTopSecurityToHand {
        of: CompiledPlayerRef,
    },
    MayAddTopSecurityToHand {
        of: CompiledPlayerRef,
    },
    AddToHandFromReveal {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    AddThisOptionToHand,
    TrashFromHandByIndex {
        of: CompiledPlayerRef,
        hand_index: CompiledBindingRef,
    },
    TrashFromReveal {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    ReturnToDeckFromReveal {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
        position: CompiledStackPosition,
    },
    ShuffleDeck {
        of: CompiledPlayerRef,
    },
    ShuffleSecurity {
        of: CompiledPlayerRef,
    },
    RevealTopDeck {
        of: CompiledPlayerRef,
        count: u8,
        zone: Option<CompiledZone>,
        bind_as: Option<String>,
    },
    PlaceRemainderOnDeck {
        of: CompiledPlayerRef,
        position: CompiledStackPosition,
    },
    /// Phase 2 Track E (2026-05-17): pick one revealed card matching `filter`
    /// and route it to `destination`. Lowers as a single selection install
    /// whose callback routes the picked card to the typed destination. The
    /// optional `bind_as` records the picked CardHandle for downstream
    /// reference.
    ChooseFromReveal {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        destination: CompiledRevealDestination,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    /// Phase 2 Track E (2026-05-17): place the entire reveal pool onto the
    /// controller's deck. When `destinations.len() == 1` behaves like
    /// `place_remainder_on_deck`. When `destinations.len() == 2` surfaces a
    /// player effect-choice over the entries before placing. Ordering is
    /// always exposed via `select_ordered_permutation` (Working Rule §17).
    OrderRemainder {
        of: CompiledPlayerRef,
        destinations: Vec<CompiledRemainderDestination>,
        prompt: Option<String>,
        prompt_key: Option<String>,
    },
    DeletePermanent {
        target: CompiledBindingRef,
    },
    DeleteBoundPermanents {
        binding: String,
    },
    TrashBreedingPermanent {
        target: CompiledBindingRef,
    },
    ReturnToHand {
        target: CompiledBindingRef,
    },
    ReturnToDeck {
        target: CompiledBindingRef,
        position: CompiledStackPosition,
        include_sources: bool,
    },
    Suspend {
        target: CompiledBindingRef,
    },
    Unsuspend {
        target: CompiledBindingRef,
    },
    DeDigivolve {
        target: CompiledBindingRef,
        amount: Option<u8>,
        amount_fn: Option<CompiledFormula>,
        stop_at_level: Option<u8>,
    },
    PlaceOnSecurity {
        of: CompiledPlayerRef,
        source: CompiledBindingRef,
        position: CompiledStackPosition,
        face_up: bool,
    },
    PlayToken {
        controller: CompiledPlayerRef,
        token_name: String,
        bind_as: Option<String>,
    },
    PlaceAsBottomSource {
        source: CompiledBindingRef,
        target: CompiledBindingRef,
        face_down: bool,
    },
    /// Phase 2 Track F (2026-05-17) — deterministic "top stacked card →
    /// bottom" source-stack rotation. Closes G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM
    /// for BT23-008 / BT23-018 / BT24-079 / BT24-082-shape costs that read
    /// "By placing this Digimon's top stacked card as its bottom digivolution
    /// card …". The verb does NOT surface a player choice — printed text
    /// pins the top source as the moved card.
    PlaceTopSourceAsBottom {
        target: CompiledBindingRef,
    },
    TrashTopSource {
        target: CompiledBindingRef,
    },
    TrashBottomSources {
        target: CompiledBindingRef,
        count: u8,
    },
    TrashAllSources {
        target: CompiledBindingRef,
    },
    /// Pick one of `of`'s Tamers that carries a face-down stash, then trash
    /// that Tamer's bottom face-down digivolution source. Compiled from the
    /// `trash_bottom_face_down_source_under_tamer` verb; used as an activation
    /// cost by BEATBREAK / DATA SQUAD cards.
    TrashBottomFaceDownSourceUnderTamer {
        of: CompiledPlayerRef,
    },
    Hatch {
        of: CompiledPlayerRef,
    },
    MoveFromBreeding {
        of: CompiledPlayerRef,
    },
    PlayFromHand {
        of: CompiledPlayerRef,
        hand_index: CompiledBindingRef,
        cost_delta: Option<CompiledCostDelta>,
    },
    PlayFromHandFree {
        of: CompiledPlayerRef,
        hand_index: CompiledBindingRef,
        /// Bind the just-played permanent handle for use in later steps.
        /// `None` preserves prior behavior (no binding insert).
        /// G-PLAY-FROM-HAND-FREE-BIND-AS (Phase 2 Track H closure).
        bind_as: Option<String>,
    },
    UseOptionFromHand {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        use_cost_lte_opponent_memory: bool,
        optional: bool,
        prompt: Option<String>,
    },
    PlayFromRevealedFree {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
        /// Bind the just-played permanent handle for use in later steps.
        bind_as: Option<String>,
    },
    PlayFromTrash {
        of: CompiledPlayerRef,
        trash_index: CompiledBindingRef,
        cost_delta: Option<CompiledCostDelta>,
    },
    PlayFromTrashFree {
        of: CompiledPlayerRef,
        trash_index: CompiledBindingRef,
        /// PUPPETS-G030 — when `true`, the played Digimon's own `[On Play]`
        /// effects are skipped for this play event only (BT5-106 [Security]).
        suppress_on_play: bool,
    },
    /// PUPPETS-G014 — play a `select_union_zone`-bound card for free from its
    /// true origin zone (hand, trash, or material). `binding` names a `select_union_zone`
    /// `bind_as`; the origin zone is carried in the binding value.
    PlayUnionBoundFree {
        binding: String,
        /// Bind the just-played permanent handle for use in later steps.
        bind_as: Option<String>,
        suppress_on_play: bool,
    },
    TrashUnionBound {
        binding: String,
    },
    PlayFromSecurity,
    PlayFromMaterials {
        target: CompiledBindingRef,
        source_index: CompiledBindingRef,
        cost_delta: Option<CompiledCostDelta>,
        suppress_on_play: bool,
        bind_as: Option<String>,
    },
    PlaySelectedSourcesFree {
        source_refs: String,
    },
    EffectInitiatedDigivolve {
        target: CompiledBindingRef,
        from_hand: CompiledBindingRef,
        cost: CompiledCostDelta,
        ignore_requirements: bool,
    },
    EffectInitiatedDnaDigivolve {
        target_a: CompiledBindingRef,
        target_b: CompiledBindingRef,
        from_hand: CompiledBindingRef,
        cost: CompiledCostDelta,
        ignore_requirements: bool,
    },
    /// DNA digivolve where `target` is a battle-area permanent and
    /// `hand_partner` is the second DNA material drawn from hand; the merged
    /// permanent is topped with `from_hand` (the result card). BT17-095 B.
    EffectInitiatedDnaDigivolveHandPartner {
        target: CompiledBindingRef,
        hand_partner: CompiledBindingRef,
        from_hand: CompiledBindingRef,
        cost: CompiledCostDelta,
        ignore_requirements: bool,
    },
    TrashTopSecurity {
        of: CompiledPlayerRef,
        /// Number of top security cards to trash, as a run-time formula.
        /// `None` trashes exactly one (the historical behavior).
        count: Option<CompiledFormula>,
    },
    TrashBottomSecurity {
        of: CompiledPlayerRef,
    },
    AddBottomSecurityToHand {
        of: CompiledPlayerRef,
    },
    TrashTopSecurityAndCancelReplacement {
        of: CompiledPlayerRef,
    },
    BounceSelf,
    PlaceSelfAtSecurity {
        position: CompiledStackPosition,
        face_up: bool,
    },
    PlaceSelfOptionAtSecurity {
        position: CompiledStackPosition,
        face_up: bool,
    },
    PlacePermanentBottomSecurityAndCancelReplacement {
        of: CompiledPlayerRef,
        target: CompiledBindingRef,
    },
    PlacePermanentOnSecurity {
        of: CompiledPlayerRef,
        target: CompiledBindingRef,
        position: CompiledStackPosition,
        face_up: bool,
    },
    PlacePermanentOnSecurityAndHandleReplacement {
        of: CompiledPlayerRef,
        target: CompiledBindingRef,
        position: CompiledStackPosition,
        face_up: bool,
    },
    PlacePermanentOnSecurityObserved {
        of: CompiledPlayerRef,
        target: CompiledBindingRef,
        position: CompiledStackPosition,
        face_up: bool,
        include_sources: bool,
    },
    SecurityPlaceStackedCard {
        carrier: CompiledBindingRef,
        source: Option<CompiledBindingRef>,
        source_index_from_top: Option<u8>,
        of: CompiledPlayerRef,
        position: CompiledStackPosition,
        face_up: bool,
    },
    SecurityPlaceTopStackedCard {
        carrier: CompiledBindingRef,
        of: CompiledPlayerRef,
        position: CompiledStackPosition,
        face_up: bool,
    },
    ReturnAllTrashToDeckBottom {
        of: CompiledPlayerRef,
    },
    /// Move a bound card list out of trash to the deck. `to_top` selects the
    /// deck top (the next card drawn) over the bottom.
    /// G-ZONE-TRASH-TO-DECK / G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
    ReturnTrashListToDeckBottom {
        of: CompiledPlayerRef,
        cards: CompiledBindingRef,
        to_top: bool,
    },
    /// Move a single selected trash card to the TOP of its owner's deck.
    /// `of` identifies whose trash the card is in; the card returns to its
    /// OWNER's deck. G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
    MoveTrashCardToDeckTop {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    TrashTopNDigivolutionCardsOfEach {
        of: CompiledPlayerRef,
        n: CompiledFormula,
    },
    TrashOpponentHandToCount {
        opponent: CompiledPlayerRef,
        target_count: CompiledFormula,
    },
    SearchOwnSecurityStack {
        filter: Box<CompiledPredicate>,
        prompt: String,
        bind_as: Option<String>,
        optional: bool,
        on_select: Vec<CompiledStep>,
        on_no_match: Option<Vec<CompiledStep>>,
    },
    Recover {
        of: CompiledPlayerRef,
        count: u8,
    },
    MarkSecurityFaceUp {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
    },
    FlipSecurityFaceUp {
        of: CompiledPlayerRef,
    },
    AddDpModifier {
        target: CompiledBindingRef,
        value: CompiledModifierValue,
        expiry: String,
    },
    AddModifier {
        target: CompiledModifierTarget,
        modifier: String,
        value: CompiledModifierValue,
        expiry: String,
    },
    AddPlayerModifier {
        target_player: CompiledPlayerRef,
        modifier: String,
        value: i32,
        expiry: String,
    },
    GrantKeyword {
        target: CompiledBindingRef,
        keyword: String,
        expiry: String,
        value: Option<i32>,
    },
    GrantEffectImmunity {
        target: CompiledBindingRef,
        source_kind: CompiledEffectSourceKind,
        source_controller: CompiledEffectController,
        expiry: String,
    },
    /// PUPPETS-G024 — install the narrow opponent-effect protection
    /// bundle (opponent-scoped ImmuneFromDPMinus + opponent-scoped
    /// CannotBeDeDigivolved) on `target`.
    GrantNarrowOpponentEffectProtection {
        target: CompiledBindingRef,
        expiry: String,
    },
    /// Track H §3 — install a granted triggered effect on each
    /// permanent matching `target`, or on one bound permanent.
    /// DCGO `AddSkillClass.cs` analog.
    GrantTriggeredEffect {
        target: CompiledModifierTarget,
        timing: String,
        expiry: String,
        body: Vec<CompiledStep>,
    },
    SelectOwnPermanent {
        filter: CompiledPredicate,
        bind_as: Option<String>,
        selector: Option<CompiledFieldSelector>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectOpponentPermanent {
        filter: CompiledPredicate,
        bind_as: Option<String>,
        selector: Option<CompiledFieldSelector>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectAnyPermanent {
        filter: CompiledPredicate,
        bind_as: Option<String>,
        selector: Option<CompiledFieldSelector>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectDnaPair {
        left_filter: CompiledPredicate,
        right_filter: CompiledPredicate,
        bind_left_as: String,
        bind_right_as: String,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectHand {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
        /// When `optional && cost`, declining this prompt aborts the rest of
        /// the clause body via `Game::dsl_clause_aborted`. See
        /// `digimon_dsl::step::SelectZoneArgs::cost` for the printed-text
        /// pattern. Default `false` keeps the historical "decline runs tail"
        /// behavior for non-cost optional picks.
        #[serde(default)]
        cost: bool,
    },
    SelectTrash {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
        /// See `SelectHand::cost`.
        #[serde(default)]
        cost: bool,
    },
    SelectMaterial {
        of_permanent: CompiledBindingRef,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    /// Count-capped / name-unique multi-pick over a carrier permanent's
    /// digivolution-source stack — the batch sibling of `SelectMaterial`.
    /// Lowers to `EffectContext::select_count_capped_multi` with
    /// `CountCappedZone::Material`; `uniqueness` maps to `DistinctByMode`.
    SelectMaterials {
        of_permanent: CompiledBindingRef,
        max: CompiledCountBound,
        filter: CompiledPredicate,
        uniqueness: Option<CompiledDistinctBy>,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional_zero: bool,
    },
    SelectOwnSources {
        target: Option<CompiledBindingRef>,
        filter: CompiledPredicate,
        min: u8,
        max: u8,
        bind_as: Option<String>,
        prompt: String,
        then: Vec<CompiledStep>,
    },
    /// Opponent-side mirror of `SelectOwnSources`. Candidate set drawn from the
    /// OPPONENT's battle-area digivolution-source stacks.
    /// G-SELECT-OPPONENT-SOURCES.
    SelectOpponentSources {
        target: Option<CompiledBindingRef>,
        filter: CompiledPredicate,
        min: u8,
        max: u8,
        bind_as: Option<String>,
        prompt: String,
        then: Vec<CompiledStep>,
    },
    SelectOpponentDpBudget {
        dp_budget: CompiledFormula,
        min_picks: u8,
        /// Per-candidate predicate; `CompiledPredicate::default()` accepts all.
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        then: Vec<CompiledStep>,
    },
    /// Play-cost-budget analog of `SelectOpponentDpBudget`.
    /// G-MULTI-SELECT-OPP-PLAY-COST-SUM.
    SelectOpponentPlayCostBudget {
        play_cost_budget: i32,
        min_picks: u8,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        then: Vec<CompiledStep>,
    },
    SelectOwnBreedingPermanent {
        bind_as: Option<String>,
        prompt: String,
        optional: bool,
        /// Predicate the breeding permanent must satisfy before the
        /// selection prompt opens. `CompiledPredicate::default()` is the
        /// "accept any breeding permanent" carrier matching the historical
        /// behavior; a populated predicate filters by name, level, etc.
        filter: CompiledPredicate,
        then: Vec<CompiledStep>,
    },
    TrashSelectedSources {
        source_refs: String,
    },
    /// G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME — return each
    /// `SelectOwnSources`-bound digivolution source card to its owner's hand
    /// (mirror of `TrashSelectedSources`, hand destination instead of trash).
    ReturnSelectedSourcesToHand {
        source_refs: String,
    },
    BindPermanentProperty {
        from: CompiledBindingRef,
        property: CompiledPermanentProperty,
        bind_as: String,
    },
    SelectReveal {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectRevealBuckets {
        from: String,
        buckets: Vec<CompiledRevealBucket>,
        no_duplicate_cards: bool,
        prompt: Option<String>,
    },
    SelectSecurity {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
    },
    SelectUnionZone {
        of: CompiledPlayerRef,
        zones: Vec<CompiledZone>,
        material_of: Option<CompiledBindingRef>,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
        /// See `SelectHand::cost`. Prefer `then:` for steps that should only
        /// run on accept when the cost-pay is local; use `cost: true` when the
        /// printed text is "By picking X, do Y" where Y AND any further clause
        /// steps must abort on decline.
        #[serde(default)]
        cost: bool,
        then: Vec<CompiledStep>,
    },
    SelectOrderedPermutation {
        items: CompiledBindingRef,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
    },
    SelectCountCappedMulti {
        of: CompiledPlayerRef,
        zone: CompiledZone,
        max: CompiledCountBound,
        /// Minimum required picks. G-SELECT-MULTI-MIN.
        min: u8,
        filter: CompiledPredicate,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
        optional_zero: bool,
        distinct_by: Option<CompiledDistinctBy>,
    },
    SelectEffectChoice {
        labels: Vec<String>,
        bind_as: Option<String>,
        prompt: String,
        prompt_key: Option<String>,
    },
    AsSelectingPlayer {
        of: CompiledPlayerRef,
        body: Vec<CompiledStep>,
    },
    If {
        condition: CompiledPredicate,
        then: Vec<CompiledStep>,
        else_branch: Vec<CompiledStep>,
    },
    ForEach {
        over: CompiledPredicate,
        bind_as: String,
        body: Vec<CompiledStep>,
    },
    PerSelected {
        selection: String,
        bind_as: String,
        body: Vec<CompiledStep>,
    },
    ScheduleDelayed {
        when: CompiledTiming,
        body: Vec<CompiledStep>,
    },
    /// PUPPETS-G003 — schedule the permanent named by `binding` for deletion
    /// at the end of the designated turn boundary, keyed to its stable
    /// provenance identity. `binding` names a `bind_as` from a preceding
    /// free-play step. `at_opponents_turn` selects the opponent-turn-end drain
    /// (P-165); `false` (default) selects the your-turn-end drain (EX11-022,
    /// EX11-061).
    ScheduleDeletePlayedAtTurnEnd {
        binding: String,
        at_opponents_turn: bool,
    },
    PlaceSelfAsDelayOption,
    LinkToOwnDigimon {
        optional: bool,
        free: bool,
        filter: CompiledPredicate,
    },
    Optional(Vec<CompiledStep>),
    Battle {
        attacker: CompiledBindingRef,
        defender: CompiledBindingRef,
    },
    MayAttackNow {
        attacker: CompiledBindingRef,
        targets: CompiledAttackTargetSpec,
        without_suspending: bool,
        optional: bool,
        prompt: Option<String>,
        cost_upgrade: Option<CompiledAttackCostUpgrade>,
    },
    ForceAttack {
        attacker: CompiledBindingRef,
        targets: CompiledAttackTargetSpec,
        without_suspending: bool,
        prompt: Option<String>,
        cost_upgrade: Option<CompiledAttackCostUpgrade>,
    },
    RedirectAttackTarget {
        new_target: Option<CompiledBindingRef>,
        player: Option<CompiledPlayerRef>,
        targets: CompiledAttackTargetSpec,
        optional: bool,
        prompt: Option<String>,
    },
    CancelAttack,
    OpenCounterWindow,
    RefireEffect {
        source: CompiledBindingRef,
        timing: String,
        optional: bool,
    },
    EndAttack {
        enabled: bool,
    },
    CancelReplacement,
    HandleReplacement,
    RedirectReplacement {
        zone: CompiledZone,
    },
    SubstituteReplacement {
        subject: CompiledBindingRef,
    },
    RawRust {
        fn_name: String,
        consumes: Vec<String>,
        binds: Vec<String>,
    },
    /// Phase 2 Track B — declarative activation-cost step for triggered
    /// abilities. The DSL lowering MUST lift this step out of the
    /// process body and bind it to `EffectBuilder::activation_cost(...)`
    /// at clause-construction time; the validator rejects `activation_cost`
    /// appearing anywhere except as the first step of a triggered clause
    /// body. This variant is therefore unreachable at runtime — present
    /// for variant-coverage and lowering-time inspection only.
    ActivationCost {
        kind: CompiledActivationCostKind,
    },
    /// G-COST-REDUCE-ALLY-DIGIVOLVE — install a player-scoped one-shot
    /// future-digivolve cost reducer. BT3-103 Hidden Potential Discovered!.
    ArmDigivolveCostReducer {
        amount: i32,
        single_fire: bool,
        target_color: Option<CompiledColor>,
        suspend_cost: bool,
    },
    /// G-DSL-EOT-DNA-INLINE — inline DNA digivolve choice at trigger fire.
    ///
    /// Surfaces the printed "[End of Your Turn] This Digimon and any of your
    /// other Digimon may DNA digivolve into a Digimon card in the hand"
    /// pattern (BT12-021/-047, BT17-007/-019, BT22-008/-017) AT the trigger
    /// drain, not as a registration for a later turn.
    ///
    /// The step orchestrates: (1) optional accept/decline prompt, (2)
    /// partner permanent selection over own field (excluding anchor),
    /// (3) target card selection from controller's hand, (4) call to the
    /// `effect_initiated_dna_digivolve` engine primitive.
    ///
    /// `anchor` is the source DNA material (typically the trigger's source
    /// permanent — `CompiledBindingRef::Source`). `partner_filter` constrains
    /// the OTHER DNA material on own field (the engine excludes anchor as a
    /// hard invariant of the verb; YAML need not repeat the exclusion).
    /// `target_filter` constrains the Digimon card in the controller's hand
    /// that the merged permanent is topped with.
    ///
    /// `cost` is the printed memory cost (zero for all 6 currently-known
    /// affected cards). `ignore_requirements` bypasses normal digivolution
    /// affordability checks (true for the 6 affected cards' EoT "may DNA
    /// digivolve"). `optional` controls the outer accept/decline gate
    /// (true for printed "may"). `prompt` overrides the default accept
    /// prompt copy.
    MayDnaDigivolveNow {
        anchor: CompiledBindingRef,
        partner_filter: CompiledPredicate,
        target_filter: CompiledPredicate,
        cost: u16,
        ignore_requirements: bool,
        optional: bool,
        prompt: Option<String>,
    },
}

/// Concrete activation-cost shapes recognized by the DSL. Extensible —
/// new printed cost shapes (return-self-to-trash, return-self-to-hand)
/// can be added without changing the queue-side dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledActivationCostKind {
    /// "by suspending this Tamer / Digimon ..." — pays the cost by
    /// suspending the source permanent. Fails if the source is already
    /// suspended; failure consumes the OPT slot.
    SuspendSelf,
    /// "by returning this Tamer to the bottom of the deck ..." — pays
    /// the cost by moving the source permanent's top card to its
    /// owner's deck bottom (digivolution sources trashed per standard
    /// rules). Fails if the source has already left the field.
    ReturnSelfToDeckBottom,
    /// "by trashing this card ..." — pays the cost by trashing the source
    /// permanent (a `<Delay>` Option). Fails if the source has already left
    /// the field. G-ACTIVATION-COST-TRASH-SELF.
    TrashSelf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledRevealBucket {
    pub bind_as: String,
    pub filter: Option<CompiledPredicate>,
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledAttackTargetSpec {
    Any,
    Player,
    Digimon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Top card of a player's deck — resolves to `CardSourceRef::DeckTop`.
    /// Card-source binding only (never a permanent/card handle).
    DeckTop(CompiledPlayerRef),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledModifierTarget {
    Binding(CompiledBindingRef),
    Filter(CompiledPredicate),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledCostDelta {
    Free,
    Printed,
    Literal(i32),
    Reduce(i32),
    /// Formula-valued cost reduction — the evaluated integer is subtracted
    /// from the printed play cost at resolution time. G-FORMULA-COST-DELTA.
    ReduceFn(CompiledFormula),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledStackPosition {
    Top,
    Bottom,
    Random,
}

/// Phase 2 Track E (2026-05-17): compiled form of `RevealDestination` — the
/// typed routing for the `choose_from_reveal` step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledRevealDestination {
    Hand,
    DeckTop,
    DeckBottom,
    BottomSourceOf(CompiledBindingRef),
}

/// Phase 2 Track E (2026-05-17): compiled form of `RemainderDestination`
/// — only `DeckTop` / `DeckBottom` are meaningful for `order_remainder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledRemainderDestination {
    DeckTop,
    DeckBottom,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_card() -> CompiledCard {
        CompiledCard {
            card: "ST2-13".into(),
            name: "Hammer Spark".into(),
            kind: CompiledCardKind::Option,
            level: None,
            color: vec![CompiledColor::Blue],
            cost: Some(0),
            dp: None,
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            digixros_aliases: vec![],
            also_treated_as: vec![],
            dual: None,
            use_requirement: None,
            alt_paths: vec![],
            effects: vec![CompiledClause::Triggered(CompiledTriggeredClause {
                when: vec![CompiledTiming::MainFromHand],
                scope: CompiledScope::FaceUp,
                active_when: None,
                condition: None,
                optional: false,
                once_per_turn: false,
                max_per_turn: None,
                process: vec![CompiledStep::GainMemory(1)],
                summary: None,
                summary_key: None,
            })],
        }
    }

    #[test]
    fn compiled_card_round_trips_through_bincode() {
        let original = sample_card();
        let bytes = bincode::serialize(&original).expect("bincode serialize");
        let reparsed: CompiledCard = bincode::deserialize(&bytes).expect("bincode deserialize");
        assert_eq!(original, reparsed);
    }

    #[test]
    fn compiled_card_with_recursive_predicate() {
        // Exercise the recursive Box<CompiledPredicate> path that rkyv
        // couldn't handle.
        let pred = CompiledPredicate {
            any_of: vec![
                CompiledPredicate {
                    name_contains: Some("Greymon".into()),
                    ..Default::default()
                },
                CompiledPredicate {
                    name_contains: Some("Garurumon".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let card = CompiledCard {
            card: "X-1".into(),
            name: "Test".into(),
            kind: CompiledCardKind::Digimon,
            level: Some(6),
            color: vec![CompiledColor::Red],
            cost: Some(11),
            dp: Some(12000),
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            digixros_aliases: vec![],
            also_treated_as: vec![],
            dual: None,
            use_requirement: None,
            alt_paths: vec![],
            effects: vec![CompiledClause::Triggered(CompiledTriggeredClause {
                when: vec![CompiledTiming::OnPlay],
                scope: CompiledScope::FaceUp,
                active_when: None,
                condition: Some(pred),
                optional: false,
                once_per_turn: false,
                max_per_turn: None,
                process: vec![],
                summary: None,
                summary_key: None,
            })],
        };
        let bytes = bincode::serialize(&card).unwrap();
        let reparsed: CompiledCard = bincode::deserialize(&bytes).unwrap();
        assert_eq!(card, reparsed);
    }
}
