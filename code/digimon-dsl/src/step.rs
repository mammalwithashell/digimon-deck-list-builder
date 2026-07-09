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
use crate::formula::FormulaSpec;
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
    //
    // The magnitude is a `FormulaSpec` (unify-dsl-scalar-and-comparators): a
    // bare int `gain_memory: 1` deserializes to `FormulaSpec::Literal(1)`, a
    // map `gain_memory: { base: 0, per: ally_count, delta: 1 }` to the formula.
    // The retired `gain_memory_fn`/`lose_memory_fn` twin verbs survive only as
    // deserialize aliases (see `visit_map`) so existing card YAML keeps
    // parsing; they are absent from the advertised vocabulary. At compile time
    // a `Literal` routes to `CompiledStep::GainMemory(i32)` and any other
    // formula to `CompiledStep::GainMemoryFn { formula }`, so the compiled
    // output is byte-identical to the pre-unification encoding.
    GainMemory(FormulaSpec),
    LoseMemory(FormulaSpec),
    SetMemory(FormulaSpec),

    // Draw / deck / hand / trash
    Draw(DrawArgs),
    TrashFromTop(TrashFromTopArgs),
    AddToHandFromDeck(HandleMoveArgs),
    AddToHandFromTrash(HandleMoveArgs),
    AddToHandFromSecurity(HandleMoveArgs),
    /// Play a specific bound card FROM the security stack without paying its
    /// cost. The `card` binding is a `CardHandle` (typically produced by a
    /// prior `select_security` step). G-PLAY-SELECTED-SECURITY-CARD. Used by
    /// BT13-012 ("you may play 1 red or yellow Tamer card among it without
    /// paying its cost").
    PlaySecurityCard(HandleMoveArgs),
    /// Trash a specific bound card FROM a player's security stack. The `card`
    /// binding is a `CardHandle` (typically produced by a prior
    /// `select_security` step). G-TRASH-SELECTED-SECURITY. Used by BT24-018
    /// ("You may trash any 1 of your opponent's security cards").
    TrashSelectedSecurity(HandleMoveArgs),
    /// Move a specific bound card FROM a player's security stack to that
    /// player's deck (top or bottom; Digi-Eggs route to the digitama deck).
    /// The `card` binding is a `CardHandle` (typically from a prior
    /// `select_security` step). G-DSL-RETURN-SELECTED-SECURITY-TO-DECK. Used by
    /// LM-020 Quantumon ("place 1 card among them on top of your opponent's
    /// deck"). YAML: `return_selected_security_to_deck: { of, card, position }`.
    ReturnSelectedSecurityToDeck(ReturnToDeckArgs),
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
    /// collapse-dsl-step-idioms §2 — the reveal-search idiom as one composite
    /// verb. Lowers to the existing sequence (reveal_top_deck → a single
    /// `select_reveal_buckets` over all buckets → per-bucket reveal-move →
    /// place_remainder_on_deck); each bucket is an RL-visible pick honoring its
    /// `optional`. Cross-bucket de-dup is always on (a revealed card lands in
    /// at most one bucket). See `RevealSearchArgs`.
    RevealSearch(RevealSearchArgs),
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
    /// `delete_for_cost_reduction: { target: <binding> }` — delete AS A COST and
    /// reduce the in-flight cost by the deleted permanent's printed play cost.
    /// `G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST` (BT13-103).
    DeleteForCostReduction(TargetArg),
    DeleteBoundPermanents(DeleteBoundPermanentsArgs),
    /// G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR — per-color mandatory pick +
    /// batch delete (EX9-074 Kimeramon Branch B).
    DeleteOnePerOpponentColor(DeleteOnePerOpponentColorArgs),
    TrashBreedingPermanent(TrashBreedingPermanentArgs),
    ReturnToHand(TargetArg),
    ReturnToDeck(ReturnPermanentArgs),
    Suspend(TargetArg),
    Unsuspend(TargetArg),
    DeDigivolve(DeDigivolveArgs),
    PlaceOnSecurity(PlaceOnSecurityArgs),
    PlayToken(PlayTokenArgs),
    PlaceAsBottomSource(PlaceAsBottomSourceArgs),
    /// G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05): place a card as
    /// `target`'s TOP digivolution source — inserted directly beneath the
    /// active top card (DCGO `Permanent.AddDigivolutionCardsTop`, which
    /// `Insert(1, …)`s into its top-first `cardSources`). The permanent's
    /// top card / identity is unchanged (placing a digivolution card never
    /// changes what the permanent IS). Sibling of `place_as_bottom_source`
    /// with the same args shape. Consumers: EX9-074 Kimeramon ("as this
    /// Digimon's top digivolution card"), BT13-088 Belphemon: Sleep Mode
    /// ("on top of this Digimon's digivolution cards").
    PlaceAsTopSource(PlaceAsTopSourceArgs),
    /// Phase 2 Track F (2026-05-17): move `target`'s top stacked card (the
    /// digivolution source immediately beneath the active top card) to the
    /// bottom of its own stack. Closes G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM
    /// (BT23-008 / BT23-018-shape "place top stacked card as bottom" costs).
    /// Per the no-approximations policy this is a deterministic source pick
    /// — the printed text identifies a singular top source, so no
    /// `select_material` choice is exposed.
    PlaceTopSourceAsBottom(TargetArg),
    TrashTopSource(TargetArg),
    TrashBottomSources(TrashBottomSourcesArgs),
    TrashAllSources(TargetArg),
    TrashSelectedSources(TrashSelectedSourcesArgs),
    PlaceSelectedCardUnderTamer(PlaceSelectedCardUnderTamerArgs),
    PlaceSelectedSourcesUnderTamer(PlaceSelectedSourcesUnderTamerArgs),
    MoveMatchingSourcesUnderTamer(MoveMatchingSourcesUnderTamerArgs),
    TrashTopStackedSources(TrashTopStackedSourcesArgs),
    /// G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (2026-05-21) — return each
    /// `select_own_sources`-bound digivolution source card to its owner's
    /// hand. Mirrors `TrashSelectedSources` but routes the source `Card` to
    /// the owner's hand instead of trash; fires no `OnDigivolutionCardTrashed`
    /// (this is a return, not a trash). Closes BT12-031's Imperialdramon:
    /// Dragon Mode alt-cost.
    ReturnSelectedSourcesToHand(TrashSelectedSourcesArgs),
    /// G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM (2026-06-14) — deck-routing
    /// sibling of `ReturnSelectedSourcesToHand`. Return each
    /// `select_own_sources`-bound digivolution source card to its owner's deck
    /// (`position: top | bottom`). Like the to-hand verb this is a return, NOT
    /// a trash, so it fires no `OnDigivolutionCardTrashed`. Closes BT13-075
    /// Alphamon's would-leave self-protection cost (return 1 [X Antibody]/[Royal
    /// Knight] source to the BOTTOM OF YOUR DECK to prevent leaving).
    ReturnSelectedSourcesToDeck(ReturnSelectedSourcesToDeckArgs),
    TrashBottomFaceDownSourceUnderTamer(TrashBottomFaceDownSourceUnderTamerArgs),
    /// G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER (2026-06-15) — the multi-count /
    /// multi-Tamer sibling of `TrashBottomFaceDownSourceUnderTamer`. Pays a
    /// `count`-total bottom-face-down trash cost distributed across the
    /// controller's Tamers: each pick installs a real `select_own_permanent`
    /// over `{ kind: tamer, has_face_down_source: true }` and trashes that
    /// Tamer's bottom face-down source, re-evaluating eligibility before the
    /// next pick — so "2 from one Tamer" and "1 from each of two Tamers" are
    /// both reachable, every pick surfaced (no auto-resolve). When fewer than
    /// `count` face-down sources exist across all Tamers the cost is unpayable
    /// and the clause's remaining steps are skipped (`TailAlreadyRan`). Drives
    /// BT25-035 Cougarmon's "by trashing 2 bottom face-down cards from under any
    /// of your Tamers" cost.
    TrashBottomFaceDownSourcesUnderTamers(TrashBottomFaceDownSourcesUnderTamersArgs),
    /// G-DSL-LINK-TRASH-AS-COST (BT25-073 Dragomon) — the ACTIVATION-cost
    /// sibling of the replacement-only `cost: { trash_own_link_card: true }`.
    /// Picks one of `of`'s Digimon that carries ≥1 link card (first selection),
    /// then one of ITS link cards (second selection), trashes it (firing
    /// `OnLinkedCardTrashed`), and runs the step's tail ONLY if the trash
    /// happened (no-approximations cost gate). When no own Digimon has a link
    /// card the cost is unpayable → the clause's remaining steps are skipped
    /// (`TailAlreadyRan`). DCGO `BT25_073.cs`: `SelectPermanentEffect`
    /// (`!HasNoLinkCards`) → `SelectCardEffect Root.LinkedCards` →
    /// `TrashLinkCardsAndProcessAccordingToResult` → `successProcess`.
    TrashLinkCardOfOwnDigimon(TrashLinkCardOfOwnDigimonArgs),
    /// Trash-Option-from-{digivolution|link}-cards ACTIVATION cost (BT25-085
    /// BeelStarmon): pick one of `of`'s Digimon whose digivolution cards OR link
    /// cards carry ≥1 Option, then one such Option, and trash it — the correct
    /// observer fires per zone (`OnDigivolutionCardTrashed` for a digivolution
    /// source, `OnLinkedCardTrashed` for a link card). The tail runs only if a
    /// card was trashed. When no own Digimon has an Option in its
    /// digivolution/link cards the cost is unpayable → the clause's remaining
    /// steps are skipped (`TailAlreadyRan`). DCGO `BT25_085.cs`:
    /// `SelectPermanentEffect(PermanentWithTrashableCard)` → `SelectCardEffect
    /// Mode.Discard` over `permanent.DigivolutionOrLinkCards.Any(IsOption)`.
    /// The union-of-sources sibling of `TrashLinkCardOfOwnDigimon`.
    TrashOptionFromOwnStacks(TrashOptionFromOwnStacksArgs),
    BindPermanentProperty(BindPermanentProperty),
    Hatch(PlayerArg),
    /// Move the specified player's eligible breeding-area Digimon to the
    /// battle area through the effect-initiated engine path. Pair with
    /// `select_own_breeding_permanent optional: true` when printed text says
    /// "you may move..." so the accept/decline choice stays visible.
    MoveFromBreeding(PlayerArg),

    // Play / digivolve
    PlayFromHand(PlayFromHandArgs),
    PlayFromHandFree(PlayFromHandFreeArgs),
    UseOptionFromHand(UseOptionFromHandArgs),
    /// Effect-driven Option USE from the controller's trash with a cost delta
    /// (Gap 2, `G-DSL-USE-OPTION-FROM-SOURCES`). Trash analogue of
    /// `use_option_from_hand`.
    UseOptionFromTrash(UseOptionFromTrashArgs),
    /// Use an Option previously picked from the game-level reveal pool by a
    /// `select_reveal` step, applying `cost` to its printed use cost (omitted =
    /// free), preserving the full Option lifecycle. `card` names the reveal-pool
    /// binding. Driver EX7-048 "reveal top 6, use 1 [Three Musketeers] Option
    /// among them free"; the remaining revealed cards are handled by the
    /// enclosing reveal step. `G-DSL-USE-OPTION-FROM-SOURCES`.
    UseOptionFromRevealed(UseOptionFromRevealedArgs),
    /// Use an Option from a permanent's digivolution stack (a
    /// `select_own_sources` binding), applying `cost` to its printed use cost
    /// (omitted = free). `card` names the source-refs binding. Driver BT25-085
    /// "use 1 [Three Musketeers]/[TS] Option from your hand or this Digimon's
    /// digivolution cards free" (the digivolution-cards origin — the engine
    /// `OptionSource::Source` fork resolves both `card_sources` and
    /// `linked_cards`). `G-DSL-USE-OPTION-FROM-SOURCES`.
    UseOptionFromSources(UseOptionFromSourcesArgs),
    /// Use a `select_union_zone{hand,trash}`-bound Option from its true origin
    /// zone, applying `cost` to its printed use cost (omitted = free). Driver
    /// BT21-062 "use 1 [Ragnarok Cannon] from your hand or trash without paying
    /// the cost". `G-DSL-USE-OPTION-FROM-SOURCES`.
    UseOptionBound(UseOptionBoundArgs),
    /// Unified "play OR use 1 card from hand" — inspects the bound hand card's
    /// kind and routes Digimon/Tamer → play and Option → use (a DUAL card
    /// surfaces a "Play as Digimon / Use as Option" face choice). The card is
    /// bound by an upstream `select_hand` step. `G-PLAY-OR-USE-FROM-HAND`.
    PlayOrUseFromHand(PlayOrUseFromHandArgs),
    /// Unified source-origin play/use of a selected digivolution source.
    PlayOrUseFromSources(PlayOrUseFromSourcesArgs),
    PlayFromRevealedFree(PlayFromRevealedFreeArgs),
    PlayFromTrash(PlayFromHandArgs),
    PlayFromTrashFree(PlayFromHandArgs),
    /// PUPPETS-G014 — play a `select_union_zone`-bound card for free from its
    /// true origin zone (hand, trash, or material), recovered from the binding.
    PlayUnionBoundFree(PlayUnionBoundFreeArgs),
    /// Trash a `select_union_zone`-bound card from its true origin zone. Used
    /// for costs that can be paid by trashing a card from hand or from one of
    /// your Digimon's digivolution cards.
    TrashUnionBound(UnionBoundArgs),
    PlayFromSecurity(PlayFromSecurityArgs),
    PlayFromMaterials(PlayFromMaterialsArgs),
    PlaySelectedSourcesFree(TrashSelectedSourcesArgs),
    PlayUnderTamerSource(PlayUnderTamerSourceArgs),
    EffectInitiatedDigivolve(EffectDigivolveArgs),
    EffectInitiatedDnaDigivolve(EffectDnaDigivolveArgs),
    EffectInitiatedDnaDigivolveHandPartner(EffectDnaDigivolveHandPartnerArgs),
    /// DNA digivolve where one material is a battle-area permanent (`target`)
    /// and the other is a card in the controller's TRASH (`trash_partner`);
    /// the merged permanent is topped with `from_hand` (the result card, from
    /// hand). BT18-015 / BT18-073 `[On Deletion]` shape. Lowers to
    /// `EffectContext::effect_initiated_dna_digivolve_trash_partner`
    /// (G-ENGINE-DNA-TRASH-MATERIAL). G-DSL-DNA-TRASH-PARTNER.
    EffectInitiatedDnaDigivolveTrashPartner(EffectDnaDigivolveTrashPartnerArgs),

    // Security
    TrashTopSecurity(TrashTopSecurityArgs),
    TrashBottomSecurity(PlayerArg),
    AddBottomSecurityToHand(PlayerArg),
    TrashTopSecurityAndCancelReplacement(PlayerArg),
    BounceSelf(EmptyArgs),
    /// Relocate THIS effect's source Option (an in-battle-area field Option)
    /// face-down under a chosen own permanent — a new Option-lifecycle exit
    /// distinct from trashing. Fires neither `OnOptionTrashed` nor
    /// `OnDigivolutionCardTrashed`. Drives ST23-15 e-Pulse / ST24-15 DNA Charge
    /// "By placing this card from the battle area face down under any of your
    /// [BEATBREAK]/[DATA SQUAD] trait Tamers, …". G-MOVE-SELF-OPTION-UNDER-PERMANENT.
    MoveSelfOptionUnderPermanent(MoveSelfOptionUnderPermanentArgs),
    /// Place THIS effect's source Option as the bottom digivolution card of a
    /// chosen own permanent — the "[Main] … Then, place this card as the
    /// bottom digivolution card of 1 of your … Digimon" tail on P-180 /
    /// EX7-070 / EX7-071. Unlike `move_self_option_under_permanent` (which
    /// requires a live field-Option `source_permanent`), this composes with
    /// the standard Option [Main]-play path by claiming the in-flight
    /// `pending_option`, so the subsequent `dispose_option` finds nothing and
    /// the card is seated (FACE-UP by default) instead of trashed. Lowers to
    /// `EffectContext::place_self_under_permanent`.
    /// G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL.
    PlaceSelfUnderPermanent(PlaceSelfUnderPermanentArgs),
    SecurityPlaceStackedCard(SecurityPlaceStackedCardArgs),
    SecurityPlaceTopStackedCard(SecurityPlaceTopStackedCardArgs),
    ReturnAllTrashToDeckBottom(PlayerArg),
    ReturnTrashListToDeckBottom(ReturnTrashListToDeckBottomArgs),
    MoveTrashCardToDeckTop(MoveTrashCardToDeckTopArgs),
    TrashTopNDigivolutionCardsOfEach(TrashTopNDigivolutionCardsOfEachArgs),
    TrashOpponentHandToCount(TrashOpponentHandToCountArgs),
    SearchOwnSecurityStack(SearchOwnSecurityStackArgs),
    Recover(DrawArgs),
    MarkSecurityFaceUp(MarkSecurityArgs),
    FlipSecurityFaceUp(PlayerArg),

    // Modifiers
    AddDpModifier(AddDpModifierArgs),
    AddModifier(AddModifierArgs),
    AddPlayerModifier(AddPlayerModifierArgs),
    GrantKeyword(GrantKeywordArgs),
    AllowDigixrosMaterialZone(AllowDigixrosMaterialZoneArgs),
    AddDigixrosCostDelta(DigixrosCostDeltaArgs),
    PreattachDigixrosMaterial(PreattachDigixrosMaterialArgs),
    RegisterDigixrosWildcardForTurn(DigixrosWildcardArgs),
    AddDigixrosWildcardToPendingTransaction(DigixrosWildcardArgs),
    GrantEffectImmunity(GrantEffectImmunityArgs),
    /// PUPPETS-G024 — install the narrow opponent-effect protection
    /// bundle (ImmuneFromDPMinus opponent-scoped + CannotBeDeDigivolved
    /// opponent-scoped). For text like BT16-055's "can't have its DP
    /// reduced by your opponent's effects and isn't affected by
    /// ＜De-Digivolve＞ effects".
    GrantNarrowOpponentEffectProtection(GrantNarrowOpponentEffectProtectionArgs),
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
    SelectMaterials(SelectMaterialsArgs),
    SelectOwnSources(SelectOwnSourcesArgs),
    SelectUnderTamerSources(SelectOwnSourcesArgs),
    SelectOpponentSources(SelectOpponentSourcesArgs),
    DigiBurst(DigiBurstArgs),
    SelectOpponentDpBudget(SelectOpponentDpBudgetArgs),
    SelectOpponentPlayCostBudget(SelectOpponentPlayCostBudgetArgs),
    SelectOwnBreedingPermanent(SelectOwnBreedingPermanentArgs),
    SelectReveal(SelectZoneArgs),
    SelectRevealBuckets(SelectRevealBucketsArgs),
    SelectSecurity(SelectZoneArgs),
    SelectUnionZone(SelectUnionArgs),
    SelectOrderedPermutation(SelectPermutationArgs),
    SelectCountCappedMulti(SelectCountCappedArgs),
    SelectEffectChoice(SelectEffectChoiceArgs),
    RepeatEffectChoice(RepeatEffectChoiceArgs),
    AsSelectingPlayer(AsSelectingPlayerArgs),

    // Control flow
    If(IfStep),
    ForEach(ForEachStep),
    PerSelected(PerSelectedStep),
    ScheduleDelayed(ScheduleDelayedStep),
    /// PUPPETS-G003 — schedule the bound permanent for deletion at turn end.
    ScheduleDeletePlayedAtTurnEnd(ScheduleDeletePlayedAtTurnEndArgs),
    PlaceSelfAsDelayOption(EmptyArgs),
    LinkToOwnDigimon(LinkToOwnDigimonArgs),
    /// `app_fuse:` — effect-initiated App Fuse (see [`AppFuseArgs`]).
    AppFuse(AppFuseArgs),
    /// Gap 5 — reduce the cost of the link about to resolve in the active
    /// `WhenWouldLink` window by `amount`. Authoring verb over the engine's
    /// `reduce_pending_link_cost` primitive; the body of a host-side
    /// `when: when_would_link_to_this` reducer clause (BT25-004 / BT25-045).
    ReduceLinkCost(ReduceLinkCostArgs),
    /// Gap 2 — link 1..N chosen cards from a set of source zones onto a
    /// Digimon host, without paying a link cost. Drives BT25-060 Rebootmon /
    /// BT25-075 Vulcanusmon / BT25-089 Kazuki & Itsuki. The authoring verb over
    /// the engine's `link_chosen_card_into_host` primitive: per pick it presents
    /// a zone-choice prompt (when ≥2 source zones have candidates — DCGO ST22_12
    /// parity), a single-zone card select, then (for `to: own_digimon`) a host
    /// select, then attaches the card and fires `OnLink`.
    LinkCards(LinkCardsArgs),
    /// `relink_self_to_own_digimon:` — move the effect's own standing permanent
    /// to become a link card on a chosen OTHER own Digimon (EX11-027 "link this
    /// Digimon to 1 of your other Digimon"). Over `absorb_standing_digimon_as_link`.
    RelinkSelfToOwnDigimon(RelinkSelfToOwnDigimonArgs),
    Optional(OptionalStep),

    // Combat / replacement process outcomes
    Battle(BattleArgs),
    MayAttackNow(MayAttackNowArgs),
    ForceAttack(ForceAttackArgs),
    RedirectAttackTarget(RedirectAttackTargetArgs),
    CancelAttack(EmptyArgs),
    OpenCounterWindow(EmptyArgs),
    /// Refund this clause's once-per-turn use (DCGO `ActivateClass.RemoveUse()`
    /// — "if nothing executed, the per-turn use is not consumed"). Place it
    /// under a final `if:` whose condition detects the nothing-executed case
    /// (typically `binding_absent` over every pick the body could make).
    /// Only meaningful inside a `once_per_turn`/`max_per_turn` triggered
    /// clause; a no-op elsewhere. G-OPT-REFUND-ON-DECLINE.
    RefundOpt(EmptyArgs),
    RefireEffect(RefireEffectArgs),
    /// Activate a timing-filtered effect printed on a CARD OBJECT (not a
    /// battle-area permanent's top card) — the foreign-card refire variant
    /// (BT15-102 Apocalymon: "activate 1 [On Play] effect on that card as an
    /// effect of this Digimon"). The `card` binding is a `CardHandle`
    /// (typically bound by `place_as_bottom_source.bind_placed_as`), and the
    /// card must currently be a digivolution source of this effect's carrier
    /// permanent. The chosen effect runs with the CARRIER as "this Digimon"
    /// (DCGO `EffectList_ForCard(timing, card)` — the foreign card's effect
    /// bodies are constructed against the carrier's card), one EffectChoice
    /// surfaces when the card has >1 eligible effect, and the pick is
    /// mandatory (DCGO `canNoSelect: () => false`).
    RefireCardEffect(RefireCardEffectArgs),
    EndAttack(bool),
    CancelReplacement(EmptyArgs),
    HandleReplacement(EmptyArgs),
    RedirectReplacement(RedirectReplacementArgs),
    SubstituteReplacement(SubstituteReplacementArgs),

    /// G-COST-REDUCE-ALLY-DIGIVOLVE — install a player-scoped one-shot
    /// future-digivolve cost reducer. Used by BT3-103 Hidden Potential
    /// Discovered!'s `[Main]` clause: "For the turn, when one of your green
    /// Digimon would next digivolve, by suspending 1 of your Digimon,
    /// reduce the digivolution cost by 5." The reducer fires at the next
    /// qualifying digivolution; if `suspend_cost` is set the player is
    /// prompted to suspend 1 of their own Digimon (a player-visible cost).
    ArmDigivolveCostReducer(ArmDigivolveCostReducerArgs),

    /// G-DSL-EOT-DNA-INLINE — surface the printed `[End of Your Turn] This
    /// Digimon and any of your other Digimon may DNA digivolve into a Digimon
    /// card in the hand` flow AS A PLAYER CHOICE AT TRIGGER FIRE, rather than
    /// registering an alt-path action for a later turn. Used by BT12-021,
    /// BT12-047, BT17-007, BT17-019, BT22-008, BT22-017. See the
    /// `CompiledStep::MayDnaDigivolveNow` docstring for the full contract.
    MayDnaDigivolveNow(MayDnaDigivolveNowArgs),

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
            // Draw / deck / hand / trash
            StepSpec::Draw(v) => kv!(s, "draw", v),
            StepSpec::TrashFromTop(v) => kv!(s, "trash_from_top", v),
            StepSpec::AddToHandFromDeck(v) => kv!(s, "add_to_hand_from_deck", v),
            StepSpec::AddToHandFromTrash(v) => kv!(s, "add_to_hand_from_trash", v),
            StepSpec::AddToHandFromSecurity(v) => kv!(s, "add_to_hand_from_security", v),
            StepSpec::PlaySecurityCard(v) => kv!(s, "play_security_card", v),
            StepSpec::TrashSelectedSecurity(v) => kv!(s, "trash_selected_security", v),
            StepSpec::ReturnSelectedSecurityToDeck(v) => {
                kv!(s, "return_selected_security_to_deck", v)
            }
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
            StepSpec::RevealSearch(v) => kv!(s, "reveal_search", v),
            StepSpec::ChooseFromReveal(v) => kv!(s, "choose_from_reveal", v),
            StepSpec::OrderRemainder(v) => kv!(s, "order_remainder", v),
            // Field / permanent
            StepSpec::DeletePermanent(v) => kv!(s, "delete_permanent", v),
            StepSpec::DeleteForCostReduction(v) => kv!(s, "delete_for_cost_reduction", v),
            StepSpec::DeleteBoundPermanents(v) => kv!(s, "delete_bound_permanents", v),
            StepSpec::DeleteOnePerOpponentColor(v) => {
                kv!(s, "delete_one_per_opponent_color", v)
            }
            StepSpec::TrashBreedingPermanent(v) => kv!(s, "trash_breeding_permanent", v),
            StepSpec::ReturnToHand(v) => kv!(s, "return_to_hand", v),
            StepSpec::ReturnToDeck(v) => kv!(s, "return_to_deck", v),
            StepSpec::Suspend(v) => kv!(s, "suspend", v),
            StepSpec::Unsuspend(v) => kv!(s, "unsuspend", v),
            StepSpec::DeDigivolve(v) => kv!(s, "de_digivolve", v),
            StepSpec::PlaceOnSecurity(v) => kv!(s, "place_on_security", v),
            StepSpec::PlayToken(v) => kv!(s, "play_token", v),
            StepSpec::PlaceAsBottomSource(v) => kv!(s, "place_as_bottom_source", v),
            StepSpec::PlaceAsTopSource(v) => kv!(s, "place_as_top_source", v),
            StepSpec::PlaceTopSourceAsBottom(v) => kv!(s, "place_top_source_as_bottom", v),
            StepSpec::TrashTopSource(v) => kv!(s, "trash_top_source", v),
            StepSpec::TrashBottomSources(v) => kv!(s, "trash_bottom_sources", v),
            StepSpec::TrashAllSources(v) => kv!(s, "trash_all_sources", v),
            StepSpec::TrashSelectedSources(v) => kv!(s, "trash_selected_sources", v),
            StepSpec::PlaceSelectedCardUnderTamer(v) => {
                kv!(s, "place_selected_card_under_tamer", v)
            }
            StepSpec::PlaceSelectedSourcesUnderTamer(v) => {
                kv!(s, "place_selected_sources_under_tamer", v)
            }
            StepSpec::MoveMatchingSourcesUnderTamer(v) => {
                kv!(s, "move_matching_sources_under_tamer", v)
            }
            StepSpec::TrashTopStackedSources(v) => kv!(s, "trash_top_stacked_sources", v),
            StepSpec::ReturnSelectedSourcesToHand(v) => {
                kv!(s, "return_selected_sources_to_hand", v)
            }
            StepSpec::ReturnSelectedSourcesToDeck(v) => {
                kv!(s, "return_selected_sources_to_deck", v)
            }
            StepSpec::TrashBottomFaceDownSourceUnderTamer(v) => {
                kv!(s, "trash_bottom_face_down_source_under_tamer", v)
            }
            StepSpec::TrashBottomFaceDownSourcesUnderTamers(v) => {
                kv!(s, "trash_bottom_face_down_sources_under_tamers", v)
            }
            StepSpec::TrashLinkCardOfOwnDigimon(v) => {
                kv!(s, "trash_link_card_of_own_digimon", v)
            }
            StepSpec::TrashOptionFromOwnStacks(v) => {
                kv!(s, "trash_option_from_own_stacks", v)
            }
            StepSpec::BindPermanentProperty(v) => kv!(s, "bind_permanent_property", v),
            StepSpec::Hatch(v) => kv!(s, "hatch", v),
            StepSpec::MoveFromBreeding(v) => kv!(s, "move_from_breeding", v),
            // Play / digivolve
            StepSpec::PlayFromHand(v) => kv!(s, "play_from_hand", v),
            StepSpec::PlayFromHandFree(v) => kv!(s, "play_from_hand_free", v),
            StepSpec::UseOptionFromHand(v) => kv!(s, "use_option_from_hand", v),
            StepSpec::UseOptionFromTrash(v) => kv!(s, "use_option_from_trash", v),
            StepSpec::UseOptionFromRevealed(v) => kv!(s, "use_option_from_revealed", v),
            StepSpec::UseOptionFromSources(v) => kv!(s, "use_option_from_sources", v),
            StepSpec::UseOptionBound(v) => kv!(s, "use_option_bound", v),
            StepSpec::PlayOrUseFromHand(v) => kv!(s, "play_or_use_from_hand", v),
            StepSpec::PlayOrUseFromSources(v) => kv!(s, "play_or_use_from_sources", v),
            StepSpec::PlayFromRevealedFree(v) => kv!(s, "play_from_revealed_free", v),
            StepSpec::PlayFromTrash(v) => kv!(s, "play_from_trash", v),
            StepSpec::PlayFromTrashFree(v) => kv!(s, "play_from_trash_free", v),
            StepSpec::PlayUnionBoundFree(v) => kv!(s, "play_union_bound_free", v),
            StepSpec::TrashUnionBound(v) => kv!(s, "trash_union_bound", v),
            StepSpec::PlayFromSecurity(v) => kv!(s, "play_from_security", v),
            StepSpec::PlayFromMaterials(v) => kv!(s, "play_from_materials", v),
            StepSpec::PlaySelectedSourcesFree(v) => kv!(s, "play_selected_sources_free", v),
            StepSpec::PlayUnderTamerSource(v) => kv!(s, "play_under_tamer_source", v),
            StepSpec::EffectInitiatedDigivolve(v) => kv!(s, "effect_initiated_digivolve", v),
            StepSpec::EffectInitiatedDnaDigivolve(v) => kv!(s, "effect_initiated_dna_digivolve", v),
            StepSpec::EffectInitiatedDnaDigivolveHandPartner(v) => {
                kv!(s, "effect_initiated_dna_digivolve_hand_partner", v)
            }
            StepSpec::EffectInitiatedDnaDigivolveTrashPartner(v) => {
                kv!(s, "effect_initiated_dna_digivolve_trash_partner", v)
            }
            // Security
            StepSpec::TrashTopSecurity(v) => kv!(s, "trash_top_security", v),
            StepSpec::TrashBottomSecurity(v) => kv!(s, "trash_bottom_security", v),
            StepSpec::AddBottomSecurityToHand(v) => kv!(s, "add_bottom_security_to_hand", v),
            StepSpec::TrashTopSecurityAndCancelReplacement(v) => {
                kv!(s, "trash_top_security_and_cancel_replacement", v)
            }
            StepSpec::BounceSelf(v) => kv!(s, "bounce_self", v),
            StepSpec::MoveSelfOptionUnderPermanent(v) => {
                kv!(s, "move_self_option_under_permanent", v)
            }
            StepSpec::PlaceSelfUnderPermanent(v) => {
                kv!(s, "place_self_under_permanent", v)
            }
            StepSpec::SecurityPlaceStackedCard(v) => kv!(s, "security_place_stacked_card", v),
            StepSpec::SecurityPlaceTopStackedCard(v) => {
                kv!(s, "security_place_top_stacked_card", v)
            }
            StepSpec::ReturnAllTrashToDeckBottom(v) => {
                kv!(s, "return_all_trash_to_deck_bottom", v)
            }
            StepSpec::ReturnTrashListToDeckBottom(v) => {
                kv!(s, "return_trash_list_to_deck_bottom", v)
            }
            StepSpec::MoveTrashCardToDeckTop(v) => {
                kv!(s, "move_trash_card_to_deck_top", v)
            }
            StepSpec::TrashTopNDigivolutionCardsOfEach(v) => {
                kv!(s, "trash_top_n_digivolution_cards_of_each", v)
            }
            StepSpec::TrashOpponentHandToCount(v) => kv!(s, "trash_opponent_hand_to_count", v),
            StepSpec::SearchOwnSecurityStack(v) => kv!(s, "search_own_security_stack", v),
            StepSpec::Recover(v) => kv!(s, "recover", v),
            StepSpec::MarkSecurityFaceUp(v) => kv!(s, "mark_security_face_up", v),
            StepSpec::FlipSecurityFaceUp(v) => kv!(s, "flip_security_face_up", v),
            // Modifiers
            StepSpec::AddDpModifier(v) => kv!(s, "add_dp_modifier", v),
            StepSpec::AddModifier(v) => kv!(s, "add_modifier", v),
            StepSpec::AddPlayerModifier(v) => kv!(s, "add_player_modifier", v),
            StepSpec::GrantKeyword(v) => kv!(s, "grant_keyword", v),
            StepSpec::AllowDigixrosMaterialZone(v) => {
                kv!(s, "allow_digixros_material_zone", v)
            }
            StepSpec::AddDigixrosCostDelta(v) => kv!(s, "add_digixros_cost_delta", v),
            StepSpec::PreattachDigixrosMaterial(v) => {
                kv!(s, "preattach_digixros_material", v)
            }
            StepSpec::RegisterDigixrosWildcardForTurn(v) => {
                kv!(s, "register_digixros_wildcard_for_turn", v)
            }
            StepSpec::AddDigixrosWildcardToPendingTransaction(v) => {
                kv!(s, "add_digixros_wildcard_to_pending_transaction", v)
            }
            StepSpec::GrantTriggeredEffect(v) => kv!(s, "grant_triggered_effect", v),
            StepSpec::GrantEffectImmunity(v) => kv!(s, "grant_effect_immunity", v),
            StepSpec::GrantNarrowOpponentEffectProtection(v) => {
                kv!(s, "grant_narrow_opponent_effect_protection", v)
            }
            // Selection
            StepSpec::SelectOwnPermanent(v) => kv!(s, "select_own_permanent", v),
            StepSpec::SelectOpponentPermanent(v) => kv!(s, "select_opponent_permanent", v),
            StepSpec::SelectAnyPermanent(v) => kv!(s, "select_any_permanent", v),
            StepSpec::SelectDnaPair(v) => kv!(s, "select_dna_pair", v),
            StepSpec::SelectHand(v) => kv!(s, "select_hand", v),
            StepSpec::SelectTrash(v) => kv!(s, "select_trash", v),
            StepSpec::SelectMaterial(v) => kv!(s, "select_material", v),
            StepSpec::SelectMaterials(v) => kv!(s, "select_materials", v),
            StepSpec::SelectOwnSources(v) => kv!(s, "select_own_sources", v),
            StepSpec::SelectUnderTamerSources(v) => kv!(s, "select_under_tamer_sources", v),
            StepSpec::SelectOpponentSources(v) => kv!(s, "select_opponent_sources", v),
            StepSpec::DigiBurst(v) => kv!(s, "digi_burst", v),
            StepSpec::SelectOpponentDpBudget(v) => kv!(s, "select_opponent_dp_budget", v),
            StepSpec::SelectOpponentPlayCostBudget(v) => {
                kv!(s, "select_opponent_play_cost_budget", v)
            }
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
            StepSpec::RepeatEffectChoice(v) => kv!(s, "repeat_effect_choice", v),
            StepSpec::AsSelectingPlayer(v) => kv!(s, "as_selecting_player", v),
            // Control flow
            StepSpec::If(v) => kv!(s, "if", v),
            StepSpec::ForEach(v) => kv!(s, "for_each", v),
            StepSpec::PerSelected(v) => kv!(s, "per_selected", v),
            StepSpec::ScheduleDelayed(v) => kv!(s, "schedule_delayed", v),
            StepSpec::ScheduleDeletePlayedAtTurnEnd(v) => {
                kv!(s, "schedule_delete_played_at_turn_end", v)
            }
            StepSpec::PlaceSelfAsDelayOption(v) => kv!(s, "place_self_as_delay_option", v),
            StepSpec::LinkToOwnDigimon(v) => kv!(s, "link_to_own_digimon", v),
            StepSpec::AppFuse(v) => kv!(s, "app_fuse", v),
            StepSpec::ReduceLinkCost(v) => kv!(s, "reduce_link_cost", v),
            StepSpec::LinkCards(v) => kv!(s, "link_cards", v),
            StepSpec::RelinkSelfToOwnDigimon(v) => kv!(s, "relink_self_to_own_digimon", v),
            StepSpec::Optional(v) => kv!(s, "optional", v),
            // Combat / replacement process outcomes
            StepSpec::Battle(v) => kv!(s, "battle", v),
            StepSpec::MayAttackNow(v) => kv!(s, "may_attack_now", v),
            StepSpec::ForceAttack(v) => kv!(s, "force_attack", v),
            StepSpec::RedirectAttackTarget(v) => kv!(s, "redirect_attack_target", v),
            StepSpec::CancelAttack(v) => kv!(s, "cancel_attack", v),
            StepSpec::OpenCounterWindow(v) => kv!(s, "open_counter_window", v),
            StepSpec::RefundOpt(v) => kv!(s, "refund_opt", v),
            StepSpec::RefireEffect(v) => kv!(s, "refire_effect", v),
            StepSpec::RefireCardEffect(v) => kv!(s, "refire_card_effect", v),
            StepSpec::EndAttack(v) => kv!(s, "end_attack", v),
            StepSpec::CancelReplacement(v) => kv!(s, "cancel_replacement", v),
            StepSpec::HandleReplacement(v) => kv!(s, "handle_replacement", v),
            StepSpec::RedirectReplacement(v) => kv!(s, "redirect_replacement", v),
            StepSpec::SubstituteReplacement(v) => kv!(s, "substitute_replacement", v),
            StepSpec::ArmDigivolveCostReducer(v) => kv!(s, "arm_digivolve_cost_reducer", v),
            StepSpec::MayDnaDigivolveNow(v) => kv!(s, "may_dna_digivolve_now", v),
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
            // Deserialize-only back-compat aliases for the retired `_fn` twins.
            // The old YAML wrapped the formula in `{ formula: <FormulaSpec> }`
            // (FormulaStepArgs); the new canonical shape takes the formula
            // directly, so unwrap the legacy wrapper here.
            "gain_memory_fn" => StepSpec::GainMemory(map.next_value::<FormulaStepArgs>()?.formula),
            "lose_memory_fn" => StepSpec::LoseMemory(map.next_value::<FormulaStepArgs>()?.formula),

            // Draw / deck / hand / trash
            "draw" => StepSpec::Draw(map.next_value()?),
            "trash_from_top" => StepSpec::TrashFromTop(map.next_value()?),
            "add_to_hand_from_deck" => StepSpec::AddToHandFromDeck(map.next_value()?),
            "add_to_hand_from_trash" => StepSpec::AddToHandFromTrash(map.next_value()?),
            "add_to_hand_from_security" => StepSpec::AddToHandFromSecurity(map.next_value()?),
            "play_security_card" => StepSpec::PlaySecurityCard(map.next_value()?),
            "trash_selected_security" => StepSpec::TrashSelectedSecurity(map.next_value()?),
            "return_selected_security_to_deck" => {
                StepSpec::ReturnSelectedSecurityToDeck(map.next_value()?)
            }
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
            "reveal_search" => StepSpec::RevealSearch(map.next_value()?),
            "choose_from_reveal" => StepSpec::ChooseFromReveal(map.next_value()?),
            "order_remainder" => StepSpec::OrderRemainder(map.next_value()?),

            // Field / permanent
            "delete_permanent" => StepSpec::DeletePermanent(map.next_value()?),
            "delete_for_cost_reduction" => StepSpec::DeleteForCostReduction(map.next_value()?),
            "delete_bound_permanents" => StepSpec::DeleteBoundPermanents(map.next_value()?),
            "delete_one_per_opponent_color" => {
                StepSpec::DeleteOnePerOpponentColor(map.next_value()?)
            }
            "trash_breeding_permanent" => StepSpec::TrashBreedingPermanent(map.next_value()?),
            "return_to_hand" => StepSpec::ReturnToHand(map.next_value()?),
            "return_to_deck" => StepSpec::ReturnToDeck(map.next_value()?),
            "suspend" => StepSpec::Suspend(map.next_value()?),
            "unsuspend" => StepSpec::Unsuspend(map.next_value()?),
            "de_digivolve" => StepSpec::DeDigivolve(map.next_value()?),
            "place_on_security" => StepSpec::PlaceOnSecurity(map.next_value()?),
            "play_token" => StepSpec::PlayToken(map.next_value()?),
            "place_as_bottom_source" => StepSpec::PlaceAsBottomSource(map.next_value()?),
            "place_as_top_source" => StepSpec::PlaceAsTopSource(map.next_value()?),
            "place_top_source_as_bottom" => StepSpec::PlaceTopSourceAsBottom(map.next_value()?),
            "trash_top_source" => StepSpec::TrashTopSource(map.next_value()?),
            "trash_bottom_sources" => StepSpec::TrashBottomSources(map.next_value()?),
            "trash_all_sources" => StepSpec::TrashAllSources(map.next_value()?),
            "trash_selected_sources" => StepSpec::TrashSelectedSources(map.next_value()?),
            "place_selected_card_under_tamer" => {
                StepSpec::PlaceSelectedCardUnderTamer(map.next_value()?)
            }
            "place_selected_sources_under_tamer" => {
                StepSpec::PlaceSelectedSourcesUnderTamer(map.next_value()?)
            }
            "move_matching_sources_under_tamer" => {
                StepSpec::MoveMatchingSourcesUnderTamer(map.next_value()?)
            }
            "trash_top_stacked_sources" => StepSpec::TrashTopStackedSources(map.next_value()?),
            "return_selected_sources_to_hand" => {
                StepSpec::ReturnSelectedSourcesToHand(map.next_value()?)
            }
            "return_selected_sources_to_deck" => {
                StepSpec::ReturnSelectedSourcesToDeck(map.next_value()?)
            }
            "trash_bottom_face_down_source_under_tamer" => {
                StepSpec::TrashBottomFaceDownSourceUnderTamer(map.next_value()?)
            }
            "trash_bottom_face_down_sources_under_tamers" => {
                StepSpec::TrashBottomFaceDownSourcesUnderTamers(map.next_value()?)
            }
            "trash_link_card_of_own_digimon" => {
                StepSpec::TrashLinkCardOfOwnDigimon(map.next_value()?)
            }
            "trash_option_from_own_stacks" => StepSpec::TrashOptionFromOwnStacks(map.next_value()?),
            "bind_permanent_property" => StepSpec::BindPermanentProperty(map.next_value()?),
            "hatch" => StepSpec::Hatch(map.next_value()?),
            "move_from_breeding" => StepSpec::MoveFromBreeding(map.next_value()?),

            // Play / digivolve
            "play_from_hand" => StepSpec::PlayFromHand(map.next_value()?),
            "play_from_hand_free" => StepSpec::PlayFromHandFree(map.next_value()?),
            "use_option_from_hand" => StepSpec::UseOptionFromHand(map.next_value()?),
            "use_option_from_trash" => StepSpec::UseOptionFromTrash(map.next_value()?),
            "use_option_from_revealed" => StepSpec::UseOptionFromRevealed(map.next_value()?),
            "use_option_from_sources" => StepSpec::UseOptionFromSources(map.next_value()?),
            "use_option_bound" => StepSpec::UseOptionBound(map.next_value()?),
            "play_or_use_from_hand" => StepSpec::PlayOrUseFromHand(map.next_value()?),
            "play_or_use_from_sources" => StepSpec::PlayOrUseFromSources(map.next_value()?),
            "play_from_revealed_free" => StepSpec::PlayFromRevealedFree(map.next_value()?),
            "play_from_trash" => StepSpec::PlayFromTrash(map.next_value()?),
            "play_from_trash_free" => StepSpec::PlayFromTrashFree(map.next_value()?),
            "play_union_bound_free" => StepSpec::PlayUnionBoundFree(map.next_value()?),
            "trash_union_bound" => StepSpec::TrashUnionBound(map.next_value()?),
            "play_from_security" => StepSpec::PlayFromSecurity(map.next_value()?),
            "play_from_materials" => StepSpec::PlayFromMaterials(map.next_value()?),
            "play_selected_sources_free" => StepSpec::PlaySelectedSourcesFree(map.next_value()?),
            "play_under_tamer_source" => StepSpec::PlayUnderTamerSource(map.next_value()?),
            "effect_initiated_digivolve" => StepSpec::EffectInitiatedDigivolve(map.next_value()?),
            "effect_initiated_dna_digivolve" => {
                StepSpec::EffectInitiatedDnaDigivolve(map.next_value()?)
            }
            "effect_initiated_dna_digivolve_hand_partner" => {
                StepSpec::EffectInitiatedDnaDigivolveHandPartner(map.next_value()?)
            }
            "effect_initiated_dna_digivolve_trash_partner" => {
                StepSpec::EffectInitiatedDnaDigivolveTrashPartner(map.next_value()?)
            }

            // Security
            "trash_top_security" => StepSpec::TrashTopSecurity(map.next_value()?),
            "trash_bottom_security" => StepSpec::TrashBottomSecurity(map.next_value()?),
            "add_bottom_security_to_hand" => StepSpec::AddBottomSecurityToHand(map.next_value()?),
            "trash_top_security_and_cancel_replacement" => {
                StepSpec::TrashTopSecurityAndCancelReplacement(map.next_value()?)
            }
            "bounce_self" => StepSpec::BounceSelf(map.next_value()?),
            "move_self_option_under_permanent" => {
                StepSpec::MoveSelfOptionUnderPermanent(map.next_value()?)
            }
            "place_self_under_permanent" => StepSpec::PlaceSelfUnderPermanent(map.next_value()?),
            "security_place_stacked_card" => StepSpec::SecurityPlaceStackedCard(map.next_value()?),
            "security_place_top_stacked_card" => {
                StepSpec::SecurityPlaceTopStackedCard(map.next_value()?)
            }
            "return_all_trash_to_deck_bottom" => {
                StepSpec::ReturnAllTrashToDeckBottom(map.next_value()?)
            }
            "return_trash_list_to_deck_bottom" => {
                StepSpec::ReturnTrashListToDeckBottom(map.next_value()?)
            }
            "move_trash_card_to_deck_top" => StepSpec::MoveTrashCardToDeckTop(map.next_value()?),
            "trash_top_n_digivolution_cards_of_each" => {
                StepSpec::TrashTopNDigivolutionCardsOfEach(map.next_value()?)
            }
            "trash_opponent_hand_to_count" => StepSpec::TrashOpponentHandToCount(map.next_value()?),
            "search_own_security_stack" => StepSpec::SearchOwnSecurityStack(map.next_value()?),
            "recover" => StepSpec::Recover(map.next_value()?),
            "mark_security_face_up" => StepSpec::MarkSecurityFaceUp(map.next_value()?),
            "flip_security_face_up" => StepSpec::FlipSecurityFaceUp(map.next_value()?),

            // Modifiers
            "add_dp_modifier" => StepSpec::AddDpModifier(map.next_value()?),
            "add_modifier" => StepSpec::AddModifier(map.next_value()?),
            "add_player_modifier" => StepSpec::AddPlayerModifier(map.next_value()?),
            "grant_keyword" => StepSpec::GrantKeyword(map.next_value()?),
            "allow_digixros_material_zone" => {
                StepSpec::AllowDigixrosMaterialZone(map.next_value()?)
            }
            "add_digixros_cost_delta" => StepSpec::AddDigixrosCostDelta(map.next_value()?),
            "preattach_digixros_material" => StepSpec::PreattachDigixrosMaterial(map.next_value()?),
            "register_digixros_wildcard_for_turn" => {
                StepSpec::RegisterDigixrosWildcardForTurn(map.next_value()?)
            }
            "add_digixros_wildcard_to_pending_transaction" => {
                StepSpec::AddDigixrosWildcardToPendingTransaction(map.next_value()?)
            }
            "grant_triggered_effect" => StepSpec::GrantTriggeredEffect(map.next_value()?),
            "grant_effect_immunity" => StepSpec::GrantEffectImmunity(map.next_value()?),
            "grant_narrow_opponent_effect_protection" => {
                StepSpec::GrantNarrowOpponentEffectProtection(map.next_value()?)
            }

            // Selection
            "select_own_permanent" => StepSpec::SelectOwnPermanent(map.next_value()?),
            "select_opponent_permanent" => StepSpec::SelectOpponentPermanent(map.next_value()?),
            "select_any_permanent" => StepSpec::SelectAnyPermanent(map.next_value()?),
            "select_dna_pair" => StepSpec::SelectDnaPair(map.next_value()?),
            "select_hand" => StepSpec::SelectHand(map.next_value()?),
            "select_trash" => StepSpec::SelectTrash(map.next_value()?),
            "select_material" => StepSpec::SelectMaterial(map.next_value()?),
            "select_materials" => StepSpec::SelectMaterials(map.next_value()?),
            "select_own_sources" => StepSpec::SelectOwnSources(map.next_value()?),
            "select_under_tamer_sources" => StepSpec::SelectUnderTamerSources(map.next_value()?),
            "select_opponent_sources" => StepSpec::SelectOpponentSources(map.next_value()?),
            "digi_burst" => StepSpec::DigiBurst(map.next_value()?),
            "select_opponent_dp_budget" => StepSpec::SelectOpponentDpBudget(map.next_value()?),
            "select_opponent_play_cost_budget" => {
                StepSpec::SelectOpponentPlayCostBudget(map.next_value()?)
            }
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
            "repeat_effect_choice" => StepSpec::RepeatEffectChoice(map.next_value()?),
            "as_selecting_player" => StepSpec::AsSelectingPlayer(map.next_value()?),

            // Control flow
            "if" => StepSpec::If(map.next_value()?),
            "for_each" => StepSpec::ForEach(map.next_value()?),
            "per_selected" => StepSpec::PerSelected(map.next_value()?),
            "schedule_delayed" => StepSpec::ScheduleDelayed(map.next_value()?),
            "schedule_delete_played_at_turn_end" => {
                StepSpec::ScheduleDeletePlayedAtTurnEnd(map.next_value()?)
            }
            "place_self_as_delay_option" => StepSpec::PlaceSelfAsDelayOption(map.next_value()?),
            "link_to_own_digimon" => StepSpec::LinkToOwnDigimon(map.next_value()?),
            "app_fuse" => StepSpec::AppFuse(map.next_value()?),
            "reduce_link_cost" => StepSpec::ReduceLinkCost(map.next_value()?),
            "link_cards" => StepSpec::LinkCards(map.next_value()?),
            "relink_self_to_own_digimon" => StepSpec::RelinkSelfToOwnDigimon(map.next_value()?),
            "optional" => StepSpec::Optional(map.next_value()?),

            // Combat / replacement process outcomes
            "battle" => StepSpec::Battle(map.next_value()?),
            "may_attack_now" => StepSpec::MayAttackNow(map.next_value()?),
            "force_attack" => StepSpec::ForceAttack(map.next_value()?),
            "redirect_attack_target" => StepSpec::RedirectAttackTarget(map.next_value()?),
            "cancel_attack" => StepSpec::CancelAttack(map.next_value()?),
            "open_counter_window" => StepSpec::OpenCounterWindow(map.next_value()?),
            "refund_opt" => StepSpec::RefundOpt(map.next_value()?),
            "refire_effect" => StepSpec::RefireEffect(map.next_value()?),
            "refire_card_effect" => StepSpec::RefireCardEffect(map.next_value()?),
            "end_attack" => StepSpec::EndAttack(map.next_value()?),
            "cancel_replacement" => StepSpec::CancelReplacement(map.next_value()?),
            "handle_replacement" => StepSpec::HandleReplacement(map.next_value()?),
            "redirect_replacement" => StepSpec::RedirectReplacement(map.next_value()?),
            "substitute_replacement" => StepSpec::SubstituteReplacement(map.next_value()?),

            // G-COST-REDUCE-ALLY-DIGIVOLVE
            "arm_digivolve_cost_reducer" => StepSpec::ArmDigivolveCostReducer(map.next_value()?),

            // G-DSL-EOT-DNA-INLINE
            "may_dna_digivolve_now" => StepSpec::MayDnaDigivolveNow(map.next_value()?),

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
                        "draw",
                        "trash_from_top",
                        "add_to_hand_from_deck",
                        "add_to_hand_from_trash",
                        "add_to_hand_from_security",
                        "play_security_card",
                        "trash_selected_security",
                        "return_selected_security_to_deck",
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
                        "reveal_search",
                        "choose_from_reveal",
                        "order_remainder",
                        "delete_permanent",
                        "delete_for_cost_reduction",
                        "delete_bound_permanents",
                        "delete_one_per_opponent_color",
                        "trash_breeding_permanent",
                        "return_to_hand",
                        "return_to_deck",
                        "suspend",
                        "unsuspend",
                        "de_digivolve",
                        "place_on_security",
                        "play_token",
                        "place_as_bottom_source",
                        "place_as_top_source",
                        "place_top_source_as_bottom",
                        "trash_top_source",
                        "trash_bottom_sources",
                        "trash_all_sources",
                        "trash_selected_sources",
                        "return_selected_sources_to_hand",
                        "trash_bottom_face_down_source_under_tamer",
                        "trash_bottom_face_down_sources_under_tamers",
                        "trash_link_card_of_own_digimon",
                        "trash_option_from_own_stacks",
                        "return_selected_sources_to_deck",
                        "bind_permanent_property",
                        "hatch",
                        "play_from_hand",
                        "play_from_hand_free",
                        "use_option_from_hand",
                        "use_option_from_trash",
                        "play_or_use_from_hand",
                        "play_or_use_from_sources",
                        "play_from_revealed_free",
                        "play_from_trash",
                        "play_from_trash_free",
                        "play_union_bound_free",
                        "play_from_security",
                        "play_from_materials",
                        "play_selected_sources_free",
                        "effect_initiated_digivolve",
                        "effect_initiated_dna_digivolve",
                        "effect_initiated_dna_digivolve_hand_partner",
                        "effect_initiated_dna_digivolve_trash_partner",
                        "app_fuse",
                        "trash_top_security",
                        "trash_bottom_security",
                        "add_bottom_security_to_hand",
                        "trash_top_security_and_cancel_replacement",
                        "bounce_self",
                        "move_self_option_under_permanent",
                        "place_self_under_permanent",
                        "security_place_stacked_card",
                        "security_place_top_stacked_card",
                        "return_all_trash_to_deck_bottom",
                        "return_trash_list_to_deck_bottom",
                        "move_trash_card_to_deck_top",
                        "trash_top_n_digivolution_cards_of_each",
                        "trash_opponent_hand_to_count",
                        "search_own_security_stack",
                        "recover",
                        "mark_security_face_up",
                        "flip_security_face_up",
                        "add_dp_modifier",
                        "add_modifier",
                        "grant_keyword",
                        "grant_effect_immunity",
                        "grant_narrow_opponent_effect_protection",
                        "select_own_permanent",
                        "select_opponent_permanent",
                        "select_any_permanent",
                        "select_dna_pair",
                        "select_hand",
                        "select_trash",
                        "select_material",
                        "select_materials",
                        "select_own_sources",
                        "select_opponent_sources",
                        "digi_burst",
                        "select_opponent_dp_budget",
                        "select_opponent_play_cost_budget",
                        "select_own_breeding_permanent",
                        "select_reveal",
                        "select_reveal_buckets",
                        "select_security",
                        "select_union_zone",
                        "select_ordered_permutation",
                        "select_count_capped_multi",
                        "select_effect_choice",
                        "repeat_effect_choice",
                        "as_selecting_player",
                        "if",
                        "for_each",
                        "per_selected",
                        "schedule_delayed",
                        "place_self_as_delay_option",
                        "link_to_own_digimon",
                        "reduce_link_cost",
                        "link_cards",
                        "relink_self_to_own_digimon",
                        "optional",
                        "battle",
                        "may_attack_now",
                        "force_attack",
                        "redirect_attack_target",
                        "cancel_attack",
                        "open_counter_window",
                        "refire_effect",
                        "refire_card_effect",
                        "end_attack",
                        "cancel_replacement",
                        "handle_replacement",
                        "redirect_replacement",
                        "substitute_replacement",
                        "arm_digivolve_cost_reducer",
                        "may_dna_digivolve_now",
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

