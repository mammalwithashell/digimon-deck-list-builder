//! Filter / predicate tree. Spec §3.8.
//!
//! `PredicateSpec` is a flat struct where every leaf predicate is an
//! `Option<_>` field and compound forms (`all_of` / `any_of` / `none_of`
//! / `not`) are sibling fields. At evaluation time (Phase 2) every
//! present field contributes an AND-joined constraint.
//!
//! NOTE: this struct deliberately does NOT set `deny_unknown_fields`
//! because it is flattened into several call-sites (MaterialSpec, field
//! predicates). Typos in leaf-predicate fields are silently dropped at
//! parse time; the semantic validator (Task 12) must re-check.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::common::PlayerRef;
use crate::formula::{AggregateSelector, FormulaSpec};
use crate::spec::{CardKind, ColorSpec};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct PredicateSpec {
    // Leaf — card/permanent identity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<CardKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_eq: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_eq_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_matches_aggregate: Option<LevelAggregatePredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_matches_aggregate: Option<MaterialCountAggregatePredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_is: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_only: Option<Vec<ColorSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_matches_any_field_digimon: Option<PlayerRefSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_matches_binding: Option<String>,
    /// True when the candidate card shares ≥1 color with ANY card recorded in
    /// this effect's `returned_to_deck` result log (the cards a preceding
    /// `return_trash_list_to_deck_bottom` / `return_all_trash_to_deck_bottom`
    /// moved). The returned card never becomes a permanent, so it cannot be a
    /// permanent binding — this leaf reads the result log directly rather than a
    /// binding name. Candidate side is kind-aware exactly like
    /// `color_matches_binding`. G-RETURNED-CARD-COLOR-BINDING (driver EX10-068).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_matches_returned_card: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "trait",
        alias = "subject_trait"
    )]
    pub trait_has: Option<String>,
    /// Substring sibling of `trait_has`. Matches when ANY of the subject's
    /// traits CONTAINS this token (case-insensitive substring), mirroring
    /// DCGO `CardSource.ContainsTraits`. `trait_has` is an EXACT
    /// case-insensitive match; `trait_contains` is the substring reading
    /// demanded by printed text of the form "[Dragon], [saur] or
    /// [Ceratopsian] in any of its traits" — where e.g. `saur` only ever
    /// appears inside `Dinosaur` / `Plesiosaur` and `Dragon` mostly inside
    /// `Dragonkin` / `Dark Dragon`. Threaded identically to `trait_has`,
    /// including synth-identity / `ChangeTraits` overlay visibility.
    /// G-DSL-TRAIT-CONTAINS-SUBSTRING. Driver: EX3-014 Dorbickmon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    /// Case-insensitive substring scan against the candidate card's
    /// printed text — `effect_text`, `inherited_text`, and
    /// `security_text` concatenated. Distinct from `name_contains`,
    /// which only scans `card_name`. Used by BT22-017's bucket 1
    /// ("1 card with [Omnimon] in its text"). DCGO `source.HasText(s)`.
    /// G-DSL-PREDICATE-TEXT-CONTAINS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_text_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_in: Option<Vec<String>>,
    /// Card-subject leaf: true when NO battle-area Digimon belonging to the
    /// scoped player shares the candidate card's name. Models the printed
    /// "This effect can't play cards with the same names as any of your
    /// Digimon" exclusion on the Jesmon family (BT23-013) — applied as a
    /// filter on a `select_union_zone` (hand+trash) play candidate set so
    /// the in-play names are masked out, never auto-picked.
    /// G-UNION-HAND-TRASH-NAME-EXCLUSION (Phase 2 Track J Task S2.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_not_shared_by_field_digimon: Option<PlayerRefSelector>,
    /// Card-subject leaf: true when NO battle-area Tamer belonging to the
    /// scoped player shares the candidate card's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_not_shared_by_field_tamer: Option<PlayerRefSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_number_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_cost_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_cost_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_digivolve_from_source: Option<bool>,

    // Leaf — permanent-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_eq: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials_count_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_inherited: Option<Box<PredicateSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_suspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unsuspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_keyword: Option<String>,
    /// Permanent-subject predicate. True when the candidate currently has
    /// any Security Attack delta, whether from printed/granted
    /// `<Security A. +/-N>` keywords, temporary `SecurityAttackChange`
    /// modifiers, or formula-driven security-attack auras.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_security_attack_change: Option<bool>,
    /// Phase 2 Track F (G-DSL-HAS-ON-DELETION-EFFECT) — true when the
    /// permanent's top card (or any card in its digivolution stack) has a
    /// triggered effect with `EffectTiming::OnDeletion` either via a
    /// compiled DSL clause or a hand-written `CardEffect` impl. Used by
    /// EX1-021 MetalGarurumon's "[When Attacking] return 1 opponent
    /// Digimon **that has an [On Deletion] effect** to the bottom of
    /// deck." DCGO `permanent.HasOnDeletionEffect`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_on_deletion_effect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_color_count_gte: Option<u8>,
    /// Permanent-subject predicate. Matches whether the permanent's
    /// digivolution stack contains at least one face-down source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_face_down_source: Option<bool>,
    /// True when the observer's Tamers (battle-area Tamer permanents)
    /// collectively have at least N distinct colors. A no-subject
    /// global predicate — does not inspect the candidate. Used by
    /// ST20-10's warp-into-WarGreymon alt-path condition ("your Tamers
    /// have 3 or more total colors"). G-DSL-DISTINCT-TAMER-COLORS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct_tamer_colors_gte: Option<u8>,
    /// True when this effect's carrier is currently battling an opposing
    /// Digimon with zero digivolution source cards. Used by inherited
    /// battle-only auras such as ST2-01 Tsunomon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battle_opponent_no_sources: Option<bool>,

    // Leaf — zone / owner
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub zone: Vec<Zone>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "of")]
    pub owner: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<bool>,
    /// `is_source: true` — the subject permanent must BE the effect's source
    /// permanent (the mirror of `other: true`). Use it to filter a select
    /// down to "this Digimon" — e.g. DCGO's standalone "Will you unsuspend
    /// this card?" prompt becomes an optional `select_own_permanent` with
    /// `is_source: true`, exposing the Yes/No to the RL action space.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of_permanent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_in_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_owner: Option<BindingOwnerPredicate>,
    /// True when the card bound to `binding` has the given card category.
    /// Resolves the named card binding (e.g. from `reveal_top_deck { bind_as }`)
    /// and compares its printed kind. Used by LM-020 Quantumon to test whether
    /// the revealed opponent deck-top matches the declared category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_card_kind: Option<BindingCardKindPredicate>,

    // Leaf — source-relative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_tamer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_unsuspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_permanent_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_digivolution_contains_name: Option<String>,
    /// Source-subject predicate (Tamer face-down stash). Matches `CardSource.face_down`.
    /// Only meaningful when the predicate subject is a digivolution-stack source
    /// (e.g. inside a `select_own_sources` filter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_face_down: Option<bool>,
    /// Source-subject predicate. Matches whether the source sits at
    /// `card_sources` index 0 (the bottom of the digivolution stack).
    /// Only meaningful when the predicate subject is a digivolution-stack source
    /// (e.g. inside a `select_own_sources` filter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_bottom_source: Option<bool>,
    /// Source-subject predicate. Matches the `CardKind` of the host
    /// permanent's top card (e.g. `tamer` for a source stashed under a Tamer).
    /// Only meaningful when the predicate subject is a digivolution-stack source
    /// (e.g. inside a `select_own_sources` filter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_kind_is: Option<CardKind>,
    /// Like `self_digivolution_contains_name` but scans ONLY the
    /// digivolution *source* cards beneath the carrier — the carrier's
    /// own top card is excluded. `self_digivolution_contains_name` calls
    /// `Permanent::contains_card_name`, which scans the top card too, so
    /// a card named "Omnimon (X Antibody)" always self-matches "Omnimon"
    /// and the negative case ("no Omnimon among the digivolution cards")
    /// is inexpressible. BT20-102 needs the sources-only scan.
    /// G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_digivolution_sources_contain_name: Option<String>,
    /// Like `self_digivolution_sources_contain_name`, but matches any
    /// digivolution source card carrying the named trait. Used by Royal
    /// Knights breeding-source effects to gate carriers that actually contain
    /// playable [Royal Knight] sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_digivolution_sources_trait_has: Option<String>,
    /// True when the carrier Digimon's printed rules text (effect_text +
    /// inherited_text + security_text of the top card) contains the given
    /// substring (case-insensitive). Evaluated against the subject permanent
    /// in an inherited-aura `while_condition` context.
    ///
    /// Card driver: BT16-055 Namakemon — "[All Turns] While this Digimon has
    /// [Pulsemon] in its text, it gets +1000 DP." PUPPETS-G025.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_text_contains: Option<String>,

    // Leaf — global / observer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_gte: Option<DpConstraint>,
    /// Memory from the perspective of the predicate's CONTROLLER (the
    /// effect's owner), unlike `memory_lte`/`memory_gte` which compare the
    /// raw turn-player-perspective gauge. "While you have 0 or less memory"
    /// (EX8-073 / BT17-016 immunity) is `own_memory_lte: 0` — true when the
    /// controller's signed memory (the gauge when it is their turn, the
    /// negated gauge otherwise) is at or below the bound.
    /// G-DSL-OWN-MEMORY-PREDICATE.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_memory_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_memory_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_count_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_count_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent_security_count_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent_security_count_gte: Option<DpConstraint>,
    /// True when the observer's face-up security-card count is at most this
    /// threshold. Face-up state lives in `Player.face_up_security`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_up_security_count_lte: Option<DpConstraint>,
    /// True when the observer's face-up security-card count is at least this
    /// threshold. Face-up state lives in `Player.face_up_security`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_up_security_count_gte: Option<DpConstraint>,
    /// True when the named player has NO face-up security card matching the
    /// given identity filter. Face-up state lives in
    /// `Player.face_up_security` (a `card_index` index set), which is
    /// unreachable from any other predicate leaf — security cards are raw
    /// `Card`s, not `Permanent`s, so `any_permanent { zone: [security] }`
    /// cannot see them and has no face-up discriminator. Models card text
    /// of the form "While you have no face-up [Name] security cards, ...".
    /// G-PRED-NO-FACE-UP-SECURITY-NAMED.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_face_up_security_named: Option<FaceUpSecurityNamedPredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_turn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents_turn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_turns: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_hatch: Option<PlayerRef>,
    /// True when the referenced player has attacked with at least one Digimon
    /// during the current turn. Supports normal `not` / `none_of` negation for
    /// printed text such as "if your opponent didn't attack with a Digimon this
    /// turn".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digimon_attacked_this_turn: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_breeding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_field: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dna_origin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_kind: Option<CardKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_trait_has: Option<String>,
    /// Match the event-target permanent's printed level. Works for live
    /// event targets (played/digivolved/moved/suspended permanents) and
    /// deleted-object snapshots. G-EVENT-TARGET-LEVEL-LTE.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_level_eq: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_level_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_level_gte: Option<DpConstraint>,
    /// Match the event target's effective DP. Deletion events read the
    /// deleted-object snapshot captured immediately before removal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_dp_eq: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_dp_lte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_dp_gte: Option<DpConstraint>,
    /// Case-insensitive substring scan against the *event target*
    /// permanent's card name — i.e. the digivolving / played / deleted
    /// permanent carried on the triggered-effect read context. Used by
    /// EX4-061's clause 2 ("if that Digimon has [Greymon] in its name").
    /// Sibling of `event_target_trait_has` / `event_target_kind`.
    /// G-EVENT-TARGET-NAME-CONTAINS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_owner: Option<PlayerRef>,
    /// Match when the *event target* permanent's printed color set
    /// intersects this list — i.e. the digivolving / played / deleted /
    /// suspended permanent on the triggered-effect read context has at
    /// least one of the listed colors. Sibling of `event_target_kind` /
    /// `event_target_trait_has`, using the same `event_target_card`
    /// resolver. Used by BT13-012's inherited clause ("when one of your
    /// red or yellow Tamers becomes suspended"). G-EVENT-TARGET-COLOR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_color_any_of: Option<Vec<ColorSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_permanent_is_source: Option<bool>,
    /// True when the current deletion event's target is this effect
    /// source's battle opponent and the source's carrier is still present.
    /// Used for inherited "deletes an opponent's Digimon in battle and
    /// survives" clauses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_deleted_battle_opponent: Option<bool>,
    /// True when the triggering event's host permanent is this effect's
    /// source permanent. Used by OnDigivolutionCardTrashed observers that
    /// care about "this Digimon's digivolution cards" rather than any own
    /// stack.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_host_permanent_is_source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_is_effect_initiated: Option<bool>,
    /// For `OnAddToHand` observers: the player whose hand gained cards
    /// (`TriggerContext.affected_player`) must match this player-ref, resolved
    /// relative to the observer (`you` / `opponent`). See G-ON-ADD-TO-HAND-OBSERVER.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_add_to_hand_player: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_is_player: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_is_source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_was_self: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_target_change_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_name_contains: Option<String>,
    /// Case-insensitive substring scan against the triggering event card's
    /// PRINTED text (effect / inherited / security). Sibling of
    /// `event_card_name_contains` (which matches the NAME) and the event-side
    /// analogue of the static `effect_text_contains`. Gates observers on
    /// "when you play a card with <X> in its text". G-DSL-EVENT-CARD-TEXT-CONTAINS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_text_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_level_eq: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_level_gte: Option<DpConstraint>,
    /// True when every color of the triggering event card is within the given
    /// set. Used to gate observers on "the just-played card is black/yellow
    /// only" without listing individual card names. Mirrors `color_only` but
    /// operates on the event payload rather than the predicate subject.
    /// PUPPETS-G023.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_color_only: Option<Vec<ColorSpec>>,
    /// True when the triggering event card has AT LEAST ONE of the listed
    /// colors (intersection / "has" semantics). Sibling of
    /// `event_card_color_only` (subset semantics — not a faithful
    /// substitute). Used by BT16-085's "when a blue or green Digimon
    /// digivolves" trigger gate. G-EVENT-CARD-COLOR-IS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_color_has: Option<Vec<ColorSpec>>,
    /// True when the triggering event card has exactly N distinct colors.
    /// Pair with `event_card_color_only` to express "exactly 2-color
    /// black/yellow". PUPPETS-G023.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_card_color_count: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_cause: Option<EventCauseSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_target_same_level_as_previous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_permanent_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trashed_source_trait_has: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trashed_source_card_id_is: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_cause: Option<ReplacementCauseSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_source_is_opponent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_subject_is_mine: Option<bool>,
    /// True when the card about to link in the active `WhenWouldLink` window
    /// (the standing-Digimon link subject) carries AT LEAST ONE of the listed
    /// traits. Used by a host-side `when_would_link_to_this` reducer to gate on
    /// the linking card's traits — "when a [Social]/[Tool]/[Game] trait card
    /// would link to this Digimon" (Gap 5 — BT25-004 / BT25-045). `None`
    /// outside a standing-link `WhenWouldLink` window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_link_card_trait_any_of: Option<Vec<String>>,

    // Binding comparisons
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Vec<serde_json::Value>>")]
    pub equals: Option<Vec<serde_yml::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Vec<serde_json::Value>>")]
    pub not_equals: Option<Vec<serde_yml::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_exists: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "binding_is_present")]
    pub binding_present: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "binding_is_none")]
    pub binding_absent: Option<String>,
    /// True when the named list-typed binding (a `source_refs`,
    /// permanent-list or card-list binding produced by a multi-select /
    /// `select_own_sources` step) holds exactly `n` entries. Used by
    /// EX4-073 clause C's "if you trashed 3 cards" tail. A scalar / single
    /// binding counts as 1; a missing binding counts as 0.
    /// G-DSL-BINDING-COUNT-EQ.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_count_eq: Option<BindingCountPredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_suspended_any_own_digimon: Option<bool>,
    /// Opponent-side sibling of `effect_suspended_any_own_digimon`. True
    /// when the current effect's result log records a suspend of any of
    /// the controller's OPPONENT's Digimon. Used by BT16-025 Paildramon
    /// clause 2 ("If this effect didn't suspend, unsuspend this Digimon").
    /// G-DSL-EFFECT-SUSPENDED-RESULT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_suspended_any_opponent_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "any_returned_card")]
    pub effect_returned_any_card: Option<bool>,
    /// Filtered variant of `effect_returned_any_card`. True when at least one
    /// card moved by a preceding return / zone-move step in the SAME effect
    /// satisfies the inner card-shape predicate. The inner predicate is
    /// evaluated as a `Card` subject against each returned card identity in
    /// the per-effect result log (`returned_to_deck`). Distinct field name
    /// from the bare-bool `any_returned_card` alias so the two never collide.
    /// Example: `returned_card_matching: { color_is: white, level_eq: 7 }`.
    /// G-ANY-RETURNED-CARD-PREDICATE — driver BT17-077 clause 1c.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returned_card_matching: Option<Box<PredicateSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_deleted_any_own_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_deleted_any_opponent_digimon: Option<bool>,
    /// True iff at least one OPPONENT Digimon deleted by THIS effect had
    /// pre-removal effective DP `>= N`. The DP-threshold sibling of
    /// `effect_deleted_any_opponent_digimon`; reads the per-deletion DP
    /// snapshot recorded in the effect-result log (the carrier is in trash by
    /// the time a rider evaluates, so the snapshot is the only faithful DP
    /// source). Driver: EX4-065 Trident Gaia
    /// ("If a Digimon with 13000 DP or more is deleted by this effect, trash
    /// the opponent's top security card"). G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_deleted_opponent_digimon_dp_gte: Option<DpConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_played_any_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_digivolved_any_digimon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_added_any_card_to_hand: Option<bool>,

    // Count aggregates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_lte: Option<CountAggregate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_gte: Option<CountAggregate>,

    // Existential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_permanent: Option<Box<ExistentialPredicate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_field_permanent: Option<Box<ExistentialPredicate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_permanent: Option<Box<ExistentialPredicate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_permanents: Option<Box<ExistentialPredicate>>,

    // Compound
    #[serde(skip_serializing_if = "Vec::is_empty", alias = "all")]
    pub all_of: Vec<PredicateSpec>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<PredicateSpec>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub none_of: Vec<PredicateSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<PredicateSpec>>,

    // Misc contextual predicates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_alt_path: Option<String>,

    /// BeforePayCost target card predicate. When present, the inner
    /// predicate is evaluated against the card whose cost is currently
    /// being computed (`cost_target_card` on the effect read context),
    /// treated as a `Card` subject. Fails when no cost target is active
    /// (i.e., outside `BeforePayCost` cost-calc dispatch). Use the full
    /// card-shape vocabulary inside: `trait_has`, `color_is`,
    /// `name_contains`, `level_eq`/`_lte`/`_gte`, `kind`, etc.
    ///
    /// Example: a cost-reduction clause that fires only when the card
    /// being digivolved into has the [Free] trait:
    ///
    /// ```yaml
    /// active_when:
    ///   your_turn: true
    ///   cost_target: { trait_has: Free }
    /// ```
    ///
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_target: Option<Box<PredicateSpec>>,

    /// True when the effect's `source_permanent` is the (or one of the)
    /// permanent(s) being digivolved by the action whose cost is being
    /// computed. Use to gate "When THIS Digimon would digivolve into …"
    /// printed semantics so the observer / cost reducer only fires when
    /// its carrier permanent is actually the digivolution target. Single
    /// entry for normal digivolve; both DNA materials for DNA digivolve.
    /// Always false outside cost-calc dispatch and for effects whose
    /// `source_permanent` is `None`.
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_is_cost_target_permanent: Option<bool>,

    /// Captures unrecognized fields for controlled extension. Validator
    /// (Task 12) checks this for typos in inline predicate positions.
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: IndexMap<String, serde_yml::Value>,
}

