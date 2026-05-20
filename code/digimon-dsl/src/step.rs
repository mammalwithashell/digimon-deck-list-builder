//! Mutation verbs and control-flow forms for `process:` / `extra_cost:` /
//! `on_burst_turn_end:` step lists. Spec §3.7.
//!
//! The step model is a tagged enum with one variant per verb. The serde
//! representation uses a single-key map per step — e.g.
//! `gain_memory: 1`, `select_trash: { of: you, ... }` — so authors can
//! write natural YAML while the compiler sees a strict sum type.
//!
//! ## `If` step YAML shape
//!
//! Because serde's external-tag representation requires exactly one key as the
//! discriminant, the `if` step nests `condition`, `then`, and `else` **inside**
//! the `if:` map:
//!
//! ```yaml
//! - if:
//!     condition: { name_contains: Greymon }
//!     then:
//!       - gain_memory: 1
//!     else:
//!       - gain_memory: 2
//! ```
//!
//! A flat form (`if: <pred>\nthen: [...]`) would require an adjacent-tag or
//! untagged representation that conflicts with the single-discriminant pattern
//! used by every other variant, so we keep the nested form.
//!
//! ## serde_yml compatibility note
//!
//! `serde_yml` 0.0.12 does not support the standard serde external-tag
//! `{key: value}` map form when its `deserialize_enum` method is called
//! directly from the YAML event stream — it expects YAML `!tag` syntax.
//! The `Content`-buffering that serde uses for `#[serde(untagged)]` outer
//! enums happens to paper over this, which is why `process: Vec<StepSpec>`
//! inside `ClauseSpec` (which IS untagged) works fine.
//!
//! To make `StepSpec` work in ALL contexts (including `extra_cost:` inside
//! `AltPathSpec`, which is a plain struct), we implement `Deserialize`
//! manually.  The custom impl calls `d.deserialize_map(...)` — which serde_yml
//! handles for `MappingStart` events — and uses serde's
//! `MapAccessDeserializer` to feed the resulting map into the derive-generated
//! `__StepSpecHelper::deserialize`, which internally calls `deserialize_enum`
//! on a `MapAccessDeserializer` that properly handles the external-tag
//! one-key-map format.

use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use crate::common::PlayerRef;
use crate::predicate::{PredicateSpec, Zone};

/// A single step. Parsed from a one-key YAML map via a custom `Deserialize`
/// that calls `deserialize_map` to bypass serde_yml's `deserialize_enum`
/// limitation (see module-level docs).
///
/// Serialization uses a custom `Serialize` that emits a one-key map
/// `{verb: value}` (e.g. `gain_memory: 1`) so that `serde_yml::to_string`
/// round-trips correctly through the custom `Deserialize`.  The derive-
/// generated serializer would emit YAML tag syntax (`!gain_memory 1`) which
/// the custom deserializer cannot read back.
#[derive(Debug, Clone, PartialEq, schemars::JsonSchema)]
pub enum StepSpec {
    // Memory / turn
    GainMemory(i32),
    LoseMemory(i32),
    SetMemory(i32),
    /// Phase 2 Track F (G-DSL-GAIN-MEMORY-FN) — formula-valued gain.
    /// Mirrors the literal `gain_memory: N` shape but accepts a
    /// `FormulaSpec` evaluated at resolution time. Use for printed text
    /// like "[When Digivolving] Gain 1 memory for every 4 cards in your
    /// hand." (EX1-021 MetalGarurumon).
    GainMemoryFn(FormulaStepArgs),
    /// Symmetric of `GainMemoryFn` — kept for completeness so author-facing
    /// API doesn't surprise (literal `lose_memory: N` has a `lose_memory_fn`
    /// sibling). No known card uses it as of 2026-05-17 but adding both
    /// halves at once keeps the eval-arm coverage matrix uniform.
    LoseMemoryFn(FormulaStepArgs),

    // Draw / deck / hand / trash
    Draw(DrawArgs),
    TrashFromTop(DrawArgs),
    AddToHandFromDeck(HandleMoveArgs),
    AddToHandFromTrash(HandleMoveArgs),
    AddToHandFromSecurity(HandleMoveArgs),
    AddTopSecurityToHand(PlayerArg),
    MayAddTopSecurityToHand(PlayerArg),
    AddToHandFromReveal(HandleMoveArgs),
    AddThisOptionToHand(EmptyArgs),
    TrashFromHandByIndex(IndexedMoveArgs),
    TrashFromReveal(HandleMoveArgs),
    ReturnToDeckFromReveal(ReturnToDeckArgs),
    ShuffleDeck(PlayerArg),
    ShuffleSecurity(PlayerArg),
    RevealTopDeck(RevealArgs),
    PlaceRemainderOnDeck(PlaceRemainderArgs),
    /// Phase 2 Track E (2026-05-17): pick one card from the current reveal
    /// pool and route it to a single typed destination. Ergonomic combo of
    /// `select_reveal` + `{add_to_hand_from_reveal,return_to_deck_from_reveal,
    /// place_as_bottom_source}`. Pair with `order_remainder` for the
    /// "reveal N, choose 1 to hand/source, place rest top-or-bottom in any
    /// order" pattern that recurs across Rocks searchers and general training
    /// effects.
    ChooseFromReveal(ChooseFromRevealArgs),
    /// Phase 2 Track E (2026-05-17): place all remaining revealed cards onto
    /// the controller's deck. Unlike `place_remainder_on_deck`, the
    /// destination (top vs bottom) can itself be a player choice when the
    /// printed text reads "top or bottom" (P-167 et al). Always surfaces the
    /// `select_ordered_permutation` ordering selection per Working Rule §17.
    OrderRemainder(OrderRemainderArgs),

    // Field / permanent
    DeletePermanent(TargetArg),
    DeleteBoundPermanents(DeleteBoundPermanentsArgs),
    ReturnToHand(TargetArg),
    ReturnToDeck(ReturnPermanentArgs),
    Suspend(TargetArg),
    Unsuspend(TargetArg),
    DeDigivolve(DeDigivolveArgs),
    PlaceOnSecurity(PlaceOnSecurityArgs),
    PlayToken(PlayTokenArgs),
    PlaceAsBottomSource(PlaceAsBottomSourceArgs),
    /// Phase 2 Track F (2026-05-17): move `target`'s top stacked card (the
    /// digivolution source immediately beneath the active top card) to the
    /// bottom of its own stack. Closes G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM
    /// (BT23-008 / BT23-018-shape "place top stacked card as bottom" costs).
    /// Per the no-approximations policy this is a deterministic source pick
    /// — the printed text identifies a singular top source, so no
    /// `select_material` choice is exposed.
    PlaceTopSourceAsBottom(TargetArg),
    TrashTopSource(TargetArg),
    TrashAllSources(TargetArg),
    TrashSelectedSources(TrashSelectedSourcesArgs),
    BindPermanentProperty(BindPermanentProperty),
    Hatch(PlayerArg),