/// Args for `delete_one_per_opponent_color`
/// (G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR). Iterates the game's colors;
/// for each color that appears among the opponent's Digimon (matching `filter`,
/// if any), drives a MANDATORY pick of 1 not-yet-chosen opponent Digimon of that
/// color, accumulates the picks, and BATCH-deletes them after every color's pick
/// resolves (DCGO `EX9_074.cs` Branch B: "delete 1 of each of your opponent's
/// Digimon with different colors"). Colors with no legal target are skipped; a
/// picked Digimon of a multi-color card is excluded from later colors' picks
/// (so a two-color Digimon is deleted at most once). Every pick surfaces in the
/// action space (no PASS — mandatory). Driver: EX9-074 Kimeramon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteOnePerOpponentColorArgs {
    /// Extra restriction AND-ed onto "opponent Digimon of the current color".
    /// Defaults to no extra restriction (every opponent Digimon is eligible),
    /// matching EX9-074 Branch B. Authored as a `filter:` predicate block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<PredicateSpec>,
    /// Prompt template. `{color}` is substituted with the current color name at
    /// each per-color pick (DCGO `"Select 1 {deletableColor} Digimon to delete"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Optional localization-key override for `prompt`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
}

/// Args for `reduce_link_cost` — reduce the cost of the link about to resolve
/// in the active `WhenWouldLink` window by `amount` (saturating at 0). Only
/// meaningful inside a `when: when_would_link_to_this` clause's `process`
/// (Gap 5 — BT25-004 / BT25-045: "you may reduce the cost by 1").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReduceLinkCostArgs {
    pub amount: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashBreedingPermanentArgs {
    pub target: BindingRef,
}

