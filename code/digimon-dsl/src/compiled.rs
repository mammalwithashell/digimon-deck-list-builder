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
    /// Digimon-face behavioral clauses (`G-DSL-DUAL-PER-FACE-EFFECTS`).
    /// Lowered with the Digimon identity — natural timings.
    #[serde(default)]
    pub effects: Vec<CompiledClause>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledDualOption {
    pub use_cost: u16,
    pub colors: Vec<CompiledColor>,
    pub effect_text: String,
    pub security_text: String,
    /// Option-face keywords. `arts_digivolve: true` on the spec compiles into
    /// an `ArtsDigivolve` entry here, so the engine's existing keyword check
    /// (`pending_option_can_arts_digivolve`) needs no extra wiring.
    pub keywords: Vec<String>,
    pub use_requirement: Option<Box<CompiledPredicate>>,
    /// Option-face behavioral clauses — the BEATBREAK `[Main]` body. Lowered
    /// with the Dual identity so `when: main` → `OptionMain`
    /// (`G-DSL-DUAL-PER-FACE-EFFECTS`).
    #[serde(default)]
    pub effects: Vec<CompiledClause>,
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

/// Compiled source zone for the `LinkCards` step. Mirrors
/// `step::LinkCardSourceZone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledLinkSourceZone {
    Hand,
    Trash,
    SelfSources,
    OwnDigimonSources,
    /// Gap 3b — the in-play Option links itself onto the host.
    SelfOption,
}

/// Compiled attach target for the `LinkCards` step. Mirrors
/// `step::LinkCardsTo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledLinkTo {
    SelfPermanent,
    OwnDigimon,
}

/// Compiled count bound for the `LinkCards` step. `UpTo` makes every pick
/// declinable; `Exactly` keeps picking until N picks or no candidates remain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledLinkCount {
    Exactly(u8),
    UpTo(u8),
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
    /// Gap 4 (BT18-065) — conditionally-enabled extra DigiXros material zones,
    /// each gated by a play-start predicate. See
    /// [`CompiledConditionalMaterialZone`].
    #[serde(default)]
    pub extra_material_zones: Vec<CompiledConditionalMaterialZone>,
}