    // Play / digivolve
    PlayFromHand(PlayFromHandArgs),
    PlayFromHandFree(PlayFromHandFreeArgs),
    PlayFromTrash(PlayFromHandArgs),
    PlayFromTrashFree(PlayFromHandArgs),
    PlayFromSecurity(PlayFromSecurityArgs),
    PlayFromMaterials(PlayFromMaterialsArgs),
    PlaySelectedSourcesFree(TrashSelectedSourcesArgs),
    EffectInitiatedDigivolve(EffectDigivolveArgs),
    EffectInitiatedDnaDigivolve(EffectDnaDigivolveArgs),

    // Security
    TrashTopSecurity(PlayerArg),
    TrashTopSecurityAndCancelReplacement(PlayerArg),
    BounceSelf(EmptyArgs),
    PlaceSelfAtSecurity(SelfSecurityPlacementArgs),
    PlaceSelfOptionAtSecurity(SelfSecurityPlacementArgs),
    PlacePermanentBottomSecurityAndCancelReplacement(PlacePermanentSecurityReplacementArgs),
    PlacePermanentOnSecurity(PlacePermanentOnSecurityReplacementArgs),
    PlacePermanentOnSecurityAndHandleReplacement(PlacePermanentOnSecurityReplacementArgs),
    PlacePermanentOnSecurityObserved(PlacePermanentOnSecurityObservedArgs),
    SecurityPlaceStackedCard(SecurityPlaceStackedCardArgs),
    SecurityPlaceTopStackedCard(SecurityPlaceTopStackedCardArgs),
    ReturnAllTrashToDeckBottom(PlayerArg),
    TrashTopNDigivolutionCardsOfEach(TrashTopNDigivolutionCardsOfEachArgs),
    TrashOpponentHandToCount(TrashOpponentHandToCountArgs),
    SearchOwnSecurityStack(SearchOwnSecurityStackArgs),
    Recover(DrawArgs),
    MarkSecurityFaceUp(MarkSecurityArgs),

    // Modifiers
    AddDpModifier(AddDpModifierArgs),
    AddModifier(AddModifierArgs),
    AddPlayerModifier(AddPlayerModifierArgs),
    GrantKeyword(GrantKeywordArgs),
    GrantEffectImmunity(GrantEffectImmunityArgs),
    /// Track H §3 — install a granted triggered effect on each
    /// permanent matching `target`. The granted body fires on the
    /// carrier's matching `timing` (DCGO `AddSkillClass.cs` analog).
    /// EX1-068 Ice Wall! is the canonical fixture.
    GrantTriggeredEffect(GrantTriggeredEffectArgs),

    // Selection
    SelectOwnPermanent(SelectFieldArgs),
    SelectOpponentPermanent(SelectFieldArgs),
    SelectAnyPermanent(SelectFieldArgs),
    SelectDnaPair(SelectDnaPairArgs),
    SelectHand(SelectZoneArgs),
    SelectTrash(SelectZoneArgs),
    SelectMaterial(SelectMaterialArgs),
    SelectOwnSources(SelectOwnSourcesArgs),
    DigiBurst(DigiBurstArgs),
    SelectOpponentDpBudget(SelectOpponentDpBudgetArgs),
    SelectOwnBreedingPermanent(SelectOwnBreedingPermanentArgs),
    SelectReveal(SelectZoneArgs),
    SelectRevealBuckets(SelectRevealBucketsArgs),
    SelectSecurity(SelectZoneArgs),
    SelectUnionZone(SelectUnionArgs),
    SelectOrderedPermutation(SelectPermutationArgs),
    SelectCountCappedMulti(SelectCountCappedArgs),
    SelectEffectChoice(SelectEffectChoiceArgs),
    AsSelectingPlayer(AsSelectingPlayerArgs),

    // Control flow
    If(IfStep),
    ForEach(ForEachStep),
    PerSelected(PerSelectedStep),
    ScheduleDelayed(ScheduleDelayedStep),
    PlaceSelfAsDelayOption(EmptyArgs),
    LinkToOwnDigimon(LinkToOwnDigimonArgs),
    Optional(OptionalStep),

    // Combat / replacement process outcomes
    Battle(BattleArgs),
    MayAttackNow(MayAttackNowArgs),
    ForceAttack(ForceAttackArgs),
    RedirectAttackTarget(RedirectAttackTargetArgs),
    CancelAttack(EmptyArgs),
    OpenCounterWindow(EmptyArgs),
    RefireEffect(RefireEffectArgs),
    EndAttack(bool),
    CancelReplacement(EmptyArgs),
    HandleReplacement(EmptyArgs),
    RedirectReplacement(RedirectReplacementArgs),
    SubstituteReplacement(SubstituteReplacementArgs),

    // Escape hatch (step-level)
    RawRust(RawRustStep),

    // Phase 2 Track B — declarative activation-cost step. Only valid as
    // the first step of a triggered clause body; lifted onto
    // `EffectBuilder::activation_cost(...)` at lowering time. The
    // compile-side validator rejects mid-body uses.
    ActivationCost(ActivationCostArgs),
}

// ── Custom Serialize for StepSpec ──────────────────────────────────────
//
// We emit a one-key map `{verb: value}` instead of the YAML tag form
// `!verb value` that serde's derive-generated Serialize would produce.
// This matches the format the custom Deserialize expects, enabling
// round-trip: `format_spec(parse(format_spec(spec))) == format_spec(spec)`.