/// Move a selected list of trash cards to the deck. `cards` is the name of a
/// card-list binding (e.g. produced by `select_count_capped_multi`). Unlike
/// `return_all_trash_to_deck_bottom` this moves only the bound cards. The
/// `destination` field selects the deck top or bottom; omitting it defaults
/// to bottom — the historical behavior of this verb.
/// G-ZONE-TRASH-TO-DECK / G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnTrashListToDeckBottomArgs {
    pub of: PlayerRef,
    pub cards: BindingRef,
    /// Deck end the cards are returned to. Omitted → `Bottom` (legacy
    /// behavior, so trash-return steps authored before `destination` existed
    /// keep compiling identically).
    #[serde(default, skip_serializing_if = "DeckDestination::is_bottom")]
    pub destination: DeckDestination,
}

/// Destination end of a deck for a trash-return step: the **top** (the next
/// card drawn) or the **bottom**.
/// G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DeckDestination {
    Top,
    #[default]
    Bottom,
}

impl DeckDestination {
    /// True for the default (`Bottom`). Used by `skip_serializing_if` so a
    /// trash-return step that omits `destination` round-trips unchanged.
    pub fn is_bottom(&self) -> bool {
        matches!(self, Self::Bottom)
    }
}

/// Move a single selected trash card to the TOP of the deck. `card` names a
/// single-card binding produced by a prior `select_trash` step (a `TrashIndex`
/// or `Card` binding). `of` identifies whose trash the card is currently in;
/// the card always returns to its OWNER's deck. Selected-trash analog of
/// `return_trash_list_to_deck_bottom`, but single-card and deck-TOP.
/// G-ZONE-SELECTED-TRASH-TO-DECK-TOP — driver LM-030 clause B.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveTrashCardToDeckTopArgs {
    pub of: PlayerRef,
    pub card: BindingRef,
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
    /// Top card of a player's deck — a card-source binding (not a permanent).
    /// Used by card-source steps such as `place_as_bottom_source` to stash
    /// the deck top under a Tamer. YAML: `{ deck_top: you }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_top: Option<PlayerRef>,
    /// The controller's own breeding-area permanent, resolved directly to the
    /// breeding carrier handle (`index = BREEDING_TARGET`) with NO selection
    /// prompt — the breeding area holds at most one Digimon. Used by
    /// `select_union_zone`'s `material_of` to scan breeding-area digivolution
    /// sources without a degenerate carrier pick. YAML: `{ own_breeding: true }`.
    /// G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_breeding: Option<bool>,
}