/// A conditionally-enabled DigiXros material origin zone (Gap 4). When
/// `condition` holds at transaction-build time, `zone` is added to the
/// transaction's allowed material zones. Lowered from
/// `alt_path::ConditionalMaterialZoneSpec`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledConditionalMaterialZone {
    pub zone: CompiledZone,
    pub condition: CompiledPredicate,
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
    /// Cast-time stack construction (BT15-102). See
    /// `alt_path::AltPathKind::CastTimeAssembly`. Appended at the tail so
    /// bincode variant indices of prior kinds are stable.
    CastTimeAssembly,
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
            CompiledAltPathKind::CastTimeAssembly => "cast_time_assembly",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledMaterial {
    pub filter: CompiledPredicate,
    pub repeat: Option<CompiledRepeat>,
    pub distinct_by: Option<CompiledDistinctBy>,
    pub zones: Vec<CompiledZone>,
    pub cost_delta: Option<i16>,
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
    /// Subject level `<=` the literal bound to this binding. Sibling of
    /// `level_eq_binding`. `#[serde(default)]` keeps older packs loadable.
    #[serde(default)]
    pub level_lte_binding: Option<String>,
    /// Subject level `>=` the literal bound to this binding. Sibling of
    /// `level_eq_binding`.
    #[serde(default)]
    pub level_gte_binding: Option<String>,
    pub level_lte: Option<CompiledDpConstraint>,
    pub level_gte: Option<CompiledDpConstraint>,
    pub color_is: Option<CompiledColor>,
    pub color_only: Option<Vec<CompiledColor>>,
    pub color_matches_any_field_digimon: Option<CompiledPlayerRef>,
    pub color_matches_binding: Option<String>,
    /// G-RETURNED-CARD-COLOR-BINDING — true when the candidate shares a color
    /// with a card this effect returned to deck (`returned_to_deck` log).
    pub color_matches_returned_card: Option<bool>,
    /// G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK — true when the candidate shares a
    /// color with the effect carrier's NON-FLIPPED digivolution-source color set
    /// (DCGO `DigivolutionCards.Filter(!IsFlipped).SelectMany(CardColors)
    /// .Distinct()`). Driver EX9-074 Kimeramon.
    pub color_matches_own_source_stack: Option<CompiledSourceStackScope>,
    pub trait_has: Option<String>,
    /// Substring sibling of `trait_has` — matches when any subject trait
    /// CONTAINS this token (case-insensitive). Mirrors DCGO
    /// `ContainsTraits`. G-DSL-TRAIT-CONTAINS-SUBSTRING.
    pub trait_contains: Option<String>,
    pub form_is: Option<String>,
    pub attribute_is: Option<String>,
    pub name_is: Option<String>,
    pub name_contains: Option<String>,
    /// Case-insensitive substring scan against the candidate card's
    /// concatenated printed text (`effect_text` + `inherited_text` +
    /// `security_text`). G-DSL-PREDICATE-TEXT-CONTAINS.
    pub effect_text_contains: Option<String>,
    /// Whole-card "[X] in its text" scan — the broad DCGO
    /// `CardSource.HasText` surface: name + also-treated-as / DigiXros
    /// aliases + traits (Type line) + all printed text (both dual faces).
    /// `#[serde(default)]` keeps older serialized packs deserializable.
    /// G-DSL-IN-TEXT-CONTAINS.
    #[serde(default)]
    pub in_text_contains: Option<String>,
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
    /// `_eq` completions for the uniform comparator (unify-dsl-scalar-and-comparators
    /// §2.4) — the legacy flat surface lacked `_eq` for these metrics; the
    /// canonical `<metric>: { op: eq, value }` lowers here. `#[serde(default)]`
    /// keeps older serialized packs deserializable.
    #[serde(default)]
    pub play_cost_eq: Option<CompiledDpConstraint>,
    /// G-PLAY-OR-USE-COST-LTE: max(play_cost, option_use_cost) <= N.
    #[serde(default)]
    pub play_or_use_cost_lte: Option<CompiledDpConstraint>,
    pub can_digivolve_from_source: Option<bool>,
    pub dp_eq: Option<CompiledDpConstraint>,
    pub dp_lte: Option<CompiledDpConstraint>,
    pub dp_gte: Option<CompiledDpConstraint>,
    pub stack_size_lte: Option<CompiledDpConstraint>,
    pub stack_size_gte: Option<CompiledDpConstraint>,
    #[serde(default)]
    pub stack_size_eq: Option<CompiledDpConstraint>,
    pub materials_count_lte: Option<CompiledDpConstraint>,
    pub materials_count_gte: Option<CompiledDpConstraint>,
    #[serde(default)]
    pub materials_count_eq: Option<CompiledDpConstraint>,
    /// `(filter, at_least)` — count of digivolution sources matching `filter`
    /// must be ≥ `at_least`. Compiled form of the `source_count` leaf.
    /// G-DSL-SOURCE-COUNT-FILTERED.
    #[serde(default)]
    pub source_count: Option<(Box<CompiledPredicate>, u8)>,
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
    /// True when the observer's battle-area permanents matching the inner
    /// filter include ≥ N distinct (synth-identity-aware) card names.
    /// G-DSL-DISTINCT-NAMED-PERMANENT-COUNT. `#[serde(default)]` keeps older
    /// packs loadable.
    #[serde(default)]
    pub distinct_named_count_gte: Option<CompiledDistinctNamedCount>,
    /// True when the observer's battle-area Tamers collectively carry at least
    /// N face-down digivolution sources. G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER.
    pub face_down_sources_under_tamers_gte: Option<u8>,
    /// Battle-context leaf: true when the effect's carrier is battling an
    /// opposing Digimon with zero digivolution source cards.
    pub battle_opponent_no_sources: Option<bool>,
    /// Source-relative threshold: `count(carrier sources matching filter) <op>
    /// value`. G-DSL-SELF-SOURCE-COUNT-THRESHOLD (driver BT21-006).
    #[serde(default)]
    pub self_source_count: Option<CompiledSelfSourceCountPredicate>,
    /// True when the effect CARRIER's NON-FLIPPED digivolution-source color
    /// set (the shared `non_flipped_source_colors` extraction — top card
    /// excluded, face-down sources excluded) has at least N distinct colors.
    /// A no-subject, carrier-scoped global leaf — the YAML-reachable branch
    /// gate for EX9-074 Kimeramon's "if this Digimon has 6 or more colors in
    /// its digivolution cards, instead …" discriminant.
    /// G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD. `#[serde(default)]`
    /// keeps older serialized packs deserializable.
    #[serde(default)]
    pub own_source_stack_color_count_gte: Option<u8>,
    pub zone: Vec<CompiledZone>,
    pub owner: Option<CompiledPlayerRef>,
    pub other: Option<bool>,
    /// Mirror of `other` — subject must BE the source permanent. G-OPT-REFUND-ON-DECLINE.
    #[serde(default)]
    pub is_source: Option<bool>,
    pub of_permanent: Option<String>,
    pub not_in_binding: Option<String>,
    pub binding_owner: Option<CompiledBindingOwnerPredicate>,
    pub binding_card_kind: Option<CompiledBindingCardKindPredicate>,
    /// The card bound to `.0` shares ≥1 printed color with `.1`.
    /// G-DSL-BINDING-CARD-COLOR. `#[serde(default)]` keeps older packs loadable.
    #[serde(default)]
    pub binding_card_color: Option<CompiledBindingCardColorPredicate>,
    /// The card bound to `.0` has the effective name `.1` (case-insensitive,
    /// `also_treated_as` aware). G-DSL-BINDING-CARD-NAME-EQUALS.
    #[serde(default)]
    pub binding_card_name_is: Option<(String, String)>,
    /// Candidate play-cost comparator vs a bound/event permanent's play cost
    /// plus offset. G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT.
    #[serde(default)]
    pub play_cost_eq_binding: Option<CompiledPlayCostBindingPredicate>,
    pub source_is_tamer: Option<bool>,
    pub source_is_unsuspended: Option<bool>,
    pub is_source_permanent: Option<bool>,
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
    /// Controller-relative memory bound ("while you have N or less memory").
    /// G-DSL-OWN-MEMORY-PREDICATE.
    #[serde(default)]
    pub own_memory_lte: Option<CompiledDpConstraint>,
    #[serde(default)]
    pub own_memory_gte: Option<CompiledDpConstraint>,
    pub security_count_lte: Option<CompiledDpConstraint>,
    pub security_count_gte: Option<CompiledDpConstraint>,
    #[serde(default)]
    pub security_count_eq: Option<CompiledDpConstraint>,
    pub opponent_security_count_lte: Option<CompiledDpConstraint>,
    pub opponent_security_count_gte: Option<CompiledDpConstraint>,
    /// Sum of BOTH players' security-stack card counts vs a threshold.
    /// `#[serde(default)]` keeps older serialized packs deserializable.
    /// G-DSL-TOTAL-SECURITY-COUNT-PREDICATE (driver BT13-106).
    #[serde(default)]
    pub total_security_count_lte: Option<CompiledDpConstraint>,
    #[serde(default)]
    pub total_security_count_gte: Option<CompiledDpConstraint>,
    #[serde(default)]
    pub total_security_count_eq: Option<CompiledDpConstraint>,
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
    /// Substring / root-trait sibling of `event_target_trait_has`.
    /// G-DSL-EVENT-TARGET-TRAIT-CONTAINS. `#[serde(default)]` keeps older
    /// packs loadable.
    #[serde(default)]
    pub event_target_trait_contains: Option<String>,
    /// Deletion-subject source-count gate: event target has ≥1 digivolution
    /// card (non-empty source stack below the top). G-DSL-EVENT-TARGET-SOURCE-COUNT.
    #[serde(default)]
    pub event_target_has_digivolution_cards: Option<bool>,
    /// Deletion-subject stack-size threshold: event target's stack (top card +
    /// sources) is ≥ N. G-DSL-EVENT-TARGET-SOURCE-COUNT.
    #[serde(default)]
    pub event_target_stack_size_gte: Option<u8>,
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
    /// Case-insensitive substring scan against the triggering event card's
    /// printed text (effect / inherited / security). G-DSL-EVENT-CARD-TEXT-CONTAINS.
    pub event_card_text_contains: Option<String>,
    /// Broad whole-card "[X] in its text" scan (DCGO `CardSource.HasText`)
    /// applied to the triggering event card — name + aliases + traits + text.
    /// `#[serde(default)]` keeps older serialized packs deserializable.
    /// G-DSL-EVENT-CARD-IN-TEXT-CONTAINS.
    #[serde(default)]
    pub event_card_in_text_contains: Option<String>,
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
    /// G-SHARED-OPT-HETEROGENEOUS-TIMING: trash-event host is an own Tamer.
    pub event_host_is_own_tamer: Option<bool>,
    pub event_is_effect_initiated: Option<bool>,
    pub event_target_same_level_as_previous: Option<bool>,
    pub event_cause: Option<CompiledEventCause>,
    pub replacement_cause: Option<CompiledReplacementCause>,
    pub replacement_source_is_opponent: Option<bool>,
    pub replacement_subject_is_mine: Option<bool>,
    pub would_link_card_trait_any_of: Option<Vec<String>>,
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
    /// DP-threshold sibling of `effect_deleted_any_opponent_digimon`: true iff
    /// an opponent Digimon with pre-removal effective DP `>= N` was deleted by
    /// this effect. G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD (EX4-065).
    pub effect_deleted_opponent_digimon_dp_gte: Option<CompiledDpConstraint>,
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
    /// `OnAddToHand` observer: the gaining player must match this ref.
    pub event_add_to_hand_player: Option<CompiledPlayerRef>,
    /// `on_ally_won_battle`: the battle winner's controller must match this ref.
    /// G-DSL-BATTLE-WINNER-BOARDWIDE.
    pub event_winner_owner: Option<CompiledPlayerRef>,
    /// `on_ally_won_battle`: the battle winner's top card must carry this trait.
    pub event_winner_trait_has: Option<String>,
    /// `on_discard_hand`: the trashing player (whose hand lost cards) must match.
    /// G-ENGINE-ON-DISCARD-HAND.
    pub event_discard_player: Option<CompiledPlayerRef>,
    /// `on_discard_hand`: true when the causing effect is the observer's own.
    pub event_caused_by_own_effect: Option<bool>,
    /// True when this permanent was played by an effect (OnPlay firing).
    /// G-ENGINE-ON-DISCARD-HAND.
    pub played_by_effect: Option<bool>,
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
pub struct CompiledBindingCardKindPredicate {
    pub binding: String,
    pub kind: CompiledCardKind,
}

/// Compiled `binding_card_color`. G-DSL-BINDING-CARD-COLOR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledBindingCardColorPredicate {
    pub binding: String,
    pub color_is: CompiledColor,
}