impl Serialize for StepSpec {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Helper macro: serialize as a one-key map {verb: inner}
        macro_rules! kv {
            ($map:expr, $key:literal, $val:expr) => {{
                let mut m = $map.serialize_map(Some(1))?;
                m.serialize_entry($key, $val)?;
                m.end()
            }};
        }
        match self {
            // Memory / turn
            StepSpec::GainMemory(v) => kv!(s, "gain_memory", v),
            StepSpec::LoseMemory(v) => kv!(s, "lose_memory", v),
            StepSpec::SetMemory(v) => kv!(s, "set_memory", v),
            StepSpec::GainMemoryFn(v) => kv!(s, "gain_memory_fn", v),
            StepSpec::LoseMemoryFn(v) => kv!(s, "lose_memory_fn", v),
            // Draw / deck / hand / trash
            StepSpec::Draw(v) => kv!(s, "draw", v),
            StepSpec::TrashFromTop(v) => kv!(s, "trash_from_top", v),
            StepSpec::AddToHandFromDeck(v) => kv!(s, "add_to_hand_from_deck", v),
            StepSpec::AddToHandFromTrash(v) => kv!(s, "add_to_hand_from_trash", v),
            StepSpec::AddToHandFromSecurity(v) => kv!(s, "add_to_hand_from_security", v),
            StepSpec::AddTopSecurityToHand(v) => kv!(s, "add_top_security_to_hand", v),
            StepSpec::MayAddTopSecurityToHand(v) => kv!(s, "may_add_top_security_to_hand", v),
            StepSpec::AddToHandFromReveal(v) => kv!(s, "add_to_hand_from_reveal", v),
            StepSpec::AddThisOptionToHand(v) => kv!(s, "add_this_option_to_hand", v),
            StepSpec::TrashFromHandByIndex(v) => kv!(s, "trash_from_hand_by_index", v),
            StepSpec::TrashFromReveal(v) => kv!(s, "trash_from_reveal", v),
            StepSpec::ReturnToDeckFromReveal(v) => kv!(s, "return_to_deck_from_reveal", v),
            StepSpec::ShuffleDeck(v) => kv!(s, "shuffle_deck", v),
            StepSpec::ShuffleSecurity(v) => kv!(s, "shuffle_security", v),
            StepSpec::RevealTopDeck(v) => kv!(s, "reveal_top_deck", v),
            StepSpec::PlaceRemainderOnDeck(v) => kv!(s, "place_remainder_on_deck", v),
            StepSpec::ChooseFromReveal(v) => kv!(s, "choose_from_reveal", v),
            StepSpec::OrderRemainder(v) => kv!(s, "order_remainder", v),
            // Field / permanent
            StepSpec::DeletePermanent(v) => kv!(s, "delete_permanent", v),
            StepSpec::DeleteBoundPermanents(v) => kv!(s, "delete_bound_permanents", v),
            StepSpec::ReturnToHand(v) => kv!(s, "return_to_hand", v),
            StepSpec::ReturnToDeck(v) => kv!(s, "return_to_deck", v),
            StepSpec::Suspend(v) => kv!(s, "suspend", v),
            StepSpec::Unsuspend(v) => kv!(s, "unsuspend", v),
            StepSpec::DeDigivolve(v) => kv!(s, "de_digivolve", v),
            StepSpec::PlaceOnSecurity(v) => kv!(s, "place_on_security", v),
            StepSpec::PlayToken(v) => kv!(s, "play_token", v),
            StepSpec::PlaceAsBottomSource(v) => kv!(s, "place_as_bottom_source", v),
            StepSpec::PlaceTopSourceAsBottom(v) => kv!(s, "place_top_source_as_bottom", v),
            StepSpec::TrashTopSource(v) => kv!(s, "trash_top_source", v),
            StepSpec::TrashAllSources(v) => kv!(s, "trash_all_sources", v),
            StepSpec::TrashSelectedSources(v) => kv!(s, "trash_selected_sources", v),
            StepSpec::BindPermanentProperty(v) => kv!(s, "bind_permanent_property", v),
            StepSpec::Hatch(v) => kv!(s, "hatch", v),
            // Play / digivolve
            StepSpec::PlayFromHand(v) => kv!(s, "play_from_hand", v),
            StepSpec::PlayFromHandFree(v) => kv!(s, "play_from_hand_free", v),
            StepSpec::PlayFromTrash(v) => kv!(s, "play_from_trash", v),
            StepSpec::PlayFromTrashFree(v) => kv!(s, "play_from_trash_free", v),
            StepSpec::PlayFromSecurity(v) => kv!(s, "play_from_security", v),
            StepSpec::PlayFromMaterials(v) => kv!(s, "play_from_materials", v),
            StepSpec::PlaySelectedSourcesFree(v) => kv!(s, "play_selected_sources_free", v),
            StepSpec::EffectInitiatedDigivolve(v) => kv!(s, "effect_initiated_digivolve", v),
            StepSpec::EffectInitiatedDnaDigivolve(v) => kv!(s, "effect_initiated_dna_digivolve", v),
            // Security
            StepSpec::TrashTopSecurity(v) => kv!(s, "trash_top_security", v),
            StepSpec::TrashTopSecurityAndCancelReplacement(v) => {
                kv!(s, "trash_top_security_and_cancel_replacement", v)
            }
            StepSpec::BounceSelf(v) => kv!(s, "bounce_self", v),
            StepSpec::PlaceSelfAtSecurity(v) => kv!(s, "place_self_at_security", v),
            StepSpec::PlaceSelfOptionAtSecurity(v) => {
                kv!(s, "place_self_option_at_security", v)
            }
            StepSpec::PlacePermanentBottomSecurityAndCancelReplacement(v) => {
                kv!(
                    s,
                    "place_permanent_bottom_security_and_cancel_replacement",
                    v
                )
            }
            StepSpec::PlacePermanentOnSecurity(v) => kv!(s, "place_permanent_on_security", v),
            StepSpec::PlacePermanentOnSecurityAndHandleReplacement(v) => {
                kv!(s, "place_permanent_on_security_and_handle_replacement", v)
            }
            StepSpec::PlacePermanentOnSecurityObserved(v) => {
                kv!(s, "place_permanent_on_security_observed", v)
            }
            StepSpec::SecurityPlaceStackedCard(v) => kv!(s, "security_place_stacked_card", v),
            StepSpec::SecurityPlaceTopStackedCard(v) => {
                kv!(s, "security_place_top_stacked_card", v)
            }
            StepSpec::ReturnAllTrashToDeckBottom(v) => {
                kv!(s, "return_all_trash_to_deck_bottom", v)
            }
            StepSpec::TrashTopNDigivolutionCardsOfEach(v) => {
                kv!(s, "trash_top_n_digivolution_cards_of_each", v)
            }
            StepSpec::TrashOpponentHandToCount(v) => kv!(s, "trash_opponent_hand_to_count", v),
            StepSpec::SearchOwnSecurityStack(v) => kv!(s, "search_own_security_stack", v),
            StepSpec::Recover(v) => kv!(s, "recover", v),
            StepSpec::MarkSecurityFaceUp(v) => kv!(s, "mark_security_face_up", v),
            // Modifiers
            StepSpec::AddDpModifier(v) => kv!(s, "add_dp_modifier", v),
            StepSpec::AddModifier(v) => kv!(s, "add_modifier", v),
            StepSpec::AddPlayerModifier(v) => kv!(s, "add_player_modifier", v),
            StepSpec::GrantKeyword(v) => kv!(s, "grant_keyword", v),
            StepSpec::GrantTriggeredEffect(v) => kv!(s, "grant_triggered_effect", v),
            StepSpec::GrantEffectImmunity(v) => kv!(s, "grant_effect_immunity", v),
            // Selection
            StepSpec::SelectOwnPermanent(v) => kv!(s, "select_own_permanent", v),
            StepSpec::SelectOpponentPermanent(v) => kv!(s, "select_opponent_permanent", v),
            StepSpec::SelectAnyPermanent(v) => kv!(s, "select_any_permanent", v),
            StepSpec::SelectDnaPair(v) => kv!(s, "select_dna_pair", v),
            StepSpec::SelectHand(v) => kv!(s, "select_hand", v),
            StepSpec::SelectTrash(v) => kv!(s, "select_trash", v),
            StepSpec::SelectMaterial(v) => kv!(s, "select_material", v),
            StepSpec::SelectOwnSources(v) => kv!(s, "select_own_sources", v),
            StepSpec::DigiBurst(v) => kv!(s, "digi_burst", v),
            StepSpec::SelectOpponentDpBudget(v) => kv!(s, "select_opponent_dp_budget", v),
            StepSpec::SelectOwnBreedingPermanent(v) => {
                kv!(s, "select_own_breeding_permanent", v)
            }
            StepSpec::SelectReveal(v) => kv!(s, "select_reveal", v),
            StepSpec::SelectRevealBuckets(v) => kv!(s, "select_reveal_buckets", v),
            StepSpec::SelectSecurity(v) => kv!(s, "select_security", v),
            StepSpec::SelectUnionZone(v) => kv!(s, "select_union_zone", v),
            StepSpec::SelectOrderedPermutation(v) => kv!(s, "select_ordered_permutation", v),
            StepSpec::SelectCountCappedMulti(v) => kv!(s, "select_count_capped_multi", v),
            StepSpec::SelectEffectChoice(v) => kv!(s, "select_effect_choice", v),
            StepSpec::AsSelectingPlayer(v) => kv!(s, "as_selecting_player", v),
            // Control flow
            StepSpec::If(v) => kv!(s, "if", v),
            StepSpec::ForEach(v) => kv!(s, "for_each", v),
            StepSpec::PerSelected(v) => kv!(s, "per_selected", v),
            StepSpec::ScheduleDelayed(v) => kv!(s, "schedule_delayed", v),
            StepSpec::PlaceSelfAsDelayOption(v) => kv!(s, "place_self_as_delay_option", v),
            StepSpec::LinkToOwnDigimon(v) => kv!(s, "link_to_own_digimon", v),
            StepSpec::Optional(v) => kv!(s, "optional", v),
            // Combat / replacement process outcomes
            StepSpec::Battle(v) => kv!(s, "battle", v),
            StepSpec::MayAttackNow(v) => kv!(s, "may_attack_now", v),
            StepSpec::ForceAttack(v) => kv!(s, "force_attack", v),
            StepSpec::RedirectAttackTarget(v) => kv!(s, "redirect_attack_target", v),
            StepSpec::CancelAttack(v) => kv!(s, "cancel_attack", v),
            StepSpec::OpenCounterWindow(v) => kv!(s, "open_counter_window", v),
            StepSpec::RefireEffect(v) => kv!(s, "refire_effect", v),
            StepSpec::EndAttack(v) => kv!(s, "end_attack", v),
            StepSpec::CancelReplacement(v) => kv!(s, "cancel_replacement", v),
            StepSpec::HandleReplacement(v) => kv!(s, "handle_replacement", v),
            StepSpec::RedirectReplacement(v) => kv!(s, "redirect_replacement", v),
            StepSpec::SubstituteReplacement(v) => kv!(s, "substitute_replacement", v),
            // Escape hatch
            StepSpec::RawRust(v) => kv!(s, "raw_rust", v),
            StepSpec::ActivationCost(v) => kv!(s, "activation_cost", v),
        }
    }
}