// ── Argument structs (one per verb family) ──────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayerArg {
    pub of: PlayerRef,
}

/// Args for `trash_top_security`. The optional `count` field trashes N cards
/// from the top of the security stack, where N is a formula evaluated at run
/// time; omitting it trashes exactly one (the historical behavior).
/// Drives BT17-018's "for every 10 cards in both players' trash, trash 1
/// security" — `count: { floor_div: [ <card_count_in_zone trash any>, 10 ] }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashTopSecurityArgs {
    pub of: PlayerRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<crate::formula::FormulaSpec>,
    /// "Trash the top cards … so that it has N cards left" — trashes from the
    /// top of `of`'s security stack until exactly `leave` cards remain (i.e.
    /// `max(0, security_count - leave)` cards, clamped so it never underflows /
    /// over-trashes an already-short stack). Mutually exclusive with `count`
    /// (specify one or the other — a spec carrying both is rejected by the
    /// linter). Mask-accurate: the trash amount is computed at run time from
    /// the actual stack size, so no illegal action is ever offered. Drives
    /// BT21-098 Ragnarok Cannon's [Delay] "trash the top cards of your
    /// opponent's security stack so that it has 1 card left" (`leave: 1`). DCGO
    /// `IDestroySecurity(destroySecurityCount: Enemy.SecurityCards.Count - 1,
    /// fromTop: true)`. G-DSL-TRASH-TOP-SECURITY-LEAVE (driver BT21-098).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leave: Option<crate::formula::FormulaSpec>,
}