impl PredicateSpec {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingOwnerPredicate {
    pub binding: String,
    pub of: PlayerRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingCardKindPredicate {
    pub binding: String,
    pub kind: CardKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum DpConstraint {
    Literal(i32),
    Formula(FormulaSpec),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum DpConstraintDeserialize {
    Literal(i32),
    WrappedFormula { formula: FormulaSpec },
    Formula(FormulaSpec),
}

impl<'de> Deserialize<'de> for DpConstraint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = DpConstraintDeserialize::deserialize(deserializer)?;
        Ok(match helper {
            DpConstraintDeserialize::Literal(n) => Self::Literal(n),
            DpConstraintDeserialize::WrappedFormula { formula }
            | DpConstraintDeserialize::Formula(formula) => Self::Formula(formula),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementCauseSpec {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
    Overclock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventCauseSpec {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum PlayerRefSelector {
    Player(PlayerRef),
    Scoped { of: PlayerRef },
}

impl PlayerRefSelector {
    pub fn player(self) -> PlayerRef {
        match self {
            Self::Player(player) => player,
            Self::Scoped { of } => of,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LevelAggregatePredicate {
    pub selector: AggregateSelector,
    #[serde(default = "default_level_aggregate_of")]
    pub of: PlayerRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialCountAggregatePredicate {
    pub selector: AggregateSelector,
    #[serde(default = "default_level_aggregate_of")]
    pub of: PlayerRef,
}

/// Identity filter for the `no_face_up_security_named` predicate leaf.
/// Exactly one of `card_number_is` / `name_is` / `color_is` is required —
/// the leaf counts face-up security cards of `of`'s player matching that
/// filter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FaceUpSecurityNamedPredicate {
    #[serde(default = "default_face_up_security_of")]
    pub of: PlayerRef,
    /// Match a face-up security card by exact card id (e.g. "ST20-15").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_number_is: Option<String>,
    /// Match a face-up security card by exact (case-insensitive) name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
    /// Match a face-up security card by printed color (EX10-020 Puppetmon
    /// "[On Deletion] If you have no GREEN face-up security cards, …" —
    /// judge-quiz Q3 authoring).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_is: Option<ColorSpec>,
}

fn default_face_up_security_of() -> PlayerRef {
    PlayerRef::You
}

/// Payload for the `binding_count_eq` predicate leaf — checks that a named
/// list-typed binding holds exactly `n` entries. G-DSL-BINDING-COUNT-EQ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingCountPredicate {
    pub binding: String,
    pub n: u8,
}

fn default_level_aggregate_of() -> PlayerRef {
    PlayerRef::You
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CountAggregate {
    pub filter: Box<PredicateSpec>,
    pub n: DpConstraint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct ExistentialPredicate {
    pub of: PlayerRef,
    #[serde(flatten)]
    pub predicate: PredicateSpec,
}

impl Default for ExistentialPredicate {
    fn default() -> Self {
        Self {
            of: PlayerRef::You,
            predicate: PredicateSpec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    Deck,
    Trash,
    BattleArea,
    Security,
    Breeding,
    Reveal,
    #[serde(rename = "digi_egg_deck")]
    DigiEggDeck,
    Material,
}