// ── Custom Deserialize for StepSpec ────────────────────────────────────
//
// We call `d.deserialize_map(StepSpecVisitor)` so serde_yml dispatches to
// `deserialize_map` (which works for MappingStart) rather than
// `deserialize_enum` (which requires a YAML `!tag`).
//
// The visitor reads the one-key map and dispatches based on the key name.
// Each arm calls `map.next_value::<ArgType>()` which uses `deserialize_struct`
// (or a scalar deserializer for the i32 arms) — both work fine from the YAML
// stream.
//
// Nested `Vec<StepSpec>` fields (in IfStep, ForEachStep, etc.) recurse
// through this same custom impl transparently.

impl<'de> Deserialize<'de> for StepSpec {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(StepSpecVisitor)
    }
}

struct StepSpecVisitor;

impl<'de> Visitor<'de> for StepSpecVisitor {
    type Value = StepSpec;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a one-key map identifying a step verb")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<StepSpec, A::Error> {
        let key: String = map
            .next_key()?
            .ok_or_else(|| de::Error::custom("expected a step verb key, found empty map"))?;

        let step = match key.as_str() {
            // Memory / turn
            "gain_memory" => StepSpec::GainMemory(map.next_value()?),
            "lose_memory" => StepSpec::LoseMemory(map.next_value()?),
            "set_memory" => StepSpec::SetMemory(map.next_value()?),
            "gain_memory_fn" => StepSpec::GainMemoryFn(map.next_value()?),
            "lose_memory_fn" => StepSpec::LoseMemoryFn(map.next_value()?),

            // Draw / deck / hand / trash
            "draw" => StepSpec::Draw(map.next_value()?),
            "trash_from_top" => StepSpec::TrashFromTop(map.next_value()?),
            "add_to_hand_from_deck" => StepSpec::AddToHandFromDeck(map.next_value()?),
            "add_to_hand_from_trash" => StepSpec::AddToHandFromTrash(map.next_value()?),
            "add_to_hand_from_security" => StepSpec::AddToHandFromSecurity(map.next_value()?),
            "add_top_security_to_hand" => StepSpec::AddTopSecurityToHand(map.next_value()?),
            "may_add_top_security_to_hand" => StepSpec::MayAddTopSecurityToHand(map.next_value()?),
            "add_to_hand_from_reveal" => StepSpec::AddToHandFromReveal(map.next_value()?),
            "add_this_option_to_hand" => StepSpec::AddThisOptionToHand(map.next_value()?),
            "trash_from_hand_by_index" => StepSpec::TrashFromHandByIndex(map.next_value()?),
            "trash_from_reveal" => StepSpec::TrashFromReveal(map.next_value()?),
            "return_to_deck_from_reveal" => StepSpec::ReturnToDeckFromReveal(map.next_value()?),
            "shuffle_deck" => StepSpec::ShuffleDeck(map.next_value()?),
            "shuffle_security" => StepSpec::ShuffleSecurity(map.next_value()?),
            "reveal_top_deck" => StepSpec::RevealTopDeck(map.next_value()?),
            "place_remainder_on_deck" => StepSpec::PlaceRemainderOnDeck(map.next_value()?),
            "choose_from_reveal" => StepSpec::ChooseFromReveal(map.next_value()?),
            "order_remainder" => StepSpec::OrderRemainder(map.next_value()?),

            // Field / permanent
            "delete_permanent" => StepSpec::DeletePermanent(map.next_value()?),
            "delete_bound_permanents" => StepSpec::DeleteBoundPermanents(map.next_value()?),
            "return_to_hand" => StepSpec::ReturnToHand(map.next_value()?),
            "return_to_deck" => StepSpec::ReturnToDeck(map.next_value()?),
            "suspend" => StepSpec::Suspend(map.next_value()?),
            "unsuspend" => StepSpec::Unsuspend(map.next_value()?),
            "de_digivolve" => StepSpec::DeDigivolve(map.next_value()?),
            "place_on_security" => StepSpec::PlaceOnSecurity(map.next_value()?),
            "play_token" => StepSpec::PlayToken(map.next_value()?),
            "place_as_bottom_source" => StepSpec::PlaceAsBottomSource(map.next_value()?),
            "place_top_source_as_bottom" => StepSpec::PlaceTopSourceAsBottom(map.next_value()?),
            "trash_top_source" => StepSpec::TrashTopSource(map.next_value()?),
            "trash_all_sources" => StepSpec::TrashAllSources(map.next_value()?),
            "trash_selected_sources" => StepSpec::TrashSelectedSources(map.next_value()?),
            "bind_permanent_property" => StepSpec::BindPermanentProperty(map.next_value()?),
            "hatch" => StepSpec::Hatch(map.next_value()?),

            // Play / digivolve
            "play_from_hand" => StepSpec::PlayFromHand(map.next_value()?),
            "play_from_hand_free" => StepSpec::PlayFromHandFree(map.next_value()?),
            "play_from_trash" => StepSpec::PlayFromTrash(map.next_value()?),
            "play_from_trash_free" => StepSpec::PlayFromTrashFree(map.next_value()?),
            "play_from_security" => StepSpec::PlayFromSecurity(map.next_value()?),
            "play_from_materials" => StepSpec::PlayFromMaterials(map.next_value()?),
            "play_selected_sources_free" => StepSpec::PlaySelectedSourcesFree(map.next_value()?),
            "effect_initiated_digivolve" => StepSpec::EffectInitiatedDigivolve(map.next_value()?),
            "effect_initiated_dna_digivolve" => {
                StepSpec::EffectInitiatedDnaDigivolve(map.next_value()?)
            }

            // Security
            "trash_top_security" => StepSpec::TrashTopSecurity(map.next_value()?),
            "trash_top_security_and_cancel_replacement" => {
                StepSpec::TrashTopSecurityAndCancelReplacement(map.next_value()?)
            }
            "bounce_self" => StepSpec::BounceSelf(map.next_value()?),
            "place_self_at_security" => StepSpec::PlaceSelfAtSecurity(map.next_value()?),
            "place_self_option_at_security" => {
                StepSpec::PlaceSelfOptionAtSecurity(map.next_value()?)
            }
            "place_permanent_bottom_security_and_cancel_replacement" => {
                StepSpec::PlacePermanentBottomSecurityAndCancelReplacement(map.next_value()?)
            }
            "place_permanent_on_security" => StepSpec::PlacePermanentOnSecurity(map.next_value()?),
            "place_permanent_on_security_and_handle_replacement" => {
                StepSpec::PlacePermanentOnSecurityAndHandleReplacement(map.next_value()?)
            }
            "place_permanent_on_security_observed" => {
                StepSpec::PlacePermanentOnSecurityObserved(map.next_value()?)
            }
            "security_place_stacked_card" => StepSpec::SecurityPlaceStackedCard(map.next_value()?),
            "security_place_top_stacked_card" => {
                StepSpec::SecurityPlaceTopStackedCard(map.next_value()?)
            }
            "return_all_trash_to_deck_bottom" => {
                StepSpec::ReturnAllTrashToDeckBottom(map.next_value()?)
            }
            "trash_top_n_digivolution_cards_of_each" => {
                StepSpec::TrashTopNDigivolutionCardsOfEach(map.next_value()?)
            }
            "trash_opponent_hand_to_count" => StepSpec::TrashOpponentHandToCount(map.next_value()?),
            "search_own_security_stack" => StepSpec::SearchOwnSecurityStack(map.next_value()?),
            "recover" => StepSpec::Recover(map.next_value()?),
            "mark_security_face_up" => StepSpec::MarkSecurityFaceUp(map.next_value()?),

            // Modifiers
            "add_dp_modifier" => StepSpec::AddDpModifier(map.next_value()?),
            "add_modifier" => StepSpec::AddModifier(map.next_value()?),
            "add_player_modifier" => StepSpec::AddPlayerModifier(map.next_value()?),
            "grant_keyword" => StepSpec::GrantKeyword(map.next_value()?),
            "grant_triggered_effect" => StepSpec::GrantTriggeredEffect(map.next_value()?),
            "grant_effect_immunity" => StepSpec::GrantEffectImmunity(map.next_value()?),

            // Selection
            "select_own_permanent" => StepSpec::SelectOwnPermanent(map.next_value()?),
            "select_opponent_permanent" => StepSpec::SelectOpponentPermanent(map.next_value()?),
            "select_any_permanent" => StepSpec::SelectAnyPermanent(map.next_value()?),
            "select_dna_pair" => StepSpec::SelectDnaPair(map.next_value()?),
            "select_hand" => StepSpec::SelectHand(map.next_value()?),
            "select_trash" => StepSpec::SelectTrash(map.next_value()?),
            "select_material" => StepSpec::SelectMaterial(map.next_value()?),
            "select_own_sources" => StepSpec::SelectOwnSources(map.next_value()?),
            "digi_burst" => StepSpec::DigiBurst(map.next_value()?),
            "select_opponent_dp_budget" => StepSpec::SelectOpponentDpBudget(map.next_value()?),
            "select_own_breeding_permanent" => {
                StepSpec::SelectOwnBreedingPermanent(map.next_value()?)
            }
            "select_reveal" => StepSpec::SelectReveal(map.next_value()?),
            "select_reveal_buckets" => StepSpec::SelectRevealBuckets(map.next_value()?),
            "select_security" => StepSpec::SelectSecurity(map.next_value()?),
            "select_union_zone" => StepSpec::SelectUnionZone(map.next_value()?),
            "select_ordered_permutation" => StepSpec::SelectOrderedPermutation(map.next_value()?),
            "select_count_capped_multi" => StepSpec::SelectCountCappedMulti(map.next_value()?),
            "select_effect_choice" => StepSpec::SelectEffectChoice(map.next_value()?),
            "as_selecting_player" => StepSpec::AsSelectingPlayer(map.next_value()?),

            // Control flow
            "if" => StepSpec::If(map.next_value()?),
            "for_each" => StepSpec::ForEach(map.next_value()?),
            "per_selected" => StepSpec::PerSelected(map.next_value()?),
            "schedule_delayed" => StepSpec::ScheduleDelayed(map.next_value()?),
            "place_self_as_delay_option" => StepSpec::PlaceSelfAsDelayOption(map.next_value()?),
            "link_to_own_digimon" => StepSpec::LinkToOwnDigimon(map.next_value()?),
            "optional" => StepSpec::Optional(map.next_value()?),

            // Combat / replacement process outcomes
            "battle" => StepSpec::Battle(map.next_value()?),
            "may_attack_now" => StepSpec::MayAttackNow(map.next_value()?),
            "force_attack" => StepSpec::ForceAttack(map.next_value()?),
            "redirect_attack_target" => StepSpec::RedirectAttackTarget(map.next_value()?),
            "cancel_attack" => StepSpec::CancelAttack(map.next_value()?),
            "open_counter_window" => StepSpec::OpenCounterWindow(map.next_value()?),
            "refire_effect" => StepSpec::RefireEffect(map.next_value()?),
            "end_attack" => StepSpec::EndAttack(map.next_value()?),
            "cancel_replacement" => StepSpec::CancelReplacement(map.next_value()?),
            "handle_replacement" => StepSpec::HandleReplacement(map.next_value()?),
            "redirect_replacement" => StepSpec::RedirectReplacement(map.next_value()?),
            "substitute_replacement" => StepSpec::SubstituteReplacement(map.next_value()?),

            // Escape hatch
            "raw_rust" => StepSpec::RawRust(map.next_value()?),

            // Phase 2 Track B
            "activation_cost" => StepSpec::ActivationCost(map.next_value()?),

            other => {
                return Err(de::Error::unknown_variant(
                    other,
                    &[
                        "gain_memory",
                        "lose_memory",
                        "set_memory",
                        "gain_memory_fn",
                        "lose_memory_fn",
                        "draw",
                        "trash_from_top",
                        "add_to_hand_from_deck",
                        "add_to_hand_from_trash",
                        "add_to_hand_from_security",
                        "add_top_security_to_hand",
                        "may_add_top_security_to_hand",
                        "add_to_hand_from_reveal",
                        "add_this_option_to_hand",
                        "trash_from_hand_by_index",
                        "trash_from_reveal",
                        "return_to_deck_from_reveal",
                        "shuffle_deck",
                        "shuffle_security",
                        "reveal_top_deck",
                        "place_remainder_on_deck",
                        "choose_from_reveal",
                        "order_remainder",
                        "delete_permanent",
                        "delete_bound_permanents",
                        "return_to_hand",
                        "return_to_deck",
                        "suspend",
                        "unsuspend",
                        "de_digivolve",
                        "place_on_security",
                        "play_token",
                        "place_as_bottom_source",
                        "place_top_source_as_bottom",
                        "trash_top_source",
                        "trash_all_sources",
                        "trash_selected_sources",
                        "bind_permanent_property",
                        "hatch",
                        "play_from_hand",
                        "play_from_hand_free",
                        "play_from_trash",
                        "play_from_trash_free",
                        "play_from_security",
                        "play_from_materials",
                        "play_selected_sources_free",
                        "effect_initiated_digivolve",
                        "effect_initiated_dna_digivolve",
                        "trash_top_security",
                        "trash_top_security_and_cancel_replacement",
                        "bounce_self",
                        "place_self_at_security",
                        "place_self_option_at_security",
                        "place_permanent_bottom_security_and_cancel_replacement",
                        "place_permanent_on_security",
                        "place_permanent_on_security_and_handle_replacement",
                        "place_permanent_on_security_observed",
                        "security_place_stacked_card",
                        "security_place_top_stacked_card",
                        "return_all_trash_to_deck_bottom",
                        "trash_top_n_digivolution_cards_of_each",
                        "trash_opponent_hand_to_count",
                        "search_own_security_stack",
                        "recover",
                        "mark_security_face_up",
                        "add_dp_modifier",
                        "add_modifier",
                        "grant_keyword",
                        "grant_effect_immunity",
                        "select_own_permanent",
                        "select_opponent_permanent",
                        "select_any_permanent",
                        "select_dna_pair",
                        "select_hand",
                        "select_trash",
                        "select_material",
                        "select_own_sources",
                        "digi_burst",
                        "select_opponent_dp_budget",
                        "select_own_breeding_permanent",
                        "select_reveal",
                        "select_reveal_buckets",
                        "select_security",
                        "select_union_zone",
                        "select_ordered_permutation",
                        "select_count_capped_multi",
                        "select_effect_choice",
                        "as_selecting_player",
                        "if",
                        "for_each",
                        "per_selected",
                        "schedule_delayed",
                        "place_self_as_delay_option",
                        "link_to_own_digimon",
                        "optional",
                        "battle",
                        "may_attack_now",
                        "force_attack",
                        "redirect_attack_target",
                        "cancel_attack",
                        "open_counter_window",
                        "refire_effect",
                        "end_attack",
                        "cancel_replacement",
                        "handle_replacement",
                        "redirect_replacement",
                        "substitute_replacement",
                        "raw_rust",
                        "activation_cost",
                    ],
                ));
            }
        };
        Ok(step)
    }
}