/// Args for `trash_bottom_face_down_source_under_tamer` — bundles "pick one of
/// `of`'s Tamers that carries a face-down stash → trash its bottom face-down
/// source". Used as an activation cost by BEATBREAK / DATA SQUAD cards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashBottomFaceDownSourceUnderTamerArgs {
    pub of: PlayerRef,
    /// When `true`, the Tamer pick is DECLINABLE (offers PASS) — modelling
    /// DCGO's `canNoSelect:true` Tamer selection for a "by trashing the bottom
    /// face-down card …, [effect]" that is one of several INDEPENDENT optional
    /// sub-choices inside a non-optional clause (so a clause-level `optional`
    /// can't carry its decline). On decline NOTHING is trashed and the step's
    /// tail does not run. Default `false` keeps the established mandatory-when-
    /// payable behavior (the decline is carried by a clause-level `optional`).
    /// ST24-11 Rosemon: "you may suspend up to 2 … then, by trashing … none of
    /// their Digimon can unsuspend" — the trash+lock is declinable apart from
    /// the suspend. `G-OPTIONAL-TRASH-FACE-DOWN-UNDER-TAMER`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

/// Args for `trash_bottom_face_down_sources_under_tamers` — the multi-count /
/// multi-Tamer sibling. Trashes `count` bottom-face-down digivolution sources
/// total, distributed across `of`'s Tamers (each Tamer must carry >=1 face-down
/// source). A single Tamer with >=`count` face-down sources may satisfy it
/// alone; otherwise the cost spreads across multiple Tamers. Each Tamer pick is
/// surfaced as a real `PendingSelection`. Used as an activation cost by
/// BT25-035 Cougarmon (`count: 2`). `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashBottomFaceDownSourcesUnderTamersArgs {
    pub of: PlayerRef,
    /// Total number of bottom-face-down sources to trash across Tamers.
    pub count: u8,
}

/// Args for `trash_link_card_of_own_digimon` — the link-card-trash ACTIVATION
/// cost (BT25-073 Dragomon). Picks one of `of`'s Digimon with ≥1 link card, then
/// one of its link cards, and trashes it; the step's tail runs only if the trash
/// happened. `G-DSL-LINK-TRASH-AS-COST`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashLinkCardOfOwnDigimonArgs {
    pub of: PlayerRef,
    /// When `true`, BOTH picks (own Digimon and its link card) are DECLINABLE
    /// (offer PASS) — modelling DCGO's `canNoSelect: true` on both selects for a
    /// "By trashing 1 of your Digimon's link cards, you MAY …" clause. On decline
    /// nothing is trashed and the step's tail does not run. Default `false`
    /// leaves the decline to a clause-level `optional`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

/// Args for `trash_option_from_own_stacks` — the trash-Option-from-{digivolution|
/// link}-cards ACTIVATION cost (BT25-085 BeelStarmon). Picks one of `of`'s
/// Digimon whose digivolution cards OR link cards carry ≥1 Option, then one such
/// Option, and trashes it (the correct observer fires per zone); the step's tail
/// runs only if the trash happened. `G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashOptionFromOwnStacksArgs {
    pub of: PlayerRef,
    /// When `true`, BOTH picks (own Digimon and its Option) are DECLINABLE
    /// (offer PASS) — DCGO's `canNoSelect: true` on both selects for a "By
    /// trashing 1 Option …, this Digimon MAY …" clause. On decline nothing is
    /// trashed and the step's tail does not run. Default `false` leaves the
    /// decline to a clause-level `optional`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

/// Args for `move_self_option_under_permanent` — relocate THIS effect's source
/// Option (an in-battle-area field Option) face-down under a chosen own
/// permanent. `target` names a binding (typically a prior `select_own_permanent`
/// Tamer pick); `face_down` marks the placed source face-down (default `true`,
/// matching the BEATBREAK / DATA SQUAD "face down under" wording).
/// G-MOVE-SELF-OPTION-UNDER-PERMANENT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveSelfOptionUnderPermanentArgs {
    pub target: BindingRef,
    #[serde(default = "default_true")]
    pub face_down: bool,
}

fn default_true() -> bool {
    true
}

/// Args for `place_self_under_permanent` — place THIS effect's source Option
/// as the bottom digivolution card of a chosen own permanent, composing with
/// the standard Option [Main]-play path (the in-flight `pending_option` is
/// claimed, so the Option is seated instead of trashed). `target` names a
/// binding (typically a prior `select_own_permanent` pick); `face_down`
/// defaults to `false` — the P-180 / EX7-070 / EX7-071 tail places the card
/// FACE-UP (DCGO `AddDigivolutionCardsBottom` without `isFacedown: true`).
/// G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceSelfUnderPermanentArgs {
    pub target: BindingRef,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub face_down: bool,
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
    /// "by trashing this card ..." — pays the cost by trashing the source
    /// permanent (the `<Delay>` Option). Declinable, per Comprehensive Rules
    /// 16-16-2. G-ACTIVATION-COST-TRASH-SELF.
    #[serde(default, skip_serializing_if = "is_false")]
    pub trash_self: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Args for the `arm_digivolve_cost_reducer:` DSL step
/// (`G-COST-REDUCE-ALLY-DIGIVOLVE`).
///
/// YAML shape (BT3-103 Hidden Potential Discovered!):
/// ```yaml
/// - arm_digivolve_cost_reducer:
///     amount: 5
///     single_fire: true
///     target_color: green
///     suspend_cost: true
/// ```
///
/// Installs a player-scoped, turn-scoped ("For the turn") cost reducer.
/// At the next qualifying digivolution (the digivolving permanent's top
/// card includes `target_color`, when set), the player is offered an
/// accept/decline prompt; on accept, `suspend_cost` prompts the player to
/// suspend 1 of their own Digimon. `single_fire` consumes the reducer on
/// the first successful application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArmDigivolveCostReducerArgs {
    /// Memory by which the digivolution cost is reduced.
    pub amount: i32,
    /// When `true`, the reducer fires exactly once ("would next digivolve")
    /// then removes itself.
    #[serde(default, skip_serializing_if = "is_false")]
    pub single_fire: bool,
    /// When set, the digivolving permanent's top card must include this
    /// color for the reducer to fire. Omit for "any color".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_color: Option<crate::spec::ColorSpec>,
    /// When `true`, applying the reduction prompts the player to suspend 1
    /// of their own Digimon (an interactive, player-visible cost).
    #[serde(default, skip_serializing_if = "is_false")]
    pub suspend_cost: bool,
}