/// Compiled `play_cost_eq_binding`. G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPlayCostBindingPredicate {
    pub binding: String,
    pub offset: i16,
    pub op: CompiledComparatorOp,
}

/// Compiled `distinct_named_count_gte`. G-DSL-DISTINCT-NAMED-PERMANENT-COUNT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledDistinctNamedCount {
    pub of: CompiledPlayerRef,
    pub filter: Box<CompiledPredicate>,
    pub n: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledCountAggregate {
    pub filter: Box<CompiledPredicate>,
    pub n: CompiledDpConstraint,
}

/// Compiled comparison operator. Compiled analogue of `predicate::ComparatorOp`.
/// Kept as a distinct compiled type so the pack IR does not depend on the DSL
/// parse enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledComparatorOp {
    Eq,
    Gte,
    Lte,
}

impl CompiledComparatorOp {
    /// Apply this operator to `lhs <op> rhs`.
    pub fn apply(self, lhs: i32, rhs: i32) -> bool {
        match self {
            Self::Eq => lhs == rhs,
            Self::Gte => lhs >= rhs,
            Self::Lte => lhs <= rhs,
        }
    }
}

/// Compiled form of `self_source_count` — `count(carrier sources matching
/// filter) <op> value`. G-DSL-SELF-SOURCE-COUNT-THRESHOLD (driver BT21-006).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledSelfSourceCountPredicate {
    pub filter: Option<Box<CompiledPredicate>>,
    pub op: CompiledComparatorOp,
    pub value: CompiledDpConstraint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledExistential {
    pub of: CompiledPlayerRef,
    pub predicate: CompiledPredicate,
}

/// Compiled form of `no_face_up_security_named`. Exactly one of
/// `card_number_is` / `name_is` / `color_is` is populated.
/// G-PRED-NO-FACE-UP-SECURITY-NAMED (+ color arm for EX10-020 / Q3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledFaceUpSecurityNamed {
    pub of: CompiledPlayerRef,
    pub card_number_is: Option<String>,
    pub name_is: Option<String>,
    pub color_is: Option<CompiledColor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledPlayerRef {
    You,
    Opponent,
    Any,
    Active,
}

/// Compiled scope for `color_matches_own_source_stack`
/// (G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK). Only `SelfCarrier` exists in the
/// printed corpus (EX9-074 Kimeramon); the enum shape leaves room to widen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompiledSourceStackScope {
    /// The effect's own carrier permanent (`source_permanent`).
    SelfCarrier,
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
    /// Left-associative subtraction: `subtract(a, b, c) = a - b - c`. Binary in
    /// practice (`a - b`). Composes with the other compounds so a card can
    /// express e.g. "N minus your Digimon count". G-DSL-FORMULA-SUBTRACT.
    Subtract(Vec<CompiledFormula>),
    Aggregate(CompiledAggregateSelector),
    AggregateScoped {
        selector: CompiledAggregateSelector,
        scope: CompiledPlayerRef,
    },
    BindingDp(String),
    BindingPlayCost(String),
    BindingValue(String),
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
    /// Level of the current trigger's EVENT card (e.g. the just-played
    /// Digimon for an `on_any_digimon_played` observer). 0 when no trigger
    /// context / no level. G-EVENT-PLAYED-LEVEL-FORMULA (EX5-060, Q28).
    EventTargetLevel,
    /// Distinct colors represented by source cards beneath the effect carrier's top card.
    SourceColorCount,
    SourceStackCount {
        target: String,
        filter: Option<Box<CompiledPredicate>>,
    },
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
    SourceColorCount,
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
    /// Count of the effect carrier's own digivolution sources (beneath its top
    /// card) matching `filter`. Composes in `BasePerDelta` to scale a numeric
    /// by source count. Drives EX3-014's scaling DP cap.
    /// G-DSL-PER-SOURCE-STACK-COUNT-FILTERED.
    SourceStackCountFiltered {
        filter: Option<Box<CompiledPredicate>>,
    },
    /// A player's memory-gauge value as a scalar. `of: You` = controller's
    /// signed memory; `of: Opponent` = opponent's, clamped at 0.
    /// G-DSL-FORMULA-PLAYER-MEMORY (driver BT25-086).
    PlayerMemory {
        of: CompiledPlayerRef,
    },
    /// Total link cards across every one of `of`'s battle-area Digimon.
    /// Drives BT25-075's "for each of your link cards, De-Digivolve 1 …".
    /// G-DSL-FORMULA-OWN-LINK-CARD-COUNT.
    OwnLinkCardCount {
        of: CompiledPlayerRef,
    },
    /// Link cards on the effect carrier's OWN permanent (`ctx.source_permanent`).
    /// Per-host sibling of `OwnLinkCardCount`. G-DSL-LINK-N-CARDS-PER-HOST.
    SourceLinkCardCount,
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

/// Compiled synthetic identity for a `TreatAsDigimon` modifier. Mirrors the
/// engine's `ModifierPayload::SynthIdentity` fields; the engine lowering
/// converts `kind`/`colors` to its own enums.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledSynthIdentity {
    pub kind: CompiledCardKind,
    pub level: u8,
    pub colors: Vec<CompiledColor>,
    pub traits: Vec<String>,
    pub dp: i32,
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
    /// Per-timing condition overrides (G-SHARED-OPT-HETEROGENEOUS-TIMING).
    /// When this clause fires from timing `T`, the matching entry's condition
    /// is AND-ed onto the clause-level gates; timings with no entry use only
    /// the clause-level gates. Empty for the common single-condition case.
    #[serde(default)]
    pub timing_conditions: Vec<CompiledTimingCondition>,
    pub optional: bool,
    /// Force the explicit outer accept/decline confirm even when the first
    /// body step is declinable (DCGO's always-shown initial Yes/No). Declining
    /// the confirm drops the queued effect before the OPT is recorded — DCGO
    /// `RemoveUse` semantics. G-OPT-REFUND-ON-DECLINE.
    #[serde(default)]
    pub outer_prompt: bool,
    pub once_per_turn: bool,
    pub max_per_turn: Option<u8>,
    pub process: Vec<CompiledStep>,
    pub summary: Option<String>,
    pub summary_key: Option<String>,
}