// ── Binding references ──────────────────────────────────────────────

/// Used everywhere a step needs to identify a handle: a named binding (from
/// `bind_as:`), or a structured reference with explicit fields.
///
/// NOTE: `#[serde(untagged)]` means the more-specific variant
/// (`Structured`) is tried first so that a map `{ binding: "..." }` doesn't
/// accidentally match `Named`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum BindingRef {
    Structured(StructuredBindingRef),
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteBoundPermanentsArgs {
    pub binding: String,
}

/// Target of an `add_modifier:` step — either a named binding (from a
/// prior `bind_as:`) or a predicate filter that matches many permanents.
///
/// Untagged: serde tries `Binding(BindingRef)` first (scalar string or
/// structured-ref map with binding-specific keys), then falls back to
/// `Filter(PredicateSpec)`. Predicate fields like `kind` / `zone` / `of` /
/// `any_of` have no overlap with `StructuredBindingRef` fields (`binding`,
/// `permanent`, `source_permanent`, `of_permanent`), so disambiguation is
/// unambiguous. Note also that `StructuredBindingRef.zone` is `Option<Zone>`
/// (a scalar) while `PredicateSpec.zone` is `Vec<Zone>` (a sequence), so a
/// YAML sequence for `zone` will fail `StructuredBindingRef` and fall through
/// to `PredicateSpec` correctly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ModifierTarget {
    Binding(BindingRef),
    Filter(crate::predicate::PredicateSpec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StructuredBindingRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permanent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_permanent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of_permanent: Option<String>,
}