fn is_zero_u8(n: &u8) -> bool {
    *n == 0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetArg {
    pub target: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashBottomSourcesArgs {
    pub target: BindingRef,
    pub count: u8,
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
    pub ignore_summoning_sickness: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub windowed: bool,
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

/// Args for the `may_dna_digivolve_now` step. See
/// `CompiledStep::MayDnaDigivolveNow` for the full contract.
///
/// `anchor` defaults to `source` (the trigger's source permanent) when
/// omitted, mirroring the printed "This Digimon" half of the merge.
/// `partner_filter` constrains the other DNA material on own field — the
/// engine excludes the anchor as a hard invariant of the verb, so YAML
/// need not repeat the exclusion. `target_filter` constrains the Digimon
/// card in the controller's hand that the merge is topped with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MayDnaDigivolveNowArgs {
    #[serde(default = "default_anchor_source")]
    pub anchor: BindingRef,
    pub partner_filter: PredicateSpec,
    pub target_filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub cost: u16,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

fn default_anchor_source() -> BindingRef {
    BindingRef::Named("source".to_string())
}

fn is_zero_u16(v: &u16) -> bool {
    *v == 0
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

/// Args for `refire_card_effect` — see [`StepSpec::RefireCardEffect`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RefireCardEffectArgs {
    /// Binding holding the `CardHandle` of the card whose effect fires. The
    /// card must be a digivolution source of this effect's carrier.
    pub card: BindingRef,
    /// `on_play`, `when_digivolving`, or `on_play_or_when_digivolving`.
    pub timing: String,
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

/// Args for `trash_from_top` — mill `count` cards from the top of `of`'s
/// deck. `count` is a `FormulaSpec` (unify-dsl-scalar-and-comparators): a
/// bare int `count: 2` is a literal, a map is a runtime formula (e.g.
/// BT15-102's "trash the top 2 cards … for each of this Digimon's level 6
/// digivolution cards" = `{ base: 0, per: source_stack_count_filtered, … }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashFromTopArgs {
    pub of: PlayerRef,
    pub count: FormulaSpec,
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
    /// collapse §3.1 — player-elected top/bottom. The executor installs a
    /// binary top/bottom `pending_selection` and places at the chosen end.
    /// Only valid on steps whose executor intercepts `Choice` before placement
    /// (currently `place_remainder_on_deck` and the place-on-security verb); on
    /// any other step `map_stack_position` hard-panics by design.
    Choice,
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

/// collapse-dsl-step-idioms §2 — args for the `reveal_search` composite.
///
/// ```yaml
/// - reveal_search:
///     of: you
///     count: 4
///     buckets:
///       - { filter: { any_of: [{ name_contains: "Garurumon" }, { trait_has: "X Antibody" }] },
///           to: hand, max: 2, prompt: "Add 2 [Garurumon]/[X Antibody] cards to hand" }
///     remainder: bottom
/// ```
///
/// Lowers to `reveal_top_deck { count } → select_reveal_buckets { buckets } →
/// per-bucket reveal-move → place_remainder_on_deck { position }`. A bucket's
/// `to` routes its picks (hand → `add_to_hand_from_reveal`, trash →
/// `trash_from_reveal`, deck → `return_to_deck_from_reveal` at the deck
/// bottom). A non-`optional` bucket forces picking `max` cards (or all matching
/// if fewer); `optional` lets the player pick 0..=max.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RevealSearchArgs {
    pub of: PlayerRef,
    pub count: u8,
    pub buckets: Vec<RevealSearchBucket>,
    pub remainder: RevealRemainder,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RevealSearchBucket {
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    pub to: RevealSearchDest,
    pub max: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Destination zone for a `reveal_search` bucket's picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RevealSearchDest {
    Hand,
    Trash,
    /// Bottom of the owner's deck.
    Deck,
}

/// Where the un-picked remainder of the reveal pool goes. `choose` is a
/// player-elected top/bottom (lowers through §3.1's `StackPosition::Choice`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RevealRemainder {
    Top,
    Bottom,
    Choose,
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
    PlayFree,
    /// Place the picked card as the bottom digivolution card of `target`.
    BottomSourceOf {
        target: BindingRef,
    },
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
    /// De-Digivolve magnitude (how many digivolution cards to trash). A
    /// `FormulaSpec` (unify-dsl-scalar-and-comparators): a bare int
    /// `amount: 2` is a literal, a map is a runtime formula. The retired
    /// `amount_fn` key survives as a deserialize alias. At compile time a
    /// literal routes to the compiled `amount` integer and any other formula to
    /// `amount_fn`, byte-identical to the pre-unification form.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "amount_fn")]
    pub amount: Option<FormulaSpec>,
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

/// collapse §3.3 — the single source-polymorphic place-on-security verb. It
/// supersedes the former family (`place_on_security` hand-only,
/// `place_self_at_security`, `place_self_option_at_security`,
/// `place_permanent_on_security`, `…_and_handle_replacement`,
/// `…_bottom_…_and_cancel_replacement`, `…_observed`). The `source` chooses
/// what is placed; `disposition` selects the replacement-system interaction for
/// a permanent leaving the field (only meaningful when `source: permanent`).
/// It lowers to the EXISTING compiled variants (byte-identical behavior).
///
/// ```yaml
/// - place_on_security: { source: { card: chosen }, of: you, position: choice }
/// - place_on_security: { source: self_option, position: bottom, face_up: false }
/// - place_on_security: { source: { permanent: subj }, position: bottom, disposition: handle }
/// - place_on_security: { source: { permanent: subj }, position: bottom, disposition: cancel }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceOnSecurityArgs {
    pub source: SecuritySource,
    #[serde(default = "default_player_ref_you")]
    pub of: PlayerRef,
    pub position: StackPosition,
    #[serde(default)]
    pub face_up: bool,
    /// Replacement-system interaction (permanent sources only). `cancel`
    /// requires `position: bottom` (the engine cancel-via-security tuck is
    /// bottom-only); `observed` may set `include_sources`.
    #[serde(
        default,
        skip_serializing_if = "SecurityReplacementDisposition::is_none"
    )]
    pub disposition: SecurityReplacementDisposition,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_sources: bool,
}

/// What `place_on_security` puts onto the security stack.
///
/// NOTE: `#[serde(untagged)]` (with struct/marker variants), NOT externally
/// tagged — externally-tagged newtype variants serialize as `Value::Tagged`
/// (`!variant value`), which breaks the `serde_yml::from_value(Value::Mapping)`
/// round-trip in `DeclarativeClause::typed_body()` used by `kind: replacement`
/// bodies (the same constraint documented on `LinkCardsCount`). Untagged
/// struct variants are plain maps `{ card: … }` / `{ permanent: … }` that work
/// on both the streaming and from_value paths; the self markers are strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SecuritySource {
    /// A bound card from hand or trash (`{ card: <binding> }`); the binding's
    /// own `zone` selects hand vs trash, exactly as the former hand-only
    /// `place_on_security` did.
    Card { card: BindingRef },
    /// A battle-area permanent bound by name (`{ permanent: <binding> }`).
    Permanent { permanent: BindingRef },
    /// `self` (own source permanent) / `self_option` (own source Option).
    Marker(SecuritySelfMarker),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecuritySelfMarker {
    #[serde(rename = "self")]
    SelfPermanent,
    SelfOption,
}

/// Replacement-system disposition for a permanent placed on security as it
/// leaves the field. Only legal with `source: permanent` (validated at compile).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SecurityReplacementDisposition {
    /// Plain placement; no replacement interaction.
    #[default]
    None,
    /// Place at the security bottom to CANCEL the in-flight leave replacement.
    Cancel,
    /// Place on security as the custom-handled outcome of the replacement.
    Handle,
    /// Place on security as an observed (non-replacement-consuming) move.
    Observed,
}

impl SecurityReplacementDisposition {
    fn is_none(&self) -> bool {
        matches!(self, SecurityReplacementDisposition::None)
    }
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
    /// Record the number of cards ACTUALLY trashed by this step into a named
    /// literal binding, consumable by later steps via `{ binding_value: <name> }`
    /// (e.g. `{ floor_div: [{ binding_value: trashed }, 2] }`). Defaults to
    /// `None` (no binding). On the no-op path (opponent hand already ≤ target)
    /// the binding is set to 0. G-DSL-TRASH-COUNT-RESULT-BINDING (BT19-075
    /// "For every 2 cards trashed by this effect, delete 1 of your opponent's
    /// Tamers").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_count_as: Option<String>,
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
    /// When `true`, the placed bottom digivolution source is marked
    /// face-down. Omitted → face-up (the default).
    #[serde(default)]
    pub face_down: bool,
    /// Bind the placed card's `CardHandle` under this name AFTER a
    /// successful placement (absent on a failed / skipped placement, so a
    /// downstream consumer like `refire_card_effect` silently no-ops when
    /// nothing was placed). Survives the zone move — unlike the shifting
    /// trash/hand index bound by the upstream select step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_placed_as: Option<String>,
}

/// Args for `place_as_top_source` — same shape as
/// `PlaceAsBottomSourceArgs`, but the card is inserted at the TOP-source
/// position (directly beneath the permanent's active top card) instead of
/// the bottom of the stack. G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceAsTopSourceArgs {
    pub source: BindingRef,
    pub target: BindingRef,
    /// When `true`, the placed top digivolution source is marked face-down.
    /// Omitted → face-up (the default; no current consumer sets this — it
    /// exists for args-shape parity with `place_as_bottom_source`).
    #[serde(default)]
    pub face_down: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashSelectedSourcesArgs {
    pub source_refs: String,
}

/// Args for `return_selected_sources_to_deck` — return each
/// `select_own_sources`-bound digivolution source card to its owner's deck at
/// `position` (top or bottom). Sibling of `TrashSelectedSourcesArgs` with a
/// destination position, like the `position`-carrying source/reveal verbs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnSelectedSourcesToDeckArgs {
    pub source_refs: String,
    pub position: StackPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceSelectedCardUnderTamerArgs {
    pub card: BindingRef,
    pub tamer: BindingRef,
    #[serde(default)]
    pub face_down: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceSelectedSourcesUnderTamerArgs {
    pub source_refs: String,
    pub tamer: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_count_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveMatchingSourcesUnderTamerArgs {
    pub from: BindingRef,
    pub tamer: BindingRef,
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_count_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashTopStackedSourcesArgs {
    pub target: BindingRef,
    pub count: crate::formula::FormulaSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromHandArgs {
    pub of: PlayerRef,
    pub hand_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
    /// PUPPETS-G030 — when `true`, the played Digimon's own `[On Play]`
    /// effects do NOT activate for this play event. Only honored by
    /// `play_from_trash_free` (BT5-106's [Security] clause: "Any [On Play]
    /// effects on Digimon played with this effect don't activate."). The
    /// suppression is scoped to the just-played permanent and this single
    /// play; other permanents' On Play and every other timing are
    /// unaffected. `false` (the default) preserves prior behavior.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppress_on_play: bool,
    /// G-PLAY-ENTERS-SUSPENDED (judge-quiz Q28 / EX5-060 Dragomon: "Your
    /// opponent plays 1 level 4 or lower Digimon card from their trash
    /// SUSPENDED without paying the cost"). When `true`, the played
    /// permanent enters the battle area suspended. Only honored by
    /// `play_from_trash_free`. `false` (the default) preserves prior
    /// behavior.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suspended: bool,
    /// Bind the resulting permanent handle for use in later steps in the
    /// same body (mirrors `play_from_hand_free`'s `bind_as`, for the
    /// cost-paying / cost-reduced play). Only honored by `play_from_hand`;
    /// rejected at compile time on `play_from_trash` / `play_from_trash_free`
    /// (which share these args) so it cannot silently no-op. None (the
    /// default) preserves prior behavior.
    /// G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER (BT25-039 Sirenmon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
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

/// `play_or_use_from_hand:` args — the unified "play OR use 1 card from hand
/// with the cost reduced by N" verb (Aces / BEATBREAK printed wording). The
/// hand card is bound by an upstream `select_hand` step (whose filter scopes
/// the candidate set — e.g. `[Glowing Dawn]` trait — and whose `kind`/predicate
/// allows Digimon / Tamer / Option / Dual picks). At resolution the verb
/// inspects the bound card's kind: Digimon / Tamer are PLAYED, an Option is
/// USED, and a DUAL card surfaces a "Play as Digimon / Use as Option" face
/// choice (no auto-selection — §17). `cost_delta` applies to the appropriate
/// face cost (play cost for the play path, use cost for the use path).
///
/// ST23-04 / ST23-08 / BT25-041 shape. `G-PLAY-OR-USE-FROM-HAND`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayOrUseFromHandArgs {
    pub of: PlayerRef,
    /// Names the `select_hand` binding (a `bind_as` from that step) carrying
    /// the chosen hand index.
    pub hand_index: BindingRef,
    /// Cost adjustment applied to the played/used card's face cost. `None`
    /// plays/uses at full printed cost; `{ reduce: 3 }` is the shipped shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
}

/// `play_or_use_from_sources:` args — the unified source-origin sibling of
/// `play_or_use_from_hand`. The source card is bound by an upstream
/// `select_own_sources` step. At resolution the verb inspects the selected
/// source card's kind: Digimon/Tamer cards are played from their stack without
/// paying the cost, Option cards are used from that stack, and DUAL cards
/// surface a "Play as Digimon / Use as Option" face choice.
///
/// EX12-077 shape. `G-DSL-PLAY-OR-USE-FROM-SOURCES`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayOrUseFromSourcesArgs {
    pub of: PlayerRef,
    /// Names the `select_own_sources` binding carrying the chosen source ref.
    pub card: BindingRef,
    /// EX12 uses `cost: free`, but non-free deltas are also honored by both
    /// the play and Option-use halves.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "cost_delta")]
    pub cost: Option<CostDelta>,
}

/// Play a card currently in the transient reveal pool without paying its
/// memory cost. `card` is normally bound by `select_reveal` or
/// `select_reveal_buckets`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromRevealedFreeArgs {
    pub of: PlayerRef,
    pub card: BindingRef,
    /// Bind the resulting permanent handle for use in later steps in the
    /// same body. None (the default) preserves prior behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    /// G-DSL-PLAY-FROM-REVEALED-COST-REDUCED — optional cost adjustment for the
    /// reveal-pool play. `None` (the default) plays for free, preserving prior
    /// behavior; `{ reduce: N }` makes the controller pay the printed cost minus
    /// N (clamped at 0). Mirrors `play_from_hand`'s `cost_delta`. BT25-074 shape:
    /// "play 1 ... among them with the cost reduced by 5."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseOptionFromHandArgs {
    pub of: PlayerRef,
    pub filter: crate::predicate::PredicateSpec,
    /// When true, candidate Options must have a printed use cost less than or
    /// equal to the next clockwise opponent's current memory. This is the
    /// BT24-085 shape ("with a use cost no greater than your opponent's
    /// memory") without adding a one-off formula primitive.
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_cost_lte_opponent_memory: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// `use_option_from_trash:` args — effect-driven Option USE from the
/// controller's TRASH, with `cost` applied to the printed use cost (Gap 2,
/// `G-DSL-USE-OPTION-FROM-SOURCES`). Trash analogue of `use_option_from_hand`.
///
/// Drivers: BT25-083 ("use 1 [Three Musketeers] Option from your trash with the
/// cost reduced by 3" → `cost: { reduce: 3 }`), BT21-062's trash leg
/// ("use 1 [Ragnarok Cannon] from … trash without paying the cost" → `cost: free`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseOptionFromTrashArgs {
    pub of: PlayerRef,
    pub filter: crate::predicate::PredicateSpec,
    /// Cost applied to the Option's printed USE cost. `free` = pay nothing;
    /// `{ reduce: N }` = pay `max(0, use_cost - field_reductions - N)`; `printed`
    /// / omitted = pay the printed use cost (less field reductions). Reuses the
    /// shared `CostDelta` shape (same as the play half's `cost:`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostDelta>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// `use_option_from_revealed:` args — use an Option previously picked from the
/// reveal pool (a `select_reveal` binding) with `cost` applied to its printed
/// use cost (omitted = free). The `card` binding resolves to the reveal-pool
/// card handle at run time. Driver EX7-048 clause 1.
/// `G-DSL-USE-OPTION-FROM-SOURCES`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseOptionFromRevealedArgs {
    pub of: PlayerRef,
    /// The `select_reveal` binding naming the Option to use.
    pub card: BindingRef,
    /// Cost applied to the Option's printed USE cost. `free` / omitted = pay
    /// nothing; `{ reduce: N }` = pay `max(0, use_cost - field_reductions - N)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostDelta>,
}

/// `use_option_from_sources:` args — use an Option from a permanent's
/// digivolution stack (a `select_own_sources` binding) with `cost` applied to
/// its printed use cost (omitted = free). `card` names the source-refs binding;
/// its carrier permanent + card handle are recovered from the binding. The
/// engine `OptionSource::Source` fork resolves both `card_sources` and
/// `linked_cards`. Driver BT25-085. `G-DSL-USE-OPTION-FROM-SOURCES`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseOptionFromSourcesArgs {
    pub of: PlayerRef,
    /// The `select_own_sources` binding naming the source Option to use.
    pub card: BindingRef,
    /// Cost applied to the Option's printed USE cost. `free` / omitted = pay
    /// nothing; `{ reduce: N }` = pay `max(0, use_cost - field_reductions - N)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostDelta>,
}

/// `use_option_bound:` args — use a `select_union_zone{hand,trash}`-bound Option
/// from its true origin zone, with `cost` applied to its printed use cost
/// (omitted = free). Driver BT21-062 "use 1 [Ragnarok Cannon] from your hand or
/// trash without paying the cost". `G-DSL-USE-OPTION-FROM-SOURCES`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseOptionBoundArgs {
    /// The `select_union_zone` `bind_as` naming the picked Option.
    pub binding: String,
    /// Cost applied to the Option's printed USE cost. `free` / omitted = pay
    /// nothing; `{ reduce: N }` = pay `max(0, use_cost - field_reductions - N)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostDelta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CostDelta {
    /// Cost REDUCTION subtracted from the printed play cost (clamped at 0).
    /// `reduce` is a `FormulaSpec` (unify-dsl-scalar-and-comparators): a bare
    /// int `{ reduce: 2 }` is `FormulaSpec::Literal(2)`; a map
    /// `{ reduce: { floor_div: [...] } }` is a runtime formula (e.g. AD1-019's
    /// "reduce by 1 for every 2 of your Tamers' colors"). The retired
    /// `reduce_fn` key survives as a deserialize alias so existing YAML keeps
    /// parsing. At compile time a `Literal` routes to `CompiledCostDelta::Reduce`
    /// and any other formula to `CompiledCostDelta::ReduceFn`, byte-identical to
    /// the pre-unification encoding. G-FORMULA-COST-DELTA.
    Reduce {
        #[serde(alias = "reduce_fn")]
        reduce: crate::formula::FormulaSpec,
    },
    Keyword(CostDeltaKeyword),
    Literal(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CostDeltaKeyword {
    Free,
    Printed,
}

/// `play_union_bound_free:` args — play a card previously picked by a
/// `select_union_zone` step, **without paying its cost**, from its true
/// origin zone (hand, trash, or material). `binding` names the `select_union_zone`
/// binding (a `bind_as` from that step). The origin zone is recorded in the
/// binding itself, so this step needs no zone parameter.
///
/// PUPPETS-G014 substrate. Consumed by EX11-022 / ST19-08 / BT22-098-shape
/// "play 1 of the chosen cards … without paying the cost" effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayUnionBoundFreeArgs {
    /// Name of the `select_union_zone` binding to consume. Must name a
    /// `bind_as` declared by an earlier `select_union_zone` step — the
    /// binding carries the origin zone the step replays the card from.
    pub binding: String,
    /// Bind the just-played permanent handle for use in later steps in the
    /// same body (e.g. a Task 11 cleanup step). `None` preserves no binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    /// When true, the played Digimon's own On Play effects do not activate.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppress_on_play: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnionBoundArgs {
    /// Name of the `select_union_zone` binding to consume. The binding carries
    /// the selected card's origin zone so consumers can move it from the right
    /// place without re-prompting.
    pub binding: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayUnderTamerSourceArgs {
    pub source_refs: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayFromMaterialsArgs {
    pub target: BindingRef,
    pub source_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suppress_on_play: bool,
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

/// Source zone for the result (fusing-in) card in an `app_fuse` step.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AppFuseZone {
    #[default]
    Hand,
    Trash,
}

/// `app_fuse:` — effect-initiated App Fuse. Plays an App-Fusion-capable Digimon
/// card from `from` ONTO one of your field Digimon that already has the named
/// App-Fusion materials linked. Two engine-driven selections (own permanent,
/// then result card); no explicit target binding (both are fresh picks). The
/// result card's App-Fusion materials live on its own `app_fusion` alt-path, so
/// this step only carries the source zone + an optional result-card filter. See
/// `docs/superpowers/specs/2026-06-13-effect-initiated-app-fuse-design.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppFuseArgs {
    /// Zone holding the result card (`hand` default, or `trash`).
    #[serde(default)]
    pub from: AppFuseZone,
    /// Optional predicate on the result card (e.g. BT24-087's System/Life/
    /// Transmutation trait gate). `None` = any App-Fusion-capable Digimon card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_filter: Option<PredicateSpec>,
    /// "may" — PASS is legal at each selection. The shipped riders set this
    /// true; defaults false to match every other step's convention.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
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

/// `effect_initiated_dna_digivolve_hand_partner:` — DNA digivolve where one
/// material is a battle-area permanent (`target`) and the other is a card in
/// hand (`hand_partner`); the merged permanent is topped with `from_hand`
/// (the result card, also from hand). Used by BT17-095 Clause B, whose printed
/// text reads "That Digimon and a card in the hand may DNA digivolve into a
/// Digimon card with [Omnimon] in its name in the hand."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectDnaDigivolveHandPartnerArgs {
    /// The on-field DNA material (`requirement1`).
    pub target: BindingRef,
    /// The hand-card DNA material (`requirement2`).
    pub hand_partner: BindingRef,
    /// The resulting evolved card, pulled from hand and stacked on top.
    pub from_hand: BindingRef,
    pub cost: CostDelta,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,
}

/// `effect_initiated_dna_digivolve_trash_partner:` — DNA digivolve where one
/// material is a battle-area permanent (`target`) and the other is a card in
/// the controller's TRASH (`trash_partner`); the merged permanent is topped
/// with `from_hand` (the result card, also from hand). BT18-015 / BT18-073
/// `[On Deletion]` shape ("1 of your [Machinedramon] and 1 [Kimeramon] in the
/// trash may DNA digivolve into [Millenniummon] in the hand"). The trash
/// material moves STRAIGHT into the merged stack (never an independently
/// played permanent — no `[On Play]` fires), mirroring DCGO
/// `CreateNewPermanent` + jogress. G-DSL-DNA-TRASH-PARTNER.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectDnaDigivolveTrashPartnerArgs {
    /// The on-field DNA material (`requirement1`).
    pub target: BindingRef,
    /// The trash-card DNA material (`requirement2`).
    pub trash_partner: BindingRef,
    /// The resulting evolved card, pulled from hand and stacked on top.
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
    #[serde(deserialize_with = "deserialize_modifier_value")]
    pub value: FormulaSpec,
    pub expiry: String, // parsed as enum in Task 12 validation
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddModifierArgs {
    pub target: ModifierTarget,
    pub modifier: String,
    #[serde(deserialize_with = "deserialize_modifier_value")]
    pub value: FormulaSpec,
    pub expiry: String,
    /// Structured payload for payload-bearing modifiers. Required for
    /// `modifier: TreatAsDigimon` ("treat this permanent as a [DP] DP
    /// Digimon"), forbidden for every other modifier (validated). Carries
    /// the synthetic identity the target is treated as while the modifier
    /// is live; lowers to `ModifierPayload::SynthIdentity`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synth_identity: Option<SynthIdentitySpec>,
    /// CONTINUOUS mass modifier: instead of a one-time scan over the CURRENT
    /// matches, install a source-independent floating effect re-applied to the
    /// live candidate set every tick — so Digimon that ENTER during the window
    /// also receive it ("Until [turn], all of your opponent's Digimon get ±X").
    /// Only meaningful with a `target:` FILTER (a single-permanent `bind:` target
    /// is one-shot). The `expiry` governs the window; the effect survives the
    /// source leaving the field (e.g. an `[On Deletion]` install).
    /// G-CONTINUOUS-MASS-DP-DEBUFF.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub continuous: bool,
}

/// The synthetic Digimon identity a permanent is "treated as" while a
/// `TreatAsDigimon` modifier is live (e.g. RizeGreymon treating a Marcus
/// Damon Tamer as a 3000 DP Digimon). `dp` is required; `kind` defaults to
/// `Digimon` (the only kind this mechanic targets in printed cards);
/// `level`/`colors`/`traits` default empty when the text grants none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SynthIdentitySpec {
    pub dp: i32,
    #[serde(default = "synth_identity_default_kind")]
    pub kind: crate::spec::CardKind,
    #[serde(default)]
    pub level: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<crate::spec::ColorSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
}

fn synth_identity_default_kind() -> crate::spec::CardKind {
    crate::spec::CardKind::Digimon
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddPlayerModifierArgs {
    pub target_player: PlayerRef,
    pub modifier: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_modifier_value_opt"
    )]
    pub value: Option<FormulaSpec>,
    pub expiry: String,
}