/// Compiled `timing_conditions:` entry (G-SHARED-OPT-HETEROGENEOUS-TIMING).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledTimingCondition {
    pub when: CompiledTiming,
    pub condition: CompiledPredicate,
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
        /// Scalar `value` for the named `modifier` grant (e.g. `ChangeLinkMax`
        /// "Link +N"). `None` ⇒ `0`. Fixes G-ENGINE-AURA-GRANT-LINK-MAX.
        modifier_value: Option<i32>,
        /// Name payload for the named `modifier` grant — installed as
        /// `ModifierPayload::Name { value, base: false }`. Used by
        /// `CanOnlyDigivolveInto` (judge-quiz Q3, EX10-020).
        modifier_name: Option<String>,
        /// Structured `TreatAsDigimon` SynthIdentity payload for a mass aura
        /// ("all of your [Marcus Damon]s are also treated as a 12000 DP
        /// Digimon" — BT25-104). No `skip_serializing_if`: the embedded pack
        /// round-trips `CompiledClause` through bincode (non-self-describing),
        /// so the field must always be written. G-DSL-AURA-TREAT-AS-DIGIMON-SYNTH.
        #[serde(default)]
        synth_identity: Option<CompiledSynthIdentity>,
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
        /// Continuous filtered `CannotBeAffected` grant — "this Digimon
        /// isn't affected by [your opponent's] [<kind>] effects" as a
        /// per-tick aura. Self-aura only. G-DSL-AURA-EFFECT-IMMUNITY.
        #[serde(default)]
        effect_immunity: Option<CompiledAuraEffectImmunity>,
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
    /// A Shape-B Appmon Link *Digimon*'s static self link-condition
    /// (`kind: link_condition`): a link cost + host filter. Lowers to an
    /// `EffectTiming::LinkCondition` effect read by the link-activate path,
    /// distinct from the Option-scoped `LinkRequirement`.
    LinkCondition {
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
    OnBlock,
    OnAllyAttack,
    OnOpponentAttack,
    OnDeletion,
    OnAnyDeletion,
    /// Board-wide battle-winner observer (`when: on_ally_won_battle`); lowers to
    /// `EffectTiming::EndOfBattle` with NO forced self-filter. Scope-gate via
    /// `active_when:` (`event_winner_owner`, `event_winner_trait_has`).
    /// G-DSL-BATTLE-WINNER-BOARDWIDE.
    OnAllyWonBattle,
    /// Hand-discard observer (`when: on_discard_hand`); lowers to
    /// `EffectTiming::OnDiscardHand`. Scope-gate via `active_when:`
    /// (`event_discard_player`, `event_caused_by_own_effect`).
    /// G-ENGINE-ON-DISCARD-HAND.
    OnDiscardHand,
    OnEnterFieldAnyone,
    OnAnyDigimonPlayed,
    OnAllyPlayed,
    OnLeaveField,
    OnSuspend,
    OnUnsuspend,
    OnAddToHand,
    OnHatch,
    OnMove,
    OnDigivolve,
    OnDnaDigivolve,
    OnDigixros,
    OnOpponentSecurityRemoved,
    OnOwnSecurityRemoved,
    OnDigivolutionCardTrashed,
    /// A digivolution source RETURNED to the bottom of a player's deck (not
    /// trashed). Galacticmon / Vemmon-LIBERATOR observer (BT21-058, BT18-065).
    /// G-ENGINE-DIGIVOLUTION-CARD-RETURNED-TO-DECK-BOTTOM.
    OnDigivolutionCardReturnedToDeckBottom,
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
    /// DigiLink Shape-B: "when this Digimon gets linked". Authored as
    /// `when: when_linked` on a `scope: linked` effect; lowers to
    /// `EffectTiming::OnLink` with `.linked()` and a self-filter so it fires
    /// only when THIS card is the just-linked card (design D6).
    WhenLinked,
    /// DigiLink host-side: "[When Linked] a card gets linked to this Digimon".
    /// Authored as `when: when_card_linked_to_this` on a face-up `scope`;
    /// lowers to `EffectTiming::OnLink` with a host self-filter
    /// (`event_permanent == source_permanent`) — fires once for the receiving
    /// host only. Mirrors DCGO `CanTriggerWhenLinked`.
    WhenCardLinkedToThis,
    /// DigiLink host-side pre-link replacement: "a card WOULD link to this
    /// Digimon". Authored as `when: when_would_link_to_this` on a face-up
    /// `scope`; lowers to a `EffectTiming::WhenWouldLink` REPLACEMENT effect
    /// with a host self-filter (`pending_link_host() == source_permanent`).
    /// Pair with `optional` + a `reduce_link_cost` step (Gap 5).
    WhenWouldLinkToThis,
    /// DigiLink board-wide observer: fires on every `OnLink` dispatch with NO
    /// forced self/host filter. Authored as `when: on_any_link`; scope-gate via
    /// `active_when:` (`event_target_owner`, `event_card_trait_has`,
    /// `your_turn`). G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED.
    OnAnyLink,
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

/// Compiled form of `AuraEffectImmunity` — the continuous filtered
/// `CannotBeAffected` slot on `CompiledDeclarativeClause::Aura`.
/// `source_kind: None` ⇒ immune to effects from ANY source kind.
/// G-DSL-AURA-EFFECT-IMMUNITY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledAuraEffectImmunity {
    pub source_kind: Option<CompiledEffectSourceKind>,
    pub source_controller: CompiledEffectController,
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
    /// Fewest digivolution cards beneath the top card among candidate
    /// permanents.
    LowestMaterialCount,
}

// ── Steps ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CompiledAttackCostUpgrade {
    pub dp: i32,
    pub security_attack: i32,
}