// ── Argument structs (one per verb family) ──────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayerArg {
    pub of: PlayerRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct EmptyArgs {}

/// Args for the `activation_cost:` DSL step. Phase 2 Track B.
///
/// YAML shape (only one variant key may be set):
/// ```yaml
/// - activation_cost:
///     suspend_self: true
/// ```
/// or
/// ```yaml
/// - activation_cost:
///     return_self_to_deck_bottom: true
/// ```
///
/// Only valid as the FIRST step of a triggered clause body — the
/// validator rejects mid-body uses. The lowering lifts it onto
/// `EffectBuilder::activation_cost(...)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ActivationCostArgs {
    #[serde(default, skip_serializing_if = "is_false")]
    pub suspend_self: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub return_self_to_deck_bottom: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetArg {
    pub target: BindingRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PermanentProperty {
    Level,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindPermanentProperty {
    pub from: BindingRef,
    pub property: PermanentProperty,
    pub bind_as: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BattleArgs {
    pub attacker: BindingRef,
    pub defender: BindingRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct AttackCostUpgradeArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_attack: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MayAttackNowArgs {
    pub attacker: BindingRef,
    #[serde(default)]
    pub targets: AttackTargetSpec,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub without_suspending: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_upgrade: Option<AttackCostUpgradeArgs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForceAttackArgs {
    pub attacker: BindingRef,
    #[serde(default)]
    pub targets: AttackTargetSpec,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub without_suspending: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_upgrade: Option<AttackCostUpgradeArgs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct RedirectAttackTargetArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_target: Option<BindingRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerRef>,
    #[serde(default)]
    pub targets: AttackTargetSpec,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RefireEffectArgs {
    pub source: BindingRef,
    pub timing: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum AttackTargetSpec {
    #[default]
    Any,
    Player,
    Digimon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RedirectReplacementArgs {
    pub zone: Zone,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubstituteReplacementArgs {
    pub subject: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrawArgs {
    pub of: PlayerRef,
    pub count: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HandleMoveArgs {
    pub of: PlayerRef,
    pub card: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IndexedMoveArgs {
    pub of: PlayerRef,
    pub hand_index: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnToDeckArgs {
    pub of: PlayerRef,
    pub card: BindingRef,
    pub position: StackPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnPermanentArgs {
    pub target: BindingRef,
    pub position: StackPosition,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_sources: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StackPosition {
    Top,
    Bottom,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecurityFace {
    Up,
    Down,
}

impl SecurityFace {
    pub fn is_up(self) -> bool {
        matches!(self, Self::Up)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelfSecurityPlacementArgs {
    pub position: StackPosition,
    pub face: SecurityFace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RevealArgs {
    pub of: PlayerRef,
    pub count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceRemainderArgs {
    pub of: PlayerRef,
    pub position: StackPosition,
}

/// Phase 2 Track E (2026-05-17): args for `choose_from_reveal`.
///
/// Picks one card from the current `revealed_cards` pool (optionally
/// filtered) and routes it to a typed destination. Supersedes the explicit
/// `select_reveal` → `add_to_hand_from_reveal` / `place_as_bottom_source`
/// combo for the common Rocks search shape.
///
/// ```yaml
/// - choose_from_reveal:
///     of: you
///     filter: { any_of: [trait_has: Mineral, trait_has: Rock] }
///     destination: hand
///     optional: true
///     prompt: "Add a Mineral or Rock card to your hand"
/// ```
///
/// The `destination` field is a tagged enum (see `RevealDestination`)
/// supporting hand, deck-top, deck-bottom, and bottom-source-of routings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChooseFromRevealArgs {
    pub of: PlayerRef,
    pub filter: PredicateSpec,
    pub destination: RevealDestination,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

/// Destination kinds supported by `choose_from_reveal`. Untagged for compact
/// YAML: a bare scalar (`hand`, `deck_top`, `deck_bottom`) or a mapping
/// (`bottom_source_of: { permanent: target }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RevealDestination {
    Hand,
    DeckTop,
    DeckBottom,
    /// Place the picked card as the bottom digivolution card of `target`.
    BottomSourceOf { target: BindingRef },
}

/// Phase 2 Track E (2026-05-17): args for `order_remainder`.
///
/// Places every card currently in the reveal pool onto the controller's
/// deck. If `destinations` lists a single position, behaves like
/// `place_remainder_on_deck`. If two destinations are listed
/// (`[deck_top, deck_bottom]`), surfaces an `effect_choice` so the player
/// picks where to place the remainder; the ordered permutation is exposed
/// either way (no auto-determinism — Working Rule §17).
///
/// ```yaml
/// - order_remainder:
///     of: you
///     destinations: [deck_top, deck_bottom]
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrderRemainderArgs {
    pub of: PlayerRef,
    pub destinations: Vec<RemainderDestination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemainderDestination {
    DeckTop,
    DeckBottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeDigivolveArgs {
    pub target: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_at_level: Option<u8>,
}

/// Phase 2 Track F (G-DSL-GAIN-MEMORY-FN): args for formula-valued
/// memory mutations (`gain_memory_fn:` / `lose_memory_fn:`). The single
/// `formula:` field evaluates at resolution time and the result is fed
/// to `EffectContext::add_memory` (signed) — exactly the shape the
/// existing literal `gain_memory: N` step uses, just with a runtime
/// integer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FormulaStepArgs {
    pub formula: crate::formula::FormulaSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceOnSecurityArgs {
    pub of: PlayerRef,
    pub source: BindingRef,
    pub position: StackPosition,
    #[serde(default)]
    pub face_up: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacePermanentSecurityReplacementArgs {
    #[serde(default = "default_player_ref_you")]
    pub of: PlayerRef,
    pub target: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacePermanentOnSecurityReplacementArgs {
    #[serde(default = "default_player_ref_you")]
    pub of: PlayerRef,
    pub target: BindingRef,
    pub position: StackPosition,
    #[serde(default)]
    pub face_up: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacePermanentOnSecurityObservedArgs {
    #[serde(default = "default_player_ref_you")]
    pub of: PlayerRef,
    pub target: BindingRef,
    pub position: StackPosition,
    pub face: SecurityFace,
    #[serde(default)]
    pub include_sources: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecurityPlaceStackedCardArgs {
    pub carrier: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<BindingRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_index_from_top: Option<u8>,
    #[serde(default = "default_player_ref_you", alias = "target_player")]
    pub of: PlayerRef,
    pub position: StackPosition,
    pub face: SecurityFace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecurityPlaceTopStackedCardArgs {
    pub carrier: BindingRef,
    #[serde(default = "default_player_ref_you", alias = "target_player")]
    pub of: PlayerRef,
    pub position: StackPosition,
    pub face: SecurityFace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashTopNDigivolutionCardsOfEachArgs {
    #[serde(alias = "target_player")]
    pub of: PlayerRef,
    pub n: crate::formula::FormulaSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashOpponentHandToCountArgs {
    pub opponent: PlayerRef,
    pub target_count: crate::formula::FormulaSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchOwnSecurityStackArgs {
    pub filter: PredicateSpec,
    #[serde(default = "default_search_own_security_prompt")]
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    pub on_select: Vec<StepSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_no_match: Option<Vec<StepSpec>>,
}

fn default_search_own_security_prompt() -> String {
    "Choose a card in your security".to_string()
}

fn default_player_ref_you() -> PlayerRef {
    PlayerRef::You
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayTokenArgs {
    pub controller: PlayerRef,
    pub token_name: String,
    /// Bind the resulting permanent handle for use in later steps in the
    /// same body. None (the default) preserves prior behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceAsBottomSourceArgs {
    pub source: BindingRef,
    pub target: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashSelectedSourcesArgs {
    pub source_refs: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromHandArgs {
    pub of: PlayerRef,
    pub hand_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
}

/// Free-play-from-hand args. Adds `bind_as` so the just-played permanent
/// handle can be referenced by subsequent steps (e.g. `schedule_delayed`
/// returning the played card at next opponent end turn).
/// G-PLAY-FROM-HAND-FREE-BIND-AS (Phase 2 Track H closure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromHandFreeArgs {
    pub of: PlayerRef,
    pub hand_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
    /// Bind the resulting permanent handle for use in later steps in the
    /// same body. None (the default) preserves prior behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CostDelta {
    Reduce { reduce: i32 },
    Keyword(CostDeltaKeyword),
    Literal(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CostDeltaKeyword {
    Free,
    Printed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromMaterialsArgs {
    pub target: BindingRef,
    pub source_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

/// Empty args struct for `play_from_security:` — the step carries no
/// parameters (the security card to play is implicit from the trigger context).
/// Using a dedicated struct rather than `serde_yml::Value` lets the external-
/// tag enum deserialize correctly with `serde_yml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromSecurityArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<PlayerRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectDigivolveArgs {
    pub target: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_hand: Option<BindingRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<BindingRef>,
    pub cost: CostDelta,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectDnaDigivolveArgs {
    pub target_a: BindingRef,
    pub target_b: BindingRef,
    pub from_hand: BindingRef,
    pub cost: CostDelta,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkSecurityArgs {
    pub of: PlayerRef,
    pub card: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddDpModifierArgs {
    pub target: BindingRef,
    pub value: ModifierValueSpec,
    pub expiry: String, // parsed as enum in Task 12 validation
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddModifierArgs {
    pub target: ModifierTarget,
    pub modifier: String,
    pub value: ModifierValueSpec,
    pub expiry: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddPlayerModifierArgs {
    pub target_player: PlayerRef,
    pub modifier: String,
    pub expiry: String,
}

/// Value carried by `add_dp_modifier` / `add_modifier`. Accepts either a
/// bare integer (`value: 3000`) or a `formula:`-wrapped block
/// (`value: { formula: { base: 0, per: stack_size, delta: 1000 } }`).
///
/// The wrapper key mirrors `alt_path::CostSpec` / `alt_path::FormulaCost` so
/// authors see one consistent shape across cost and modifier formulas.
///
/// Untagged: serde tries `Literal(i32)` first (a YAML scalar int), falling
/// through to `Formula` (a YAML mapping with a `formula:` key) when the
/// scalar match fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ModifierValueSpec {
    Literal(i32),
    Formula(crate::alt_path::FormulaCost),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordArgs {
    pub target: BindingRef,
    pub keyword: String,
    pub expiry: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

/// Arguments for the Track H §3 `grant_triggered_effect` step. Walks
/// battle areas for `target` matches and installs a granted-triggered-
/// effect entry on each, whose body executes `body` (a step list) when
/// the carrier's matching `timing` event drains. DCGO reference:
/// `AddSkillClass.cs` — grantor publishes a per-timing closure that
/// returns granted effects; here we install once per match instead of
/// re-evaluating per fire (acceptable for printed-text fidelity since
/// most cards install at a specific event then leave the carrier set
/// frozen until expiry).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantTriggeredEffectArgs {
    /// Predicate selecting which permanents receive the granted body.
    /// Walks both players' battle areas; combine `owner: opponent` /
    /// `owner: you` with kind/trait/level filters as needed.
    pub target: PredicateSpec,
    /// The carrier-side timing that triggers the granted body. String
    /// form (snake_case) matching the engine's `EffectTiming` map —
    /// e.g. `when_attacking`, `on_deletion`, `on_suspend`,
    /// `on_play`, `on_digivolve`.
    pub timing: String,
    /// String form (snake_case) matching the engine's `Expiry` map —
    /// e.g. `permanent`, `end_of_turn`, `end_of_opponents_turn`,
    /// `end_of_opponents_next_turn`. EX1-068 Ice Wall! uses
    /// `end_of_opponents_next_turn`.
    pub expiry: String,
    /// Step list executed when the granted body fires. Runs with
    /// `EffectContext::source_card` = grantor (DCGO
    /// `EffectSourceCard`) and `source_permanent` = carrier (DCGO
    /// `EffectSourcePermanent`). v1: bodies are non-selection — they
    /// run inline after the printed-observer queue drains. Selection-
    /// driving bodies are tracked as a separate gap.
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectSourceKindSpec {
    Digimon,
    Tamer,
    Option,
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectControllerSpec {
    Any,
    Opponent,
    Own,
}

impl Default for EffectControllerSpec {
    fn default() -> Self {
        Self::Opponent
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantEffectImmunityArgs {
    pub target: BindingRef,
    pub source_kind: EffectSourceKindSpec,
    #[serde(default)]
    pub source_controller: EffectControllerSpec,
    pub expiry: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldSelector {
    LowestDp,
    HighestDp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectFieldArgs {
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<FieldSelector>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectDnaPairArgs {
    pub left_filter: PredicateSpec,
    pub right_filter: PredicateSpec,
    pub bind_left_as: String,
    pub bind_right_as: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectZoneArgs {
    pub of: PlayerRef,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectRevealBucketsArgs {
    pub from: String,
    pub buckets: Vec<SelectRevealBucketArgs>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_duplicate_cards: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectRevealBucketArgs {
    pub bind_as: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectMaterialArgs {
    pub of_permanent: BindingRef,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

fn default_select_sources_prompt() -> String {
    "Choose source cards".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectOwnSourcesArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<BindingRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<BindingRef>,
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    pub min: u8,
    pub max: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default = "default_select_sources_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

fn default_digi_burst_prompt() -> String {
    "Choose digivolution cards for <Digi-Burst>".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigiBurstArgs {
    pub count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default = "default_digi_burst_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectOpponentDpBudgetArgs {
    pub dp_budget: i32,
    #[serde(default)]
    pub min_picks: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectOwnBreedingPermanentArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    /// Optional predicate the selected breeding permanent must satisfy.
    /// Used by Royal Knights cards like BT13-093 that require the
    /// breeding permanent to be a specific named host (e.g.
    /// `[King Drasil_7D6]`) before authoring a placement clause.
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectUnionArgs {
    pub of: PlayerRef,
    pub zones: Vec<Zone>,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectPermutationArgs {
    pub items: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectCountCappedArgs {
    pub of: PlayerRef,
    pub zone: Zone,
    pub max: CountBound,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional_zero: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_by: Option<crate::alt_path::DistinctBy>,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CountBound {
    Literal(u8),
    Formula {
        formula: crate::formula::FormulaSpec,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectEffectChoiceArgs {
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default)]
    pub prompt: String,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AsSelectingPlayerArgs {
    pub of: PlayerRef,
    pub body: Vec<StepSpec>,
}

/// Control-flow: conditional branch.
///
/// YAML shape (external-tag requires single discriminant key `if:`):
///
/// ```yaml
/// - if:
///     condition: { name_contains: Greymon }
///     then:
///       - gain_memory: 1
///     else:
///       - gain_memory: 2
/// ```
///
/// `condition` is typed as `serde_yml::Value` so that predicate expression
/// forms not yet modelled in `PredicateSpec` (e.g. `equals:`, `count_ge:`)
/// can be used in authored YAML today; Task 7 will tighten this to a typed
/// enum once all predicate forms are enumerated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IfStep {
    #[schemars(with = "serde_json::Value")]
    pub condition: serde_yml::Value,
    pub then: Vec<StepSpec>,
    #[serde(default, rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_: Option<Vec<StepSpec>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForEachStep {
    pub over: PredicateSpec,
    pub bind_as: String,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PerSelectedStep {
    pub selection: String,
    pub bind_as: String,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScheduleDelayedStep {
    pub when: super::clause::Timing,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkToOwnDigimonArgs {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub free: bool,
    pub filter: PredicateSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OptionalStep(pub Vec<StepSpec>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawRustStep {
    #[serde(rename = "fn")]
    pub fn_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binds: Vec<String>,
}