/// Deserialize a modifier `value:` field (unify-dsl-scalar-and-comparators).
/// Accepts the canonical `FormulaSpec` — a bare int `value: 3000`
/// (`FormulaSpec::Literal`) or a formula map directly
/// (`value: { base: 0, per: stack_size, delta: 1000 }`) — and, for
/// back-compat, the retired `ModifierValueSpec::Formula` wrapper
/// `value: { formula: <FormulaSpec> }`. The wrapper is unwrapped to the inner
/// formula so the authored vocabulary is uniformly `FormulaSpec`.
pub(crate) fn deserialize_modifier_value<'de, D>(de: D) -> Result<FormulaSpec, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Compat {
        /// Legacy `{ formula: <FormulaSpec> }` wrapper (retired ModifierValueSpec::Formula).
        Wrapped { formula: FormulaSpec },
        /// Canonical: a bare int or a formula map.
        Direct(FormulaSpec),
    }
    Ok(match Compat::deserialize(de)? {
        Compat::Wrapped { formula } => formula,
        Compat::Direct(f) => f,
    })
}

/// `Option` form of [`deserialize_modifier_value`] for optional `value:` fields.
/// Only invoked when the key is present (absence is handled by `#[serde(default)]`).
pub(crate) fn deserialize_modifier_value_opt<'de, D>(de: D) -> Result<Option<FormulaSpec>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_modifier_value(de).map(Some)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AllowDigixrosMaterialZoneArgs {
    pub zone: Zone,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigixrosCostDeltaArgs {
    pub delta: i16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreattachDigixrosMaterialArgs {
    pub card: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigixrosWildcardArgs {
    pub card: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<Zone>,
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
    /// Permanents that receive the granted body. A predicate target walks both
    /// players' battle areas and grants to every match; a binding target grants
    /// to exactly the previously-selected permanent.
    pub target: ModifierTarget,
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
    /// Per-permanent grant target. Required unless `continuous: true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<BindingRef>,
    pub source_kind: EffectSourceKindSpec,
    #[serde(default)]
    pub source_controller: EffectControllerSpec,
    pub expiry: String,
    /// G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA (judge-quiz Q28 /
    /// BT20-059 Gankoomon (X Antibody)): when `true`, the immunity is a
    /// source-independent CONTINUOUS field effect over `targets` — every
    /// declarative tick re-scans the live candidate set, so a Digimon
    /// played AFTER the grant (Sistermon Ciel played from trash by
    /// Dragomon during the protected window) is covered too, matching
    /// DCGO's `CanNotAffectedClass` re-evaluated CardCondition. Requires
    /// `targets`; `target` must be omitted.
    #[serde(default, skip_serializing_if = "is_false")]
    pub continuous: bool,
    /// Predicate over battle-area permanents for the `continuous` form
    /// (e.g. `{ of: you, kind: digimon }`), evaluated relative to the
    /// effect controller every tick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<PredicateSpec>,
}

/// PUPPETS-G024 — arguments for `grant_narrow_opponent_effect_protection`.
/// Installs the opponent-scoped DP-reduction + De-Digivolve protection
/// bundle on `target`. No source-kind/controller knobs: this verb is
/// keyed to the BT16-055-style "by your opponent's effects" narrow
/// protection and is always opponent-scoped by construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantNarrowOpponentEffectProtectionArgs {
    pub target: BindingRef,
    pub expiry: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldSelector {
    LowestDp,
    HighestDp,
    /// Restrict the selection to the candidate permanent(s) with the
    /// lowest printed play cost. Used by EX4-073 clause C ("delete 1 of
    /// your opponent's Digimon or Tamers with the lowest play cost").
    /// G-PLAY-COST-AGGREGATE.
    LowestPlayCost,
    /// Restrict the selection to the candidate permanent(s) with the
    /// highest printed play cost. Used by EX11-044 ("delete 1 of your
    /// opponent's highest play cost Digimon or Tamers").
    /// G-HIGHEST-PLAY-COST-SELECTOR.
    HighestPlayCost,
    /// Restrict the selection to candidate permanent(s) with the fewest
    /// digivolution cards beneath their top card. Used by effects such as
    /// "delete 1 of your opponent's Digimon with the fewest digivolution
    /// cards".
    LowestMaterialCount,
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
    /// PASSing this optional pick CONTINUES the clause with the binding
    /// unresolved (DCGO: a declined `SelectPermanentEffect` resolves with an
    /// empty list and the coroutine continues), so binding-gated follow-ups
    /// (`binding_exists` / `binding_absent`) and independent legs still run.
    /// Default `false` keeps the historical permanent-select semantic —
    /// decline drops the rest of the clause — which many existing cards use
    /// as their accept/decline cost gate (e.g. P-169's "by suspending this
    /// Tamer"). G-OPT-REFUND-ON-DECLINE.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub continue_on_decline: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
    /// collapse-dsl-step-idioms §1 — explicit scoped action-tail run on accept
    /// with the selection binding in scope. Composed with the implicit
    /// dispatcher tail (the rest of the process body) without double-running:
    /// `then` runs only when a pick is made, whereas steps placed after this
    /// select in the process body run regardless. Closure-free — this is the
    /// cloneable VM's `ResumeFrame::RunTail` data (mirrors `SelectOwnSourcesArgs`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<StepSpec>,
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
    /// Mark this prompt as a cost-pay (only meaningful when
    /// `optional: true`). When set, declining the prompt aborts the rest of
    /// the clause body — the printed text pattern "By trashing X, do Y"
    /// where declining means Y does NOT run. Default `false` preserves the
    /// "you may pick X; then always do Y" semantics where the tail runs
    /// regardless. See `G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE` and the
    /// DCGO `ActivateClass.SetUpICardEffect` cost/effect split.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cost: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
    /// collapse-dsl-step-idioms §1 — explicit scoped action-tail run on accept
    /// with the selection binding in scope. Composed with the implicit
    /// dispatcher tail without double-running: `then` runs only when a pick is
    /// made (for a non-cost optional decline it runs scoped to the empty
    /// binding = no-op); steps after this select in the process body run
    /// regardless. Closure-free — the cloneable VM's `ResumeFrame::RunTail`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<StepSpec>,
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

/// Count-capped / different-name multi-pick over a carrier permanent's
/// digivolution-source stack — the batch sibling of `select_material`.
///
/// YAML shape:
///
/// ```yaml
/// - select_materials:
///     of_permanent: <carrier-binding>   # battle-area permanent (or BREEDING_TARGET)
///     max: 4
///     uniqueness: name            # "1 of each different name"
///     filter: { trait_has: "Royal Knight" }
///     bind_as: picked
/// ```
///
/// Picks are surfaced one-at-a-time through `pending_selection` (the
/// count-capped multi-select state machine); `uniqueness` shapes the
/// legal action mask after each pick — it never auto-picks. The bound
/// `CardList` can be consumed as a batch by `play_from_materials`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectMaterialsArgs {
    /// Carrier permanent whose digivolution sources are the candidate pool.
    pub of_permanent: BindingRef,
    /// Upper bound on the number of sources the player may pick.
    pub max: CountBound,
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    /// Per-pick uniqueness constraint. `name` means "at most one pick per
    /// distinct card name" — the printed-text "1 of each different name".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uniqueness: Option<crate::alt_path::DistinctBy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    /// When `true`, the player may commit zero picks. Default `false`:
    /// PASS only becomes legal after at least one pick.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional_zero: bool,
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

/// Opponent-side mirror of `SelectOwnSourcesArgs`. The candidate set is drawn
/// from the OPPONENT's battle-area digivolution-source stacks (every card below
/// the top card of each opponent permanent), with the same exact-N / up-to-N
/// `min`/`max` counts, PASS exposed once the minimum is met, optional `filter:`,
/// and stable cross-permanent source refs. `target:` restricts the picker to a
/// single opponent permanent binding (e.g. the opponent Digimon picked just
/// before). G-SELECT-OPPONENT-SOURCES — driver BT16-085 DNA branch.
///
/// `min`/`max` accept either a literal `u8` or a `{ formula: ... }` block
/// resolved once at execution time (G-DSL-SELECT-SOURCES-FORMULA-COUNT —
/// driver EX11-057 "For each of your [Ice-Snow] trait Digimon, trash any 1
/// digivolution card from your opponent's Digimon").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectOpponentSourcesArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<BindingRef>,
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    pub min: CountBound,
    pub max: CountBound,
    /// DCGO `TrashDigivolutionCards.cs` parity
    /// (`maxDigivolutionDiscardCount = Math.Min(digivolutionCardsSum,
    /// maxCount)`): when `true`, the resolved `min`/`max` clamp to the live
    /// candidate count at execution time — the pick affects
    /// `min(N, available)` and never aborts for "fewer than N available"
    /// (zero candidates silently skip the pick and continue the clause).
    ///
    /// Default `false` preserves the historical unpayable-cost semantics:
    /// a `min` above the live candidate count drops the continuation —
    /// committed cards (EX7-021 / EX7-023 / EX11-017 / EX8-066) build
    /// explicit availability LADDERS on that exact behavior. Do NOT flip
    /// this default.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clamp_to_available: bool,
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
    /// Running DP budget. Accepts a literal integer or a formula such as
    /// `{ source_dp: {} }` for "this Digimon's DP".
    pub dp_budget: crate::formula::FormulaSpec,
    #[serde(default)]
    pub min_picks: u8,
    /// Optional per-candidate predicate. Only opponent permanents satisfying
    /// this filter are eligible (e.g. `{ kind: digimon }` for card text that
    /// targets "their Digimon" specifically). Empty filter accepts all.
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

/// Multi-select of opponent permanents under a running PLAY-COST budget —
/// the play-cost analog of `SelectOpponentDpBudget`. The player picks
/// `min_picks..N` opponent permanents whose running printed-play-cost sum
/// never exceeds `play_cost_budget`; any single permanent whose individual
/// play cost exceeds the budget is excluded outright. Models card text of
/// the form "delete up to N play cost's total worth of their Digimon".
/// G-MULTI-SELECT-OPP-PLAY-COST-SUM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectOpponentPlayCostBudgetArgs {
    /// Running play-cost budget. Accepts a literal integer (`play_cost_budget:
    /// 6`) or a formula such as `{ base: 3, per: { source_stack_count: { filter:
    /// { name_is: "Vemmon" } } }, delta: 1 }` for "3 + 1 per [Vemmon] in this
    /// Digimon's digivolution cards" (P-094 Destromon). Mirrors the
    /// `dp_budget: FormulaSpec` field on `SelectOpponentDpBudgetArgs`. A bare
    /// integer still parses because `FormulaSpec`'s first untagged variant is
    /// `Literal(i32)`, so existing scalar users (EX4-073) are unaffected.
    pub play_cost_budget: crate::formula::FormulaSpec,
    #[serde(default)]
    pub min_picks: u8,
    /// Optional per-candidate predicate. Only opponent permanents satisfying
    /// this filter are eligible (e.g. `{ kind: digimon }` for card text that
    /// targets "their Digimon" specifically). Empty filter accepts all.
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Optional predicate the selected breeding permanent must satisfy.
    /// Used by Royal Knights cards like BT13-093 that require the
    /// breeding permanent to be a specific named host (e.g.
    /// `[King Drasil_7D6]`) before authoring a placement clause.
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

/// Per-zone predicate overrides for `select_union_zone`. Each field, when
/// present, supplies the predicate applied to candidates from that origin
/// zone, **overriding** the step's shared `filter`. Absent fields fall back
/// to `filter`. G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnionZoneFilters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trash: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<PredicateSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectUnionArgs {
    pub of: PlayerRef,
    pub zones: Vec<Zone>,
    /// Optional carrier binding used when `zones` includes `material`.
    /// The binding may resolve to a battle-area or breeding-area permanent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_of: Option<BindingRef>,
    pub filter: PredicateSpec,
    /// Per-zone filter overrides. When present, the predicate for a given
    /// origin zone (hand / trash / material) **replaces** the shared `filter`
    /// for candidates from that zone; zones without an override fall back to
    /// `filter`. This lets one union prompt offer "1 X from trash OR 1 Y from
    /// breeding sources" where X and Y carry different predicates
    /// (G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY, BT13-019). Omitted → the
    /// shared `filter` applies uniformly to every zone (legacy behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_filters: Option<UnionZoneFilters>,
    /// Restricts the `material` zone scan (when `material_of` is omitted, i.e.
    /// "scan all my field permanents") to carriers whose top card matches this
    /// predicate. Each candidate carrier's TOP CARD is evaluated against the
    /// predicate before its digivolution sources are enumerated; non-matching
    /// carriers contribute no source candidates. This is distinct from
    /// `filter`/`zone_filters.material`, which filter the candidate SOURCE
    /// CARDS — this filters the CARRIER. Lets one prompt offer "1 X from hand
    /// OR 1 X from under a fielded [King Drasil_7D6]" (G-UNION-HAND-SOURCE-PLAY,
    /// EX11-053). Omitted → every field permanent's sources are scanned (legacy
    /// behavior). Has no effect when `material_of` pins a specific carrier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_carrier_filter: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Mark this prompt as a cost-pay (only meaningful when
    /// `optional: true`). When set, declining the prompt aborts the rest of
    /// the clause body (any steps following this one AND the captured outer
    /// tail). See `SelectZoneArgs::cost` for the printed-text pattern this
    /// models, and prefer `then:` for steps that should only run on accept
    /// when the cost-pay is local to a sub-block.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cost: bool,
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
    /// Success-only branch. Used for source-placement costs where the
    /// selected card is the cost payment and the following benefit must not
    /// resolve when the selection is declined or unpayable.
    #[serde(default)]
    pub then: Vec<StepSpec>,
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
    /// Minimum number of cards that MUST be selected before the player can
    /// finish the selection. Defaults to 0 (any number, including zero, is
    /// acceptable — modulated further by `optional_zero`). When `min > 0` the
    /// selection cannot complete with fewer than `min` picks; if fewer than
    /// `min` candidates exist at all the step silently no-ops (the required
    /// cost is unpayable). G-SELECT-MULTI-MIN.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub min: u8,
    /// MP-30/31 (General Rules/FAQ): when true the required pick-count clamps to
    /// the number of available candidates — the player MUST affect
    /// `min(max, available)` targets and the step never no-ops for "fewer than N
    /// in play" (a mandatory "N of your opponent's Digimon" effect affects as
    /// many as are present, but cannot stop early when N are available). Use for
    /// EFFECT-TARGET selections; leave false for unpayable-cost selections.
    /// Orthogonal to `min` (the cost floor).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clamp_to_available: bool,
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
pub struct RepeatEffectChoiceArgs {
    pub count: crate::formula::FormulaSpec,
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default)]
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
    pub body: Vec<StepSpec>,
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

/// Which turn boundary should trigger the provenance deletion. Defaults to
/// `your_turn` (end of the scheduling player's turn), which is the behaviour
/// from Task 11 (EX11-022, EX11-061). P-165 ShoeShoemon needs
/// `opponents_turn` ("at the end of your opponent's turn, delete that token").
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeleteTurnBoundary {
    #[default]
    YourTurn,
    OpponentsTurn,
}

/// `schedule_delete_played_at_turn_end:` args — PUPPETS-G003. Schedules the
/// permanent named by `binding` to be deleted at the end of the current turn,
/// keyed to its stable identity (it is deleted even if battle-area indices
/// shift; a no-op if it already left). `binding` must name a `bind_as` from a
/// preceding free-play step (`play_union_bound_free` / `play_from_hand_free`
/// / `play_token`).
///
/// Consumed by "At turn end, delete the Digimon this effect played" riders
/// (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki) and by "at the end of your
/// opponent's turn, delete that token" riders (P-165 ShoeShoemon).
/// Use `at: opponents_turn` for the latter; omit `at` (or write `at: your_turn`)
/// for the former — the default preserves Task 11 behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScheduleDeletePlayedAtTurnEndArgs {
    /// Name of the permanent binding to delete at turn end. Must name a
    /// `bind_as` declared by an earlier free-play step in the same body.
    pub binding: String,
    /// Which turn boundary triggers the deletion. Defaults to `your_turn`.
    #[serde(default, skip_serializing_if = "is_default_turn_boundary")]
    pub at: DeleteTurnBoundary,
}

fn is_default_turn_boundary(v: &DeleteTurnBoundary) -> bool {
    matches!(v, DeleteTurnBoundary::YourTurn)
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

/// A source zone the `link_cards` step may draw a card from.
///
/// - `hand` / `trash`: the controller's hand / trash.
/// - `self_sources`: the effect's own permanent's digivolution cards.
/// - `own_digimon_sources`: any of the controller's Digimon's digivolution
///   cards (cross-permanent).
/// - `self_option`: Gap 3b — the Option card currently being played links
///   ITSELF onto the host (BT25-101 "you may link this card …"). Only valid in
///   an Option's `[Main]` body; the card is lifted out of `pending_option` so
///   the Standard dispose does not also trash it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkCardSourceZone {
    Hand,
    Trash,
    SelfSources,
    OwnDigimonSources,
    SelfOption,
}

/// Where the linked card is attached.
///
/// - `self`: the effect's own permanent (BT25-060 Rebootmon).
/// - `own_digimon`: a player-selected own Digimon, chosen per pick
///   (BT25-075 Vulcanusmon, BT25-089 Kazuki & Itsuki).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkCardsTo {
    #[serde(rename = "self")]
    SelfPermanent,
    OwnDigimon,
}