/// Compiled source zone for the result card in a `CompiledStep::AppFuse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledAppFuseZone {
    Hand,
    Trash,
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
        /// Runtime-formula count (`count:` map form). When present, `count`
        /// is 0 and this formula is evaluated at resolution time.
        /// `#[serde(default)]` — NOT `skip_serializing_if` — bincode rule.
        #[serde(default)]
        count_fn: Option<Box<CompiledFormula>>,
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
    /// Move a bound card from a player's security stack to that player's deck.
    /// G-DSL-RETURN-SELECTED-SECURITY-TO-DECK.
    ReturnSelectedSecurityToDeck {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
        position: CompiledStackPosition,
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
    /// collapse §2 `reveal_search` composite. The executor expands this into
    /// the existing sequence at run time (reveal_top_deck → a single
    /// `select_reveal_buckets` over all buckets → per-bucket reveal-move →
    /// place_remainder_on_deck). Pure data → deterministic expansion
    /// (cloneable-aligned; reuses the existing reveal-bucket selection).
    RevealSearch {
        of: CompiledPlayerRef,
        count: u8,
        buckets: Vec<CompiledRevealSearchBucket>,
        remainder: CompiledRevealRemainder,
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
    /// Delete `target` AS AN INTERACTIVE COST and reduce the in-flight play/
    /// digivolve/option cost by the deleted permanent's printed play cost
    /// (snapshotted PRE-removal, rule 25 — DCGO `permanent.CostJustBeforeRemoveField`).
    /// Only meaningful inside a `cost_reduction` `pay_cost`; drives the VARIABLE
    /// amount of BT13-103 Akihiro Kurata's "reduce the play cost by the play cost
    /// of the deleted Digimon". Records the amount into
    /// `Game::pending_cost_reduction_amount_override` for the cost-continuation to
    /// credit. `G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST`.
    DeleteForCostReduction {
        target: CompiledBindingRef,
    },
    DeleteBoundPermanents {
        binding: String,
    },
    /// G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR (EX9-074 Kimeramon Branch B):
    /// per game-color, a mandatory pick of 1 not-yet-chosen opponent Digimon of
    /// that color, then batch-delete the accumulated picks. `filter` is an extra
    /// AND-ed restriction (`None` = every opponent Digimon).
    DeleteOnePerOpponentColor {
        filter: Option<Box<CompiledPredicate>>,
        prompt: Option<String>,
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
        /// Bind the placed card's `CardHandle` after a successful placement
        /// (`bind_placed_as:` in YAML). `#[serde(default)]` — NOT
        /// `skip_serializing_if` — so bincode round-trips (Compiled* rule).
        #[serde(default)]
        bind_placed_as: Option<String>,
    },
    /// G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05) — insert a card at
    /// the TOP-source position of `target`'s digivolution stack (directly
    /// beneath the active top card; the top card itself is unchanged).
    /// DCGO `Permanent.AddDigivolutionCardsTop`. Same args shape as
    /// `PlaceAsBottomSource`; fields are plain (no `skip_serializing_if`)
    /// because the compiled pack is bincode-serialized.
    PlaceAsTopSource {
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
        /// Declinable Tamer pick (PASS skips the trash AND the tail) — DCGO
        /// `canNoSelect:true`. `G-OPTIONAL-TRASH-FACE-DOWN-UNDER-TAMER`. Plain
        /// (no `skip_serializing_if`): the compiled pack is bincode-serialized
        /// (non-self-describing), so every field must occupy a fixed slot.
        #[serde(default)]
        optional: bool,
    },
    /// Multi-count / multi-Tamer sibling of
    /// `TrashBottomFaceDownSourceUnderTamer`. Trash `count` bottom-face-down
    /// digivolution sources total, distributed across `of`'s Tamers, each pick
    /// surfaced as a real selection. Compiled from
    /// `trash_bottom_face_down_sources_under_tamers`; used as an activation cost
    /// by BT25-035 Cougarmon. `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER`.
    TrashBottomFaceDownSourcesUnderTamers {
        of: CompiledPlayerRef,
        count: u8,
    },
    /// Link-card-trash ACTIVATION cost (BT25-073 Dragomon): pick one of `of`'s
    /// Digimon with ≥1 link card, then one of its link cards, trash it (firing
    /// `OnLinkedCardTrashed`), and run the tail only if the trash happened.
    /// Compiled from `trash_link_card_of_own_digimon`. `G-DSL-LINK-TRASH-AS-COST`.
    TrashLinkCardOfOwnDigimon {
        of: CompiledPlayerRef,
        /// Declinable picks (PASS skips the trash AND the tail) — DCGO
        /// `canNoSelect:true`. Plain (no `skip_serializing_if`): bincode packs
        /// need every field in a fixed slot.
        #[serde(default)]
        optional: bool,
    },
    /// Trash-Option-from-{digivolution|link}-cards ACTIVATION cost (BT25-085
    /// BeelStarmon): pick one of `of`'s Digimon whose digivolution cards (sources
    /// below the top) OR link cards contain ≥1 Option, then one such Option, and
    /// trash it — a digivolution-source Option fires `OnDigivolutionCardTrashed`,
    /// a link-card Option fires `OnLinkedCardTrashed` — running the tail only if
    /// the trash happened. Compiled from `trash_option_from_own_stacks`. The
    /// union-of-sources sibling of `TrashLinkCardOfOwnDigimon`.
    /// `G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST`.
    TrashOptionFromOwnStacks {
        of: CompiledPlayerRef,
        /// Declinable picks (PASS skips the trash AND the tail) — DCGO
        /// `canNoSelect:true`. Plain (no `skip_serializing_if`): bincode packs
        /// need every field in a fixed slot.
        #[serde(default)]
        optional: bool,
    },
    /// Trash-1-Option-from-the-CARRIER'S-digivolution-cards leave cost (EX7-048
    /// Gundramon clause 2): the single-carrier variant of `TrashOptionFromOwnStacks`
    /// scoped to the effect's own `source_permanent`. Pick one Option among the
    /// carrier's digivolution cards (below the top), trash it (firing
    /// `OnDigivolutionCardTrashed`), and cancel the parked leave — it owns the
    /// `outcome: prevent`, so no separate `CancelReplacement` follows. Synthesized
    /// from `cost: { trash_option_from_own_digivolution_cards: true }` +
    /// `outcome: prevent` on a `when_would_leave_battle_area` replacement.
    /// `G-DSL-PROTECT-OTHER-BY-TRASH-OPTION`.
    TrashOptionFromOwnDigivolutionCardsAndCancelLeave,
    /// Delete-the-CARRIER leave cost (BT25-039 Sirenmon clause 3): delete the
    /// effect's own `source_permanent` (the carrier) as the cost, then cancel the
    /// parked leave — which prevents the replacement SUBJECT's leave (the
    /// framework applies the outcome to the subject, not the carrier). A single
    /// self-contained step so it does NOT collide with the Delay-cost recognizer's
    /// `[DeletePermanent{source}, CancelReplacement]` shape (that path requires a
    /// Delayed-Option source). Synthesized from `cost: { delete_self: true }` +
    /// `outcome: prevent` on a `when_would_leave_battle_area` replacement.
    /// `G-DSL-PROTECT-OTHER-BY-SELF-DELETE`.
    DeleteSelfAndCancelLeave,
    PlaceSelectedCardUnderTamer {
        card: CompiledBindingRef,
        tamer: CompiledBindingRef,
        face_down: bool,
        bind_as: Option<String>,
    },
    /// Relocate THIS effect's source Option (an in-battle-area field Option)
    /// face-down under a chosen own permanent — a new Option-lifecycle exit
    /// distinct from trashing. Compiled from `move_self_option_under_permanent`.
    /// G-MOVE-SELF-OPTION-UNDER-PERMANENT.
    MoveSelfOptionUnderPermanent {
        target: CompiledBindingRef,
        face_down: bool,
    },
    /// Place THIS effect's source Option as the bottom digivolution card of a
    /// chosen own permanent, composing with the [Main] Option-play path (the
    /// in-flight `pending_option` is claimed, so the Option is seated —
    /// FACE-UP by default — instead of trashed). Compiled from
    /// `place_self_under_permanent`; lowers to
    /// `EffectContext::place_self_under_permanent`.
    /// G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL (P-180 / EX7-070 / EX7-071).
    PlaceSelfUnderPermanent {
        target: CompiledBindingRef,
        face_down: bool,
    },
    PlaceSelectedSourcesUnderTamer {
        source_refs: String,
        tamer: CompiledBindingRef,
        bind_count_as: Option<String>,
    },
    MoveMatchingSourcesUnderTamer {
        from: CompiledBindingRef,
        tamer: CompiledBindingRef,
        filter: CompiledPredicate,
        bind_count_as: Option<String>,
    },
    TrashTopStackedSources {
        target: CompiledBindingRef,
        count: CompiledFormula,
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
        /// Bind the just-played permanent handle for use in later steps.
        /// `None` preserves prior behavior (no binding insert). Mirrors
        /// `PlayFromHandFree::bind_as` for the cost-paying play.
        /// G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER (BT25-039).
        #[serde(default)]
        bind_as: Option<String>,
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
    /// Effect-driven Option USE from the controller's trash with a cost delta
    /// (Gap 2, `G-DSL-USE-OPTION-FROM-SOURCES`). Trash analogue of
    /// `UseOptionFromHand`. `cost_delta: None` = pay printed use cost.
    UseOptionFromTrash {
        of: CompiledPlayerRef,
        filter: CompiledPredicate,
        cost_delta: Option<CompiledCostDelta>,
        optional: bool,
        prompt: Option<String>,
    },
    /// Effect-driven Option USE of a reveal-pool pick — consumes a
    /// `select_reveal` binding (`card` → the reveal-pool card handle) and routes
    /// it through the shared `Game::use_option_from` pipeline
    /// (`OptionSource::Revealed`). Non-parking: the upstream `select_reveal`
    /// already parked; this step consumes its binding synchronously. Driver
    /// EX7-048 clause 1 ("reveal top 6 … use 1 [Three Musketeers] Option among
    /// them free"). `cost_delta: None` = free. `G-DSL-USE-OPTION-FROM-SOURCES`.
    UseOptionFromRevealed {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
        cost_delta: Option<CompiledCostDelta>,
    },
    /// Effect-driven Option USE from a permanent's digivolution stack —
    /// consumes a `select_own_sources` binding (`card` → a `SourceRefs`
    /// binding) and routes it through `Game::use_option_from`
    /// (`OptionSource::Source`, which resolves both `card_sources` and
    /// `linked_cards`). Non-parking. Driver BT25-085 ("use 1 [3M]/[TS] Option
    /// from … this Digimon's digivolution cards free"). `cost_delta: None` =
    /// free. `G-DSL-USE-OPTION-FROM-SOURCES`.
    UseOptionFromSources {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
        cost_delta: Option<CompiledCostDelta>,
    },
    /// Effect-driven Option USE of a `select_union_zone{hand,trash}` pick —
    /// consumes the named union binding and dispatches to the picked Option's
    /// TRUE origin zone (hand → `OptionSource::Hand`, trash →
    /// `OptionSource::Trash`) through `Game::use_option_from`. Non-parking.
    /// Driver BT21-062 ("use 1 [Ragnarok Cannon] from your hand or trash without
    /// paying the cost"). `cost_delta: None` = free. `G-DSL-USE-OPTION-FROM-SOURCES`.
    UseOptionBound {
        binding: String,
        cost_delta: Option<CompiledCostDelta>,
    },
    /// Unified play-or-use of a `select_hand`-bound card with a cost
    /// adjustment. `G-PLAY-OR-USE-FROM-HAND`.
    PlayOrUseFromHand {
        of: CompiledPlayerRef,
        hand_index: CompiledBindingRef,
        cost_delta: Option<CompiledCostDelta>,
    },
    PlayFromRevealedFree {
        of: CompiledPlayerRef,
        card: CompiledBindingRef,
        /// Bind the just-played permanent handle for use in later steps.
        bind_as: Option<String>,
        /// Optional reduced (non-free) cost. `None` = free (prior behavior);
        /// `Reduce(N)` pays the printed cost minus N. G-DSL-PLAY-FROM-REVEALED-COST-REDUCED.
        cost_delta: Option<CompiledCostDelta>,
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
        /// The skip is consult-gated on the played permanent NOT being
        /// protected from the suppressing effect (judge-quiz Q28).
        suppress_on_play: bool,
        /// G-PLAY-ENTERS-SUSPENDED — the played permanent enters the battle
        /// area suspended (EX5-060 "plays ... suspended", Q28).
        suspended: bool,
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
    PlayUnderTamerSource {
        source_refs: String,
        cost_delta: Option<CompiledCostDelta>,
        bind_as: Option<String>,
    },
    EffectInitiatedDigivolve {
        target: CompiledBindingRef,
        from_hand: CompiledBindingRef,
        cost: CompiledCostDelta,
        ignore_requirements: bool,
    },
    /// Effect-initiated App Fuse: two engine-driven selections (own permanent,
    /// then result card from `from_zone`) routed through the app-fusion commit.
    /// Compiled form of `StepSpec::AppFuse`; the engine lowering calls
    /// `EffectContext::initiate_effect_app_fuse`.
    AppFuse {
        from_zone: CompiledAppFuseZone,
        result_filter: Option<CompiledPredicate>,
        optional: bool,
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
    /// DNA digivolve where `target` is a battle-area permanent and
    /// `trash_partner` is the second DNA material drawn from the controller's
    /// TRASH; the merged permanent is topped with `from_hand` (the result
    /// card, from hand). BT18-015 / BT18-073 `[On Deletion]` shape. Lowers to
    /// `EffectContext::effect_initiated_dna_digivolve_trash_partner`
    /// (G-ENGINE-DNA-TRASH-MATERIAL). G-DSL-DNA-TRASH-PARTNER.
    EffectInitiatedDnaDigivolveTrashPartner {
        target: CompiledBindingRef,
        trash_partner: CompiledBindingRef,
        from_hand: CompiledBindingRef,
        cost: CompiledCostDelta,
        ignore_requirements: bool,
    },
    TrashTopSecurity {
        of: CompiledPlayerRef,
        /// Number of top security cards to trash, as a run-time formula.
        /// `None` trashes exactly one (the historical behavior).
        count: Option<CompiledFormula>,
        /// "so that it has N cards left" — trash from the top until `leave`
        /// cards remain. Mutually exclusive with `count`. `#[serde(default)]`
        /// keeps older serialized packs deserializable.
        /// G-DSL-TRASH-TOP-SECURITY-LEAVE (driver BT21-098).
        #[serde(default)]
        leave: Option<CompiledFormula>,
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
        /// Named literal binding recording the count actually trashed.
        /// G-DSL-TRASH-COUNT-RESULT-BINDING.
        bind_count_as: Option<String>,
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
        /// Structured payload for `TreatAsDigimon` (lowers to
        /// `ModifierPayload::SynthIdentity`). `None` for scalar modifiers.
        /// NOTE: no `skip_serializing_if` — the embedded pack round-trips
        /// `CompiledStep` through bincode (a non-self-describing format), so
        /// the field must always be written or the byte stream desyncs.
        #[serde(default)]
        synth_identity: Option<CompiledSynthIdentity>,
        /// CONTINUOUS mass modifier (G-CONTINUOUS-MASS-DP-DEBUFF): with a FILTER
        /// target, register a source-independent floating effect re-applied each
        /// tick instead of a one-time scan. No `skip_serializing_if` for the
        /// same bincode round-trip reason as `synth_identity`.
        #[serde(default)]
        continuous: bool,
    },
    AddPlayerModifier {
        target_player: CompiledPlayerRef,
        modifier: String,
        value: CompiledModifierValue,
        expiry: String,
    },
    GrantKeyword {
        target: CompiledBindingRef,
        keyword: String,
        expiry: String,
        value: Option<i32>,
    },
    AllowDigixrosMaterialZone {
        zone: CompiledZone,
        max_count: Option<u8>,
    },
    AddDigixrosCostDelta {
        delta: i16,
    },
    PreattachDigixrosMaterial {
        card: CompiledBindingRef,
        cost_delta: i16,
    },
    RegisterDigixrosWildcardForTurn {
        card: CompiledBindingRef,
        zone: Option<CompiledZone>,
    },
    AddDigixrosWildcardToPendingTransaction {
        card: CompiledBindingRef,
        zone: Option<CompiledZone>,
    },
    GrantEffectImmunity {
        /// Per-permanent grant target (`None` for the continuous form).
        target: Option<CompiledBindingRef>,
        source_kind: CompiledEffectSourceKind,
        source_controller: CompiledEffectController,
        expiry: String,
        /// G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA (Q28 / BT20-059): a
        /// source-independent continuous immunity over `targets`, re-scanned
        /// every declarative tick — covers permanents played later in the
        /// window (DCGO `CanNotAffectedClass` re-evaluated CardCondition).
        continuous: bool,
        /// Battle-area predicate for the continuous form.
        targets: Option<CompiledPredicate>,
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
        /// Decline continues the clause tail (binding unresolved) instead of
        /// dropping it. G-OPT-REFUND-ON-DECLINE.
        #[serde(default)]
        continue_on_decline: bool,
        /// collapse §1 explicit scoped action-tail — run on accept with the
        /// selection binding in scope, prepended to the implicit dispatcher
        /// tail. `#[serde(default)]` for bincode-pack back-compat.
        #[serde(default)]
        then: Vec<CompiledStep>,
    },
    SelectOpponentPermanent {
        filter: CompiledPredicate,
        bind_as: Option<String>,
        selector: Option<CompiledFieldSelector>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
        /// Decline continues the clause tail (binding unresolved) instead of
        /// dropping it. G-OPT-REFUND-ON-DECLINE.
        #[serde(default)]
        continue_on_decline: bool,
        /// collapse §1 explicit scoped action-tail. See `SelectOwnPermanent::then`.
        #[serde(default)]
        then: Vec<CompiledStep>,
    },
    SelectAnyPermanent {
        filter: CompiledPredicate,
        bind_as: Option<String>,
        selector: Option<CompiledFieldSelector>,
        prompt: String,
        prompt_key: Option<String>,
        optional: bool,
        /// collapse §1 explicit scoped action-tail. See `SelectOwnPermanent::then`.
        #[serde(default)]
        then: Vec<CompiledStep>,
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
        /// collapse §1 explicit scoped action-tail. See `SelectOwnPermanent::then`.
        #[serde(default)]
        then: Vec<CompiledStep>,
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
        /// collapse §1 explicit scoped action-tail. See `SelectOwnPermanent::then`.
        #[serde(default)]
        then: Vec<CompiledStep>,
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
    ///
    /// `min`/`max` are literal-or-formula bounds resolved once at execution
    /// time (G-DSL-SELECT-SOURCES-FORMULA-COUNT — driver EX11-057).
    SelectOpponentSources {
        target: Option<CompiledBindingRef>,
        filter: CompiledPredicate,
        min: CompiledCountBound,
        max: CompiledCountBound,
        /// DCGO `TrashDigivolutionCards.cs` min(N, available) parity: clamp
        /// the resolved `min`/`max` to the live candidate count; zero
        /// candidates silently skip the pick. Default `false` keeps the
        /// historical drop-continuation semantics committed availability
        /// ladders (EX7-021 / EX7-023 / EX11-017 / EX8-066) rely on.
        #[serde(default)]
        clamp_to_available: bool,
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
    /// G-MULTI-SELECT-OPP-PLAY-COST-SUM. `play_cost_budget` is a
    /// `CompiledFormula` (mirroring `SelectOpponentDpBudget.dp_budget`) so the
    /// budget can scale at run time — e.g. P-094 Destromon's "3 + 1 per [Vemmon]
    /// in this Digimon's digivolution cards". A bare integer YAML literal
    /// compiles to `CompiledFormula::Literal`, preserving scalar users (EX4-073).
    SelectOpponentPlayCostBudget {
        play_cost_budget: CompiledFormula,
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
    /// G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM — deck-routing sibling of
    /// `ReturnSelectedSourcesToHand`. Return each `SelectOwnSources`-bound
    /// digivolution source card to its owner's deck at `position` (top or
    /// bottom). A return, not a trash — fires no `OnDigivolutionCardTrashed`.
    ReturnSelectedSourcesToDeck {
        source_refs: String,
        position: CompiledStackPosition,
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
        /// collapse §1 explicit scoped action-tail. See `SelectOwnPermanent::then`.
        #[serde(default)]
        then: Vec<CompiledStep>,
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
        /// collapse §1 explicit scoped action-tail. See `SelectOwnPermanent::then`.
        #[serde(default)]
        then: Vec<CompiledStep>,
    },
    SelectUnionZone {
        of: CompiledPlayerRef,
        zones: Vec<CompiledZone>,
        material_of: Option<CompiledBindingRef>,
        filter: CompiledPredicate,
        /// Per-zone filter overrides (hand / trash / material). When a zone's
        /// override is `Some`, it replaces `filter` for candidates from that
        /// origin zone; `None` falls back to `filter`.
        /// G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY. NOTE: no
        /// `skip_serializing_if` — the embedded pack round-trips through
        /// bincode, which is not self-describing and cannot skip fields.
        #[serde(default)]
        zone_filters: CompiledUnionZoneFilters,
        /// Restricts the `material_of: None` carrier scan to field permanents
        /// whose top card matches this predicate (filters the CARRIER, not the
        /// candidate source cards). `None` → scan every field permanent's
        /// sources. G-UNION-HAND-SOURCE-PLAY. NOTE: no `skip_serializing_if` —
        /// the embedded pack round-trips through bincode (non-self-describing),
        /// so the field must always be encoded.
        #[serde(default)]
        material_carrier_filter: Option<CompiledPredicate>,
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
        /// MP-30/31: clamp the required pick-count to available candidates
        /// (`min(max, available)`); never no-op for fewer-than-N. Effect-target
        /// selections only. See `SelectCountCappedArgs::clamp_to_available`.
        clamp_to_available: bool,
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
    /// Gap 5 — reduce the in-flight `WhenWouldLink` link cost by `amount`.
    /// Compiled form of `StepSpec::ReduceLinkCost`; the engine lowering calls
    /// `EffectContext::reduce_pending_link_cost(amount)`.
    ReduceLinkCost {
        amount: u16,
    },
    /// Gap 2 — link 1..N chosen cards from a set of source zones onto a
    /// Digimon host. Compiled form of `StepSpec::LinkCards`. The engine
    /// lowering loops `count` picks: per pick it resolves which `from` zones
    /// hold a filter-matching candidate, installs a zone-choice prompt when
    /// ≥2 do, then a single-zone card select, then (for `to: OwnDigimon`) a
    /// host select, then attaches via `link_chosen_card_into_host`.
    LinkCards {
        from: Vec<CompiledLinkSourceZone>,
        filter: CompiledPredicate,
        to: CompiledLinkTo,
        count: CompiledLinkCount,
        /// Memory cost to pay per linked card. Currently always 0 (the cards
        /// this step serves all link "without paying the cost" / from a base
        /// cost of 0). Threaded so a future non-zero base cost extends the
        /// lowering without a schema change.
        cost: u8,
        prompt: Option<String>,
        /// Optional binding capturing the linked card(s) as a `CardList`, set
        /// ONLY when ≥1 card is linked (so `binding_present` is false on a full
        /// decline). Enables gating a follow-up step on a link having occurred
        /// (BT25-060). `#[serde(default)]` (NOT `skip_serializing_if`) keeps the
        /// bincode pack layout fixed — see reference_dsl_substrate_authoring_gotchas.
        #[serde(default)]
        bind_as: Option<String>,
        /// Optional host predicate (`to: own_digimon` only) — the link card's
        /// link requirement, so the host select never offers an illegal host
        /// (EX11-027). `None` ⇒ any standing own Digimon. `#[serde(default)]`
        /// for bincode-pack layout stability.
        #[serde(default)]
        host_filter: Option<CompiledPredicate>,
        /// Exclude the effect's source permanent from host candidates ("1 of
        /// your OTHER Digimon", EX11-027). `#[serde(default)]` for pack layout.
        #[serde(default)]
        exclude_source: bool,
    },
    /// G-DSL-LINK-RELINK-STANDING-PERMANENT — move the effect's own standing
    /// permanent to become a link card on a chosen OTHER own Digimon (EX11-027).
    /// Compiled form of `StepSpec::RelinkSelfToOwnDigimon`; the engine executor
    /// installs a host select (excluding self, matching `host_filter`) then calls
    /// `Game::absorb_standing_digimon_as_link`.
    RelinkSelfToOwnDigimon {
        #[serde(default)]
        host_filter: Option<CompiledPredicate>,
        #[serde(default)]
        prompt: Option<String>,
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
        ignore_summoning_sickness: bool,
        optional: bool,
        windowed: bool,
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
    /// Refund the running clause's once-per-turn use (DCGO `RemoveUse`).
    /// The engine resolves the OPT key from the step runtime (the lowering
    /// captures it statically per clause). G-OPT-REFUND-ON-DECLINE.
    RefundOpt,
    RefireEffect {
        source: CompiledBindingRef,
        timing: String,
        optional: bool,
    },
    /// Foreign-card refire (BT15-102): activate a timing-filtered effect
    /// printed on a bound CARD OBJECT that is currently a digivolution source
    /// of this effect's carrier, running it with the carrier as "this
    /// Digimon". See `StepSpec::RefireCardEffect`.
    RefireCardEffect {
        card: CompiledBindingRef,
        timing: String,
    },
    EndAttack {
        enabled: bool,
    },
    CancelReplacement,
    /// Gap 3a — trash 1 of the leaving permanent's own link cards as the cost
    /// of a `when_would_leave_battle_area` replacement, then cancel the leave.
    /// Synthesized from `cost: { trash_own_link_card: true }` + `outcome:
    /// prevent`; it owns both the cost-payment (player-chosen which link card)
    /// and the cancel, so no separate `CancelReplacement` follows it.
    TrashOwnLinkCardAndCancelLeave,
    /// EX11-027 Maquinamon leave-prevention. Synthesized from
    /// `cost: { place_link_card_as_bottom_digivolution: true }` + `outcome:
    /// prevent`. Like `TrashOwnLinkCardAndCancelLeave` but the player-chosen
    /// link card is placed as the carrier's bottom digivolution card (not
    /// trashed); owns both the cost and the cancel.
    PlaceLinkCardAsBottomSourceAndCancelLeave,
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

/// Compiled per-zone filter overrides for `select_union_zone`. Each `Some`
/// predicate replaces the step's shared `filter` for candidates from that
/// origin zone. G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompiledUnionZoneFilters {
    // NOTE: no `skip_serializing_if` — the embedded pack round-trips through
    // bincode (non-self-describing), so every field must always be encoded.
    #[serde(default)]
    pub hand: Option<CompiledPredicate>,
    #[serde(default)]
    pub trash: Option<CompiledPredicate>,
    #[serde(default)]
    pub material: Option<CompiledPredicate>,
}

impl CompiledUnionZoneFilters {
    /// True when no per-zone override is present (the shared `filter` applies
    /// uniformly). Lets serialization skip the field for legacy union steps.
    pub fn is_empty(&self) -> bool {
        self.hand.is_none() && self.trash.is_none() && self.material.is_none()
    }
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

/// collapse §2 — a `reveal_search` bucket in compiled form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledRevealSearchBucket {
    pub filter: CompiledPredicate,
    pub to: CompiledRevealSearchDest,
    pub max: u8,
    pub optional: bool,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledRevealSearchDest {
    Hand,
    Trash,
    /// Bottom of the owner's deck.
    Deck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledRevealRemainder {
    Top,
    Bottom,
    /// Player-elected top/bottom — lowers the remainder placement through
    /// `CompiledStackPosition::Choice`.
    Choose,
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
    /// The controller's own breeding-area permanent (the single breeding
    /// carrier), resolved directly with no selection prompt.
    /// G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY.
    OwnBreeding,
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
    /// collapse §3.1 — player-elected top/bottom (binary pick at run time).
    /// Only steps that intercept it before placement may carry it.
    Choice,
}

/// Phase 2 Track E (2026-05-17): compiled form of `RevealDestination` — the
/// typed routing for the `choose_from_reveal` step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledRevealDestination {
    Hand,
    DeckTop,
    DeckBottom,
    PlayFree,
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
                timing_conditions: Vec::new(),
                optional: false,
                outer_prompt: false,
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
                timing_conditions: Vec::new(),
                optional: false,
                outer_prompt: false,
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