/// How many cards to link.
///
/// - `{ exactly: N }`: mandatory until N picks or no candidates remain.
/// - `{ up_to: N }`: each pick is declinable; the player may stop early.
///
/// NOTE: `#[serde(untagged)]` is required so that `serde_yml::from_value`
/// (used in `typed_body()` for `kind: delay` declarative bodies) can
/// deserialize these correctly. Externally-tagged tuple variants serialize as
/// `Value::Tagged` (`!variant value`) in serde_yml's internal Value
/// representation, incompatible with the `Value::Mapping` path in `typed_body()`.
/// Untagged struct variants serialize as plain maps `{ exactly: N }` / `{ up_to:
/// N }`, which work for both the streaming YAML deserializer and the
/// `serde_yml::from_value(Value::Mapping)` path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum LinkCardsCount {
    Exactly { exactly: u8 },
    UpTo { up_to: u8 },
}

/// The link cost the controller pays.
///
/// - `free`: pay nothing (BT25-060, BT25-075 "without paying the cost").
/// - `{ reduce: N }`: the printed link cost reduced by N (BT25-089 "the cost
///   reduced by 2"). The cards this step serves carry no base link cost in
///   this context, so both branches currently pay 0; the field is threaded so
///   a future card with a non-zero base cost can extend the lowering without a
///   schema change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LinkCardsCost {
    Free,
    Reduce(u8),
}

impl Default for LinkCardsCost {
    fn default() -> Self {
        LinkCardsCost::Free
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkCardsArgs {
    /// Source zones, in author-declared order. When ≥2 of them currently hold
    /// a filter-matching candidate, the player is first prompted to choose a
    /// zone (DCGO ST22_12 bool-select parity); when exactly 1 does, it is used
    /// directly with no extra prompt.
    pub from: Vec<LinkCardSourceZone>,
    /// Card-level filter applied to every candidate. Empty = match all.
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub filter: PredicateSpec,
    /// Where the chosen card(s) attach.
    pub to: LinkCardsTo,
    /// How many cards to link.
    pub count: LinkCardsCount,
    /// Link cost paid (defaults to `free`).
    #[serde(default)]
    pub cost: LinkCardsCost,
    /// Optional prompt override for the card-select step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Optional binding that captures the card(s) actually linked by this step
    /// (a `CardList`). The slot is created ONLY when ≥1 card is linked, so a
    /// downstream `if { binding_present: … }` runs the follow-up exclusively
    /// when a link occurred — the data-driven way to model "By linking 1 …, do
    /// Y" where Y is gated on the link (BT25-060 Rebootmon's DCGO `if (linked)`
    /// gate). Absent ⇒ no binding is recorded (the common terminal-link case).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,

    /// Optional predicate restricting which own Digimon may HOST the link
    /// (`to: own_digimon` only). The author encodes the link card's link
    /// requirement here (e.g. `effect_text_contains: Maquinamon` for EX11-027,
    /// whose `[Link]` condition is "host has [Maquinamon] in text") so the host
    /// select never offers an illegal host. Absent ⇒ any standing own Digimon.
    /// Mirrors DCGO `CanLinkToTargetPermanent`'s host-condition check.
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub host_filter: PredicateSpec,

    /// When true (`to: own_digimon` only), the effect's own source permanent is
    /// excluded from the host candidates — "link … to 1 of your OTHER Digimon"
    /// (EX11-027). Mirrors DCGO `CanSelectLinkTarget`'s `permanent !=
    /// card.PermanentOfThisCard()`. Default false ("1 of your Digimon").
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_source: bool,
}

/// Args for `relink_self_to_own_digimon` (G-DSL-LINK-RELINK-STANDING-PERMANENT).
/// Moves the effect's own standing permanent to become a link card on a chosen
/// OTHER own Digimon (EX11-027 Maquinamon "link this Digimon to 1 of your other
/// Digimon"). The host select always excludes self and applies `host_filter`
/// (the link requirement, e.g. `effect_text_contains: Maquinamon`). Backed by
/// the engine's `absorb_standing_digimon_as_link` (DCGO `IPlacePermanentToLinkCards`):
/// the permanent's digivolution sources are trashed and its top card becomes a
/// single link card on the host, firing `OnLink`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelinkSelfToOwnDigimonArgs {
    /// Host predicate — the link requirement the chosen host must satisfy.
    /// Absent ⇒ any standing own Digimon (other than self).
    #[serde(default, skip_serializing_if = "PredicateSpec::is_empty")]
    pub host_filter: PredicateSpec,
    /// Optional host-select prompt override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
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
