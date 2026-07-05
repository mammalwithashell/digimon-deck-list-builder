//! EffectContext — the curated API surface for card effect scripts.
//!
//! Card scripts mutate the game through this context (never directly).
//! `EffectContext` wraps `&mut Game` for `process` closures; `EffectReadContext`
//! wraps `&Game` for `condition` closures and tensor-time effect inspection.
//! Both expose the same read-only query surface.
//!
//! **File layout.** Selection-prompt helpers (`select_*`, `play_pending_security`,
//! `mark_security_face_up`, plus the private `install_field_selection`
//! shared implementation) live in `selections.rs` — they are numerous and
//! will grow substantially as the gap-closing roadmap adds multi-select,
//! ordered-permutation, cross-player, and budgeted-multi-select primitives.
//! Core mutations (memory, draw, trash, suspend, modifier grants) stay here.

mod action;
pub(crate) mod selections;

pub(crate) use action::app_fuse::{
    run_app_fuse_host_selection_step, run_app_fuse_result_selection_step,
    AppFuseHostSelectionState, AppFuseResultSelectionState,
};

pub use selections::{
    CountCappedZone, DistinctByMode, EffectContextSelectorScope, RevealBucketSelection,
};

/// Material-zone carrier resolver — branches battle-area vs. breeding-area
/// (`BREEDING_TARGET` sentinel) carriers. Used by the DSL `select_material*`
/// step lowering so its filter/bind closures read the same stack the engine
/// selection helper does.
pub(crate) use selections::material_carrier_permanent;

pub use crate::selection::{BreedingPermanentSelectionRef, SourceSelectionRef};

use crate::action::mask::{
    effect_attack_target_action_ids, effect_attack_target_action_ids_with_options,
};
use crate::action::space::{decode_attack, encode_attack, SECURITY_TARGET};
use crate::card_data::CardData;
use crate::card_source::CardHandle;
use crate::combat::{AttackError, AttackInitiator, AttackOpen, AttackResult, TargetConstraint};
use crate::digixros::{
    DigiXrosMaterialOrigin, DigiXrosMaterialValidationError, DigiXrosMaterialZone,
    DigiXrosTransaction, DigiXrosZoneAllowance,
};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::{enumerate_refireable_effects, ReFireableEffect, TimingFilter};
use crate::enums::{
    CardKind, DelayTrigger, EffectSourceKind, EffectTiming, Expiry, GamePhase, Keyword,
    ModifierType, PlaySource, PlayerId, StackPosition,
};
use crate::game::Game;
use crate::game::PendingWouldPlayOrigin;
use crate::game_actions::PlayFromHandCostResult;
use crate::modifiers::{
    EffectControllerFilter, EffectImmunityFilter, ModifierEntry, PlayerModifierEntry,
};
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::replacement::{ReplacementCause, ReplacementSubject};
use crate::rules::Rules;
use crate::scheduled_effects::ScheduledEffect;
use crate::selection::{AttackTarget, DeclineCallback, PendingSelection, SelectionKind};
use digimon_dsl::compiled::CompiledStep;

pub struct PartitionRequirement {
    pub label: &'static str,
    pub matches: Box<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackTargetRestriction {
    Any,
    PlayerOnly,
    DigimonOnly,
}

impl PartitionRequirement {
    pub fn new<F>(label: &'static str, matches: F) -> Self
    where
        F: Fn(&Game, SourceSelectionRef) -> bool + Send + Sync + 'static,
    {
        Self {
            label,
            matches: Box::new(matches),
        }
    }
}

fn source_kind_for_card_kind(kind: CardKind) -> EffectSourceKind {
    match kind {
        CardKind::Digimon | CardKind::DigiEgg | CardKind::Dual => EffectSourceKind::Digimon,
        CardKind::Tamer => EffectSourceKind::Tamer,
        CardKind::Option => EffectSourceKind::Option,
        CardKind::Token => EffectSourceKind::Rule,
    }
}

fn infer_effect_source_kind(
    game: &Game,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
) -> EffectSourceKind {
    if let Some(h) = source_permanent {
        if let Some(perm) = game
            .players
            .get(h.player as usize)
            .and_then(|p| p.battle_area.get(h.index as usize))
        {
            if let Some(top) = perm.card_sources.last() {
                if top.handle() == source_card {
                    return source_kind_for_card_kind(top.card_kind(&game.card_data));
                }
            }
        }
    }
    game.card_kind_for_handle(source_card)
        .map(source_kind_for_card_kind)
        .unwrap_or(EffectSourceKind::Rule)
}

/// Read-only view of game state for effect condition closures.
///
/// Wraps `&Game` so conditions can be evaluated without a mutable borrow —
/// which is required at tensor-build time (§3.1 / §3.2 parity fixes) to
/// decide whether a conditional DP modifier currently contributes.
pub struct EffectReadContext<'a> {
    pub game: &'a Game,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub source_kind: EffectSourceKind,
    pub player: PlayerId,
    replacement_cause: Option<ReplacementCause>,
    replacement_source_controller: Option<PlayerId>,
    replacement_subject_controller: Option<PlayerId>,
    /// Card whose cost is currently being inspected by a BeforePayCost hook.
    /// `None` outside play/digivolve cost calculation.
    pub cost_target_card: Option<CardHandle>,
    pub cost_target_from_hand: bool,
    /// True when the cost currently being computed is a DIGIVOLVE cost (as
    /// opposed to a play cost or option-use cost). Consumed by the
    /// `when_any_ally_digivolves_into` cost-reduction trigger so it fires
    /// only for digivolutions. `false` outside cost-calc dispatch and for
    /// non-digivolve costs. `G-COST-REDUCTION-DIGIVOLVE-INTO`.
    pub cost_is_digivolve: bool,
    /// Permanent(s) being digivolved or otherwise mutated by the play/
    /// digivolve action whose cost is currently being computed. Single
    /// entry for a normal digivolve (the digivolve-target permanent),
    /// two for DNA digivolve (both materials), empty for play-from-hand
    /// or option use. Consumed by the `source_is_cost_target_permanent`
    /// predicate to gate effects scoped to "when THIS Digimon would
    /// digivolve" (printed semantics — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
    pub cost_target_permanents: Vec<PermanentHandle>,
}

impl<'a> EffectReadContext<'a> {
    pub fn new(
        game: &'a Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
    ) -> Self {
        let source_kind = infer_effect_source_kind(game, source_card, source_permanent);
        Self::new_with_source_kind(game, source_card, source_permanent, source_kind, player)
    }

    pub fn new_with_source_kind(
        game: &'a Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        source_kind: EffectSourceKind,
        player: PlayerId,
    ) -> Self {
        Self {
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
            replacement_cause: None,
            replacement_source_controller: None,
            replacement_subject_controller: None,
            cost_target_card: None,
            cost_target_from_hand: false,
            cost_is_digivolve: false,
            cost_target_permanents: Vec::new(),
        }
    }

    pub fn new_with_cost_target(
        game: &'a Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
        cost_target_card: CardHandle,
        cost_target_from_hand: bool,
    ) -> Self {
        let source_kind = infer_effect_source_kind(game, source_card, source_permanent);
        Self {
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
            replacement_cause: None,
            replacement_source_controller: None,
            replacement_subject_controller: None,
            cost_target_card: Some(cost_target_card),
            cost_target_from_hand,
            cost_is_digivolve: false,
            cost_target_permanents: Vec::new(),
        }
    }

    /// Mark the cost currently being computed as a digivolve cost. Chains
    /// after `new_with_cost_target`. `G-COST-REDUCTION-DIGIVOLVE-INTO`.
    pub fn with_cost_is_digivolve(mut self, is_digivolve: bool) -> Self {
        self.cost_is_digivolve = is_digivolve;
        self
    }

    /// Attach the digivolve target permanents (one for normal digivolve,
    /// two for DNA, empty for play-from-hand). Chains after
    /// `new_with_cost_target`. Consumed by the
    /// `source_is_cost_target_permanent` predicate.
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    pub fn with_cost_target_permanents(mut self, perms: Vec<PermanentHandle>) -> Self {
        self.cost_target_permanents = perms;
        self
    }

    /// True if this effect's `source_permanent` is one of the permanents
    /// being digivolved by the action whose cost is currently being
    /// computed. Returns `false` outside cost dispatch or when this
    /// effect has no source permanent.
    pub fn source_is_cost_target_permanent(&self) -> bool {
        let Some(source) = self.source_permanent else {
            return false;
        };
        self.cost_target_permanents.iter().any(|h| *h == source)
    }

    pub fn was_digixros(&self) -> bool {
        // Semantic firewall: a non-DigiXros cast-time assembly (BT15-102)
        // rides the same pending transaction but is NOT a DigiXros.
        self.game
            .pending_digixros_transaction()
            .is_some_and(|transaction| transaction.is_digixros)
    }

    pub fn digixros_count(&self) -> u8 {
        self.game
            .pending_digixros_transaction()
            .filter(|transaction| transaction.is_digixros)
            .map(|transaction| transaction.digixros_count)
            .unwrap_or(0)
    }

    pub fn pending_digixros_play_card(&self) -> Option<CardHandle> {
        self.game
            .pending_digixros_transaction()
            .map(|transaction| transaction.played_card)
    }

    pub fn with_replacement_context(
        mut self,
        cause: ReplacementCause,
        source_controller: Option<PlayerId>,
        subject_controller: Option<PlayerId>,
    ) -> Self {
        self.replacement_cause = Some(cause);
        self.replacement_source_controller = source_controller;
        self.replacement_subject_controller = subject_controller;
        self
    }

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn rules(&self) -> &Rules {
        &self.game.rules
    }

    pub fn card_data(&self) -> &[CardData] {
        &self.game.card_data
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }

    pub fn player_state(&self, id: PlayerId) -> &Player {
        self.game.player(id)
    }

    pub fn my_player(&self) -> &Player {
        self.game.player(self.player)
    }

    pub fn replacement_cause(&self) -> Option<ReplacementCause> {
        self.replacement_cause
    }

    pub fn replacement_source_controller(&self) -> Option<PlayerId> {
        self.replacement_source_controller
    }

    pub fn replacement_subject_controller(&self) -> Option<PlayerId> {
        self.replacement_subject_controller
    }

    pub fn opponent_id(&self) -> PlayerId {
        self.game.next_clockwise(self.player)
    }

    pub fn opponent(&self) -> &Player {
        self.game.player(self.opponent_id())
    }

    pub fn opponents(&self) -> Vec<PlayerId> {
        self.game.opponents(self.player)
    }

    pub fn battle_area(&self, id: PlayerId) -> &[Permanent] {
        &self.game.player(id).battle_area
    }

    pub fn hand(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).hand
    }

    pub fn trash(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).trash
    }

    pub fn security_count(&self, id: PlayerId) -> usize {
        self.game.player(id).security.len()
    }

    pub fn source_permanent(&self) -> Option<&Permanent> {
        let h = self.source_permanent?;
        let player = self.game.player(h.player);
        if h.index == crate::action::space::BREEDING_TARGET as u8 {
            return player.breeding_area.as_ref();
        }
        player.battle_area.get(h.index as usize)
    }

    pub fn cost_target_card_id(&self) -> Option<&str> {
        self.cost_target_card
            .and_then(|card| self.game.card_data_for_handle(card))
            .map(|data| data.card_id.as_str())
    }

    pub fn cost_target_has_trait(&self, needle: &str) -> bool {
        self.cost_target_card
            .and_then(|card| self.game.card_data_for_handle(card))
            .is_some_and(|data| data.traits.iter().any(|trait_name| trait_name == needle))
    }

    pub fn cost_target_from_hand(&self) -> bool {
        self.cost_target_from_hand
    }

    pub fn cost_reduction_source(&self) -> Option<PermanentHandle> {
        self.source_permanent
    }

    pub fn source_stack_source_count(&self) -> usize {
        self.source_permanent()
            .map(|perm| perm.card_sources.len().saturating_sub(1))
            .unwrap_or(0)
    }

    pub fn event_permanent(&self) -> Option<PermanentHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_permanent)
    }

    pub fn event_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_card)
    }

    pub fn event_card_name_contains(&self, needle: &str) -> bool {
        let Some(card) = self.event_card() else {
            return false;
        };
        let Some(data) = self.game.card_data_for_handle(card) else {
            return false;
        };
        data.card_name
            .to_lowercase()
            .contains(&needle.to_lowercase())
    }

    /// Case-insensitive substring scan against the triggering event card's
    /// PRINTED text (effect / inherited / security). Event-side analogue of the
    /// static `effect_text_contains`. G-DSL-EVENT-CARD-TEXT-CONTAINS.
    pub fn event_card_text_contains(&self, needle: &str) -> bool {
        let Some(card) = self.event_card() else {
            return false;
        };
        let Some(data) = self.game.card_data_for_handle(card) else {
            return false;
        };
        let needle = needle.to_lowercase();
        data.effect_text.to_lowercase().contains(&needle)
            || data.inherited_text.to_lowercase().contains(&needle)
            || data.security_text.to_lowercase().contains(&needle)
    }

    pub fn event_source_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_source_card)
    }

    pub fn event_host_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_host_card)
    }

    pub fn event_host_permanent(&self) -> Option<PermanentHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| {
                let handle = trigger.event_host_permanent?;
                live_event_permanent(self.game, handle, trigger.event_host_card)
            })
    }

    pub fn deleted_object_snapshot(
        &self,
    ) -> Option<&crate::trigger_context::DeletedObjectSnapshot> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
    }

    /// The battle WINNER carried by an `EndOfBattle` firing (board-wide
    /// battle-winner observer, G-DSL-BATTLE-WINNER-BOARDWIDE). `None` on a tie
    /// or a non-battle trigger.
    pub fn battle_winner(&self) -> Option<PermanentHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.battle_winner)
    }

    /// True when the battle winner is live and its top card carries
    /// `trait_name` (case-insensitive).
    pub fn battle_winner_has_trait(&self, trait_name: &str) -> bool {
        let Some(handle) = self.battle_winner() else {
            return false;
        };
        self.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.has_trait(trait_name, self.card_data()))
            .unwrap_or(false)
    }

    /// The player whose HAND lost cards in the current `OnDiscardHand` batch.
    /// G-ENGINE-ON-DISCARD-HAND.
    pub fn discard_hand_player(&self) -> Option<PlayerId> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.discard_hand_player)
    }

    /// The controller of the effect that caused the current `OnDiscardHand`.
    pub fn discard_cause_controller(&self) -> Option<PlayerId> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.discard_cause_controller)
    }

    /// True when the current `OnDiscardHand` event was caused by an effect the
    /// OBSERVER controls (ST16-14 Matt Ishida "one of YOUR effects").
    pub fn discard_caused_by_own_effect(&self) -> bool {
        self.discard_cause_controller()
            .map(|controller| controller == self.player())
            .unwrap_or(false)
    }

    /// True when the permanent carrying THIS effect was played by an effect
    /// (`PlaySource::ByEffect`), read at the OnPlay firing (BT25-080).
    pub fn played_by_effect(&self) -> bool {
        self.game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.effect_initiated)
            .unwrap_or(false)
    }

    /// Pre-removal effective DP of the deleted permanent (modifier-aware).
    /// `None` when no `deleted_object` snapshot is on the current trigger
    /// context or when the permanent had no DP value (e.g. Tamer).
    pub fn deleted_self_dp(&self) -> Option<i32> {
        self.deleted_object_snapshot()
            .and_then(|s| s.dp_just_before)
    }

    /// Pre-removal level of the deleted permanent.
    pub fn deleted_self_level(&self) -> Option<u8> {
        self.deleted_object_snapshot()
            .and_then(|s| s.level_just_before)
    }

    /// Pre-removal printed play cost of the deleted permanent's top card.
    pub fn deleted_self_cost(&self) -> Option<u16> {
        self.deleted_object_snapshot()
            .and_then(|s| s.cost_just_before)
    }

    /// Pre-removal top-card card names. Returns an empty slice when no
    /// `deleted_object` snapshot is on the current trigger context.
    pub fn deleted_self_names(&self) -> &[String] {
        self.deleted_object_snapshot()
            .map(|s| s.names_just_before.as_slice())
            .unwrap_or(&[])
    }

    /// Pre-removal top-card traits.
    pub fn deleted_self_traits(&self) -> &[String] {
        self.deleted_object_snapshot()
            .map(|s| s.traits_just_before.as_slice())
            .unwrap_or(&[])
    }

    /// Count of digi-source cards BELOW the top at deletion time
    /// (`card_sources.len() - 1`). Returns 0 when no snapshot is present.
    pub fn deleted_self_source_count(&self) -> usize {
        self.deleted_object_snapshot()
            .map(|s| s.source_count_just_before)
            .unwrap_or(0)
    }

    /// Pre-removal digivolution-card handles in stack order (bottom-most
    /// first), excluding the top card.
    pub fn deleted_self_digisources(&self) -> &[crate::card_source::CardHandle] {
        self.deleted_object_snapshot()
            .map(|s| s.digisources_just_before.as_slice())
            .unwrap_or(&[])
    }

    pub fn event_affected_player(&self) -> Option<PlayerId> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.affected_player)
    }

    pub fn event_source_player(&self) -> Option<PlayerId> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.source_player)
    }

    pub fn event_cause(&self) -> Option<crate::trigger_context::EventCause> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.cause)
    }

    pub fn option_last_field_state(&self) -> Option<crate::option_lifecycle::OptionFieldState> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.option_last_field_state)
    }

    pub fn event_source_effect(&self) -> Option<crate::trigger_context::EffectAttribution> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.source_effect)
    }

    pub fn event_selected_results(&self) -> &[crate::trigger_context::ResultBinding] {
        self.game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.selected_results.as_slice())
            .unwrap_or(&[])
    }

    pub fn event_moved_card_sets(&self) -> &[crate::trigger_context::MovedCardSet] {
        self.game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.moved_card_sets.as_slice())
            .unwrap_or(&[])
    }

    pub fn event_dna_origin(&self) -> Option<bool> {
        event_dna_origin(self.game)
    }

    pub fn attack_target_change(&self) -> Option<crate::trigger_context::AttackTargetChange> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.attack_target_change)
    }

    pub fn dna_origin(&self) -> bool {
        event_dna_origin(self.game).unwrap_or(false)
    }

    pub fn source_kind(&self) -> EffectSourceKind {
        self.source_kind
    }

    pub fn source_is_digimon(&self) -> bool {
        self.source_kind == EffectSourceKind::Digimon
    }

    /// Returns `true` if this effect's source card is a Tamer.
    ///
    /// Used by flood-gate discriminators like `CannotGainMemoryExceptFromTamers`
    /// that allow Tamer-sourced effects but block Digimon/Option-sourced ones.
    /// Matches DCGO's `ICardEffect.IsTamerEffect` property.
    pub fn source_is_tamer(&self) -> bool {
        self.source_kind == EffectSourceKind::Tamer
    }

    pub fn source_is_option(&self) -> bool {
        self.source_kind == EffectSourceKind::Option
    }

    // ─── Security-check sugar (§2.5g) ────────────────────────────────
    // Mirrors the helpers on `EffectContext`. Available to condition
    // closures so scripts can gate a `[Security]` effect on attacker traits.

    pub fn attacker(&self) -> Option<PermanentHandle> {
        self.game
            .security_resolution
            .as_ref()
            .and_then(|s| s.attacker)
    }

    pub fn attack_attacker(&self) -> Option<PermanentHandle> {
        self.game
            .pending_attack
            .as_ref()
            .map(|attack| attack.attacker)
    }

    pub fn attack_target(&self) -> Option<crate::AttackTarget> {
        self.game
            .pending_attack
            .as_ref()
            .map(|attack| attack.effective_target)
    }

    pub fn security_digimon(&self) -> Option<CardHandle> {
        self.game
            .security_resolution
            .as_ref()
            .map(|s| s.revealed_card)
    }

    pub fn turn_player_at_check(&self) -> Option<PlayerId> {
        self.game
            .security_resolution
            .as_ref()
            .map(|s| s.turn_player)
    }

    // ─── OnDeletion cause accessors (Phase B §B5) ───────────────────────

    /// The `ReplacementCause` of the deletion currently being observed by
    /// this `OnDeletion` (or `OnAnyDeletion`) effect. `None` outside such an
    /// observer body. Phase B §B5.
    ///
    /// Set only while the `OnDeletion` / `OnAnyDeletion` queue drain for a
    /// single `delete_permanent_with_cause` call is in flight; cleared on
    /// drain completion via panic-safe guard.
    ///
    /// Scapegoat-style predicates ("deleted by anything other than own
    /// effect") should read this directly rather than via
    /// `was_deleted_by_effect`, since they fire for `Battle` /
    /// `SecurityCheck` / `Cost` causes too.
    pub fn deletion_cause(&self) -> Option<crate::replacement::ReplacementCause> {
        observed_deletion_cause(self.game)
    }

    /// `true` when the current OnDeletion observer is firing because of an
    /// effect (own or opponent), as opposed to battle / security-check / cost.
    /// Convenience for "deleted by an effect" predicates, including
    /// Retaliation (cause == Battle, hence false here).
    pub fn was_deleted_by_effect(&self) -> bool {
        use crate::replacement::ReplacementCause;
        matches!(
            self.game.current_deletion_cause,
            Some(ReplacementCause::OwnEffect | ReplacementCause::OpponentEffect)
        )
    }

    /// `true` when the current OnDeletion observer is firing because of an
    /// opponent's effect specifically. Drives Mephistomon-style "when this is
    /// deleted by your opponent's effect" riders.
    pub fn was_deleted_by_opponent(&self) -> bool {
        matches!(
            self.game.current_deletion_cause,
            Some(crate::replacement::ReplacementCause::OpponentEffect)
        )
    }

    /// Identify the opposing combatant in the currently-resolving battle.
    ///
    /// Returns `Some(opponent_handle)` when `Game.pending_attack` is live
    /// AND the supplied `self_handle` matches one side of the battle:
    ///   - `self_handle == attacker` → returns the defender
    ///   - `self_handle == effective_target.as_digimon()` → returns the attacker
    ///   - otherwise (no pending battle, or self is not a combatant) → `None`
    ///
    /// Used by Retaliation (Phase E §E1) to identify the battle winner from
    /// inside an `OnDeletion` handler — the loser is mid-deletion (calling
    /// the handler) and the winner is the other side of the pending attack.
    /// Direct player attacks (`AttackTarget::Player`) return `None` because
    /// there is no opposing Digimon.
    pub fn battle_opponent_of(&self, self_handle: PermanentHandle) -> Option<PermanentHandle> {
        let pa = self.game.pending_attack.as_ref()?;
        let defender = match pa.effective_target {
            crate::AttackTarget::Digimon(h) => Some(h),
            crate::AttackTarget::Player(_) => None,
        }?;
        if self_handle == pa.attacker {
            Some(defender)
        } else if self_handle == defender {
            Some(pa.attacker)
        } else {
            None
        }
    }
}

fn live_event_permanent(
    game: &Game,
    handle: PermanentHandle,
    expected_card: Option<CardHandle>,
) -> Option<PermanentHandle> {
    let card = game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
        .map(|perm| perm.top_card().handle())?;
    match expected_card {
        Some(expected_card) if card != expected_card => None,
        _ => Some(handle),
    }
}

fn find_digixros_material_origin(game: &Game, card: CardHandle) -> Option<DigiXrosMaterialOrigin> {
    for player in 0..game.players.len() {
        let player_id = player as PlayerId;
        let player_state = game.player(player_id);
        if let Some(index) = player_state
            .hand
            .iter()
            .position(|candidate| candidate.handle() == card)
        {
            return Some(DigiXrosMaterialOrigin::Hand {
                player: player_id,
                index,
                card,
            });
        }
        if let Some(index) = player_state
            .trash
            .iter()
            .position(|candidate| candidate.handle() == card)
        {
            return Some(DigiXrosMaterialOrigin::Trash {
                player: player_id,
                index,
                card,
            });
        }
        for (permanent_index, permanent) in player_state.battle_area.iter().enumerate() {
            let permanent_handle = PermanentHandle {
                player: player_id,
                index: permanent_index as u8,
            };
            if permanent.top_card().handle() == card {
                return Some(DigiXrosMaterialOrigin::BattleArea {
                    permanent: permanent_handle,
                    card,
                });
            }
            if permanent.is_tamer(&game.card_data) {
                if let Some(source_index) = permanent
                    .card_sources
                    .iter()
                    .position(|candidate| candidate.handle() == card)
                {
                    return Some(DigiXrosMaterialOrigin::UnderTamer {
                        tamer: permanent_handle,
                        source_index,
                        card,
                    });
                }
            }
        }
    }
    None
}

fn event_dna_origin(game: &Game) -> Option<bool> {
    let has_trigger_context = game.current_trigger_context.is_some();
    let has_dna_scope = game.current_dna_origin.is_some();
    if !has_trigger_context && !has_dna_scope {
        return None;
    }

    let trigger_origin = game
        .current_trigger_context
        .as_ref()
        .map(|trigger| trigger.dna_origin)
        .unwrap_or(false);
    let scoped_origin = game.current_dna_origin.unwrap_or(false);
    Some(trigger_origin || scoped_origin)
}

/// Reverse of `From<ReplacementCause> for EventCause` for the deletion-cause
/// subset. Used to recover the `ReplacementCause` from a `DeletedObjectSnapshot`
/// after the live `current_deletion_cause` slot has been restored — e.g. when an
/// `[On Deletion]` bundle is deferred past the deleting effect's resolution
/// window (Q19 Part B). Non-deletion event causes map to `None`.
fn replacement_cause_from_event_cause(
    ec: crate::trigger_context::EventCause,
) -> Option<ReplacementCause> {
    use crate::trigger_context::EventCause;
    match ec {
        EventCause::BattleDeletion => Some(ReplacementCause::Battle),
        EventCause::OwnEffect => Some(ReplacementCause::OwnEffect),
        EventCause::OpponentEffect => Some(ReplacementCause::OpponentEffect),
        EventCause::SecurityRemoval => Some(ReplacementCause::SecurityCheck),
        EventCause::Cost => Some(ReplacementCause::Cost),
        EventCause::Overclock => Some(ReplacementCause::Overclock),
        _ => None,
    }
}

fn observed_deletion_cause(game: &Game) -> Option<ReplacementCause> {
    // Rule 25: `[On Deletion]` handlers read pre-removal state from the snapshot,
    // not live slots. When the bundle is deferred past the deleting effect's
    // later steps (Q19 Part B), the live `current_deletion_cause` /
    // `current_deletion_event_cause_override` slots have already been restored by
    // the batch's exit, so prefer the cause threaded into the installed trigger
    // context (`deleted_object.cause`), which survives the deferral. Battle and
    // other top-level deletions install the same snapshot during their handler,
    // so this is consistent with the live-slot reads they used before.
    if let Some(snap) = game
        .current_trigger_context
        .as_ref()
        .and_then(|t| t.deleted_object.as_ref())
    {
        if let Some(rc) = replacement_cause_from_event_cause(snap.cause) {
            return Some(rc);
        }
    }
    match game.current_deletion_event_cause_override {
        Some(crate::trigger_context::EventCause::Overclock) => Some(ReplacementCause::Overclock),
        _ => game.current_deletion_cause,
    }
}

/// The context passed to every effect's `process` closure.
/// For `condition` closures see `EffectReadContext`.
pub struct EffectContext<'a> {
    pub game: &'a mut Game,
    /// Card whose effect is being resolved.
    pub source_card: CardHandle,
    /// The permanent containing the source card, if applicable.
    pub source_permanent: Option<PermanentHandle>,
    /// Rules-facing classification of the effect source.
    pub source_kind: EffectSourceKind,
    /// Player who controls the source.
    pub player: PlayerId,
    /// Temporary override for `selecting_player` inside `as_selecting_player`
    /// scope methods. `None` at all times except during the body of an
    /// `EffectContextSelectorScope::select_*` call, where it is set to the
    /// desired selector and cleared again before the method returns.
    pub(super) override_selecting_player: Option<PlayerId>,
    /// Card whose cost is currently being inspected/resolved by a
    /// BeforePayCost hook. `None` outside cost-reducer resolution.
    pub cost_target_card: Option<CardHandle>,
    pub cost_target_from_hand: bool,
    /// True when the cost currently being resolved is a digivolve cost.
    /// `G-COST-REDUCTION-DIGIVOLVE-INTO`.
    pub cost_is_digivolve: bool,
    /// Set by a `pay_cost` step that aborts because the cost is UNPAYABLE (e.g.
    /// `trash_bottom_face_down_source_under_tamer` with no eligible Tamer). Such
    /// a step returns `RunOutcome::Synchronous` (a clean abort, not a park), which
    /// is otherwise indistinguishable from a cost that was genuinely paid
    /// synchronously. The generic `cost_reduction` `pay_cost_fn` lowering reads
    /// this flag so it does NOT credit the reduction for an unpaid cost.
    /// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    pub cost_unpayable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayCostStatus {
    Paid,
    Unpaid,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectRefireError {
    InvalidTiming(String),
}

impl<'a> EffectContext<'a> {
    pub fn new(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
    ) -> Self {
        let source_kind = infer_effect_source_kind(game, source_card, source_permanent);
        Self::new_with_source_kind(game, source_card, source_permanent, source_kind, player)
    }

    pub fn new_with_source_kind(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        source_kind: EffectSourceKind,
        player: PlayerId,
    ) -> Self {
        Self {
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
            override_selecting_player: None,
            cost_target_card: None,
            cost_target_from_hand: false,
            cost_is_digivolve: false,
            cost_unpayable: false,
        }
    }

    fn refire_effect_slot_available(&self, effect: &ReFireableEffect) -> bool {
        let Some(effects) = self
            .game
            .effects_for_card(&effect.card_id, effect.source_card)
        else {
            return false;
        };
        let Some(effect_body) = effects.get(effect.effect_id as usize) else {
            return false;
        };
        if effect_body.max_per_turn == 0 {
            return true;
        }
        let Some(perm) = self
            .game
            .players
            .get(effect.source.player as usize)
            .and_then(|p| p.battle_area.get(effect.source.index as usize))
        else {
            return false;
        };
        perm.activation_count(effect.source_card, effect.effect_id) < effect_body.max_per_turn
    }

    pub fn new_with_cost_target(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
        cost_target_card: CardHandle,
        cost_target_from_hand: bool,
    ) -> Self {
        let mut ctx = Self::new(game, source_card, source_permanent, player);
        ctx.cost_target_card = Some(cost_target_card);
        ctx.cost_target_from_hand = cost_target_from_hand;
        ctx
    }

    pub fn was_digixros(&self) -> bool {
        // Semantic firewall: a non-DigiXros cast-time assembly (BT15-102)
        // rides the same pending transaction but is NOT a DigiXros.
        self.game
            .pending_digixros_transaction()
            .is_some_and(|transaction| transaction.is_digixros)
    }

    pub fn digixros_count(&self) -> u8 {
        self.game
            .pending_digixros_transaction()
            .filter(|transaction| transaction.is_digixros)
            .map(|transaction| transaction.digixros_count)
            .unwrap_or(0)
    }

    pub fn pending_digixros_play_card(&self) -> Option<CardHandle> {
        self.game
            .pending_digixros_transaction()
            .map(|transaction| transaction.played_card)
    }

    /// Construct an `EffectContext` with an explicit selecting-player
    /// override. Used by `AsSelectingPlayer` lowering and by selection
    /// callbacks that must preserve the `(controller, override)` pair across
    /// the parked-callback boundary (Phase 2f3).
    ///
    /// `controller` becomes `self.player` (the original effect controller);
    /// `override_selecting_player` is preserved as-is so a nested `select_*`
    /// inside the callback (or the dsl_outer_tail) routes to the override
    /// rather than back to the controller.
    pub fn new_with_override(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        controller: PlayerId,
        override_selecting_player: Option<PlayerId>,
    ) -> Self {
        let mut ctx = Self::new(game, source_card, source_permanent, controller);
        ctx.override_selecting_player = override_selecting_player;
        ctx
    }

    pub fn new_with_source_kind_and_override(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        source_kind: EffectSourceKind,
        controller: PlayerId,
        override_selecting_player: Option<PlayerId>,
    ) -> Self {
        let mut ctx = Self::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            source_kind,
            controller,
        );
        ctx.override_selecting_player = override_selecting_player;
        ctx
    }

    /// Read the current selecting-player override, if any. The override is
    /// installed by `AsSelectingPlayer` lowering (Phase 2f3) and persists
    /// across selection-callback boundaries via `new_with_override`.
    pub fn override_selecting_player(&self) -> Option<PlayerId> {
        self.override_selecting_player
    }

    /// Install (or clear) the selecting-player override. Used by
    /// `AsSelectingPlayer` step lowering (Phase 2f3) to scope the override
    /// to the body and restore the previous value on synchronous
    /// completion, and by `drain_dsl_outer_tail` to clear the override
    /// before running sibling steps that follow a body-parked
    /// `AsSelectingPlayer`. Crate-private — the field itself is
    /// intentionally not widened, so all writes go through this setter.
    pub(crate) fn set_override_selecting_player(&mut self, p: Option<PlayerId>) {
        self.override_selecting_player = p;
    }

    // ─── Delayed scheduling (Phase 2f4 Task 1) ─────────────────────────

    // ─── Read-only queries ────────────────────────────────────────────

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn rules(&self) -> &Rules {
        &self.game.rules
    }

    pub fn card_data(&self) -> &[CardData] {
        &self.game.card_data
    }

    pub fn player(&self, id: PlayerId) -> &Player {
        self.game.player(id)
    }

    pub fn my_player(&self) -> &Player {
        self.game.player(self.player)
    }

    /// First clockwise opponent (sugar for `opponents()[0]`).
    pub fn opponent_id(&self) -> PlayerId {
        self.game.next_clockwise(self.player)
    }

    pub fn opponent(&self) -> &Player {
        self.game.player(self.opponent_id())
    }

    pub fn opponents(&self) -> Vec<PlayerId> {
        self.game.opponents(self.player)
    }

    pub fn battle_area(&self, id: PlayerId) -> &[Permanent] {
        &self.game.player(id).battle_area
    }

    pub fn hand(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).hand
    }

    pub fn trash(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).trash
    }

    pub fn security_count(&self, id: PlayerId) -> usize {
        self.game.player(id).security.len()
    }

    pub fn source_permanent(&self) -> Option<&Permanent> {
        let h = self.source_permanent?;
        let player = self.game.player(h.player);
        if h.index == crate::action::space::BREEDING_TARGET as u8 {
            return player.breeding_area.as_ref();
        }
        player.battle_area.get(h.index as usize)
    }

    pub fn cost_target_card_id(&self) -> Option<&str> {
        self.cost_target_card
            .and_then(|card| self.game.card_data_for_handle(card))
            .map(|data| data.card_id.as_str())
    }

    pub fn cost_target_has_trait(&self, needle: &str) -> bool {
        self.cost_target_card
            .and_then(|card| self.game.card_data_for_handle(card))
            .is_some_and(|data| data.traits.iter().any(|trait_name| trait_name == needle))
    }

    pub fn cost_target_from_hand(&self) -> bool {
        self.cost_target_from_hand
    }

    pub fn cost_reduction_source(&self) -> Option<PermanentHandle> {
        self.source_permanent
    }

    pub fn source_stack_source_count(&self) -> usize {
        self.source_permanent()
            .map(|perm| perm.card_sources.len().saturating_sub(1))
            .unwrap_or(0)
    }

    pub fn event_permanent(&self) -> Option<PermanentHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_permanent)
    }

    pub fn event_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_card)
    }

    pub fn event_card_name_contains(&self, needle: &str) -> bool {
        self.as_read().event_card_name_contains(needle)
    }

    pub fn event_card_text_contains(&self, needle: &str) -> bool {
        self.as_read().event_card_text_contains(needle)
    }

    pub fn event_source_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_source_card)
    }

    pub fn event_host_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_host_card)
    }

    pub fn event_host_permanent(&self) -> Option<PermanentHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| {
                let handle = trigger.event_host_permanent?;
                live_event_permanent(self.game, handle, trigger.event_host_card)
            })
    }

    pub fn deleted_object_snapshot(
        &self,
    ) -> Option<&crate::trigger_context::DeletedObjectSnapshot> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
    }

    /// See [`EffectReadContext::battle_winner`]. G-DSL-BATTLE-WINNER-BOARDWIDE.
    pub fn battle_winner(&self) -> Option<PermanentHandle> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.battle_winner)
    }

    /// See [`EffectReadContext::battle_winner_has_trait`].
    pub fn battle_winner_has_trait(&self, trait_name: &str) -> bool {
        let Some(handle) = self.battle_winner() else {
            return false;
        };
        self.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.has_trait(trait_name, &self.game.card_data))
            .unwrap_or(false)
    }

    /// See [`EffectReadContext::discard_hand_player`]. G-ENGINE-ON-DISCARD-HAND.
    pub fn discard_hand_player(&self) -> Option<PlayerId> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.discard_hand_player)
    }

    /// See [`EffectReadContext::discard_caused_by_own_effect`].
    pub fn discard_caused_by_own_effect(&self) -> bool {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.discard_cause_controller)
            .map(|controller| controller == self.player)
            .unwrap_or(false)
    }

    /// See [`EffectReadContext::played_by_effect`].
    pub fn played_by_effect(&self) -> bool {
        self.game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.effect_initiated)
            .unwrap_or(false)
    }

    pub fn deleted_self_dp(&self) -> Option<i32> {
        self.as_read().deleted_self_dp()
    }

    pub fn deleted_self_level(&self) -> Option<u8> {
        self.as_read().deleted_self_level()
    }

    pub fn deleted_self_cost(&self) -> Option<u16> {
        self.as_read().deleted_self_cost()
    }

    pub fn deleted_self_names(&self) -> &[String] {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
            .map(|s| s.names_just_before.as_slice())
            .unwrap_or(&[])
    }

    pub fn deleted_self_traits(&self) -> &[String] {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
            .map(|s| s.traits_just_before.as_slice())
            .unwrap_or(&[])
    }

    pub fn deleted_self_source_count(&self) -> usize {
        self.as_read().deleted_self_source_count()
    }

    pub fn deleted_self_digisources(&self) -> &[crate::card_source::CardHandle] {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
            .map(|s| s.digisources_just_before.as_slice())
            .unwrap_or(&[])
    }

    pub fn event_affected_player(&self) -> Option<PlayerId> {
        self.as_read().event_affected_player()
    }

    pub fn event_source_player(&self) -> Option<PlayerId> {
        self.as_read().event_source_player()
    }

    pub fn event_cause(&self) -> Option<crate::trigger_context::EventCause> {
        self.as_read().event_cause()
    }

    pub fn option_last_field_state(&self) -> Option<crate::option_lifecycle::OptionFieldState> {
        self.as_read().option_last_field_state()
    }

    pub fn event_source_effect(&self) -> Option<crate::trigger_context::EffectAttribution> {
        self.as_read().event_source_effect()
    }

    pub fn event_selected_results(&self) -> &[crate::trigger_context::ResultBinding] {
        self.game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.selected_results.as_slice())
            .unwrap_or(&[])
    }

    pub fn event_moved_card_sets(&self) -> &[crate::trigger_context::MovedCardSet] {
        self.game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.moved_card_sets.as_slice())
            .unwrap_or(&[])
    }

    pub fn event_dna_origin(&self) -> Option<bool> {
        self.as_read().event_dna_origin()
    }

    pub fn attack_target_change(&self) -> Option<crate::trigger_context::AttackTargetChange> {
        self.game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.attack_target_change)
    }

    pub fn source_kind(&self) -> EffectSourceKind {
        self.source_kind
    }

    pub fn source_is_digimon(&self) -> bool {
        self.source_kind == EffectSourceKind::Digimon
    }

    /// Returns `true` if this effect's source card is a Tamer.
    ///
    /// Used by flood-gate discriminators like `CannotGainMemoryExceptFromTamers`
    /// that allow Tamer-sourced effects but block Digimon/Option-sourced ones.
    /// Matches DCGO's `ICardEffect.IsTamerEffect` property.
    pub fn source_is_tamer(&self) -> bool {
        self.source_kind == EffectSourceKind::Tamer
    }

    pub fn source_is_option(&self) -> bool {
        self.source_kind == EffectSourceKind::Option
    }

    // ─── Security-check sugar (§2.5g) ────────────────────────────────
    // Readers into `game.security_resolution`. Meaningful only while a
    // security-attack resolution is in flight (i.e. inside a `[Security]`,
    // `OnSecurityCheck`, or `OnLoseSecurity` process closure).

    /// The attacker whose attack triggered the current security check, if
    /// any. `None` for non-combat security reveals.
    pub fn attacker(&self) -> Option<PermanentHandle> {
        self.game
            .security_resolution
            .as_ref()
            .and_then(|s| s.attacker)
    }

    pub fn attack_attacker(&self) -> Option<PermanentHandle> {
        self.game
            .pending_attack
            .as_ref()
            .map(|attack| attack.attacker)
    }

    pub fn attack_target(&self) -> Option<crate::AttackTarget> {
        self.game
            .pending_attack
            .as_ref()
            .map(|attack| attack.effective_target)
    }

    /// The handle of the security card currently being resolved (the card
    /// that was popped from the defender's security stack). `None` outside a
    /// security-resolution context.
    pub fn security_digimon(&self) -> Option<CardHandle> {
        self.game
            .security_resolution
            .as_ref()
            .map(|s| s.revealed_card)
    }

    /// Turn player at the moment the security check started. Stable across
    /// the entire resolution even if an effect toggles turn state mid-flight.
    pub fn turn_player_at_check(&self) -> Option<PlayerId> {
        self.game
            .security_resolution
            .as_ref()
            .map(|s| s.turn_player)
    }

    // ─── OnDeletion cause accessors (Phase B §B5) ───────────────────────

    /// See `EffectReadContext::deletion_cause`.
    ///
    /// Set only while the `OnDeletion` / `OnAnyDeletion` queue drain for a
    /// single `delete_permanent_with_cause` call is in flight; cleared on
    /// drain completion via panic-safe guard. Scapegoat-style predicates
    /// ("deleted by anything other than own effect") should read this
    /// directly rather than via `was_deleted_by_effect`, since they fire for
    /// `Battle` / `SecurityCheck` / `Cost` causes too.
    pub fn deletion_cause(&self) -> Option<crate::replacement::ReplacementCause> {
        observed_deletion_cause(self.game)
    }

    /// See `EffectReadContext::was_deleted_by_effect`. Convenience for
    /// "deleted by an effect" predicates (e.g. Retaliation, where
    /// cause == Battle, returns false).
    pub fn was_deleted_by_effect(&self) -> bool {
        use crate::replacement::ReplacementCause;
        matches!(
            self.game.current_deletion_cause,
            Some(ReplacementCause::OwnEffect | ReplacementCause::OpponentEffect)
        )
    }

    /// See `EffectReadContext::was_deleted_by_opponent`.
    pub fn was_deleted_by_opponent(&self) -> bool {
        matches!(
            self.game.current_deletion_cause,
            Some(crate::replacement::ReplacementCause::OpponentEffect)
        )
    }

    /// See [`EffectReadContext::battle_opponent_of`].
    pub fn battle_opponent_of(&self, self_handle: PermanentHandle) -> Option<PermanentHandle> {
        let pa = self.game.pending_attack.as_ref()?;
        let defender = match pa.effective_target {
            crate::AttackTarget::Digimon(h) => Some(h),
            crate::AttackTarget::Player(_) => None,
        }?;
        if self_handle == pa.attacker {
            Some(defender)
        } else if self_handle == defender {
            Some(pa.attacker)
        } else {
            None
        }
    }

    // ─── Replacement-process outcome-setters (Phase C §4.2) ──────────────

    pub fn delay_source_card_in_trash(&self, player: PlayerId, source_card: CardHandle) -> bool {
        self.find_battle_permanent_containing_card(player, source_card)
            .is_none()
            && self
                .game
                .player(player)
                .trash
                .iter()
                .any(|card| card.handle() == source_card)
    }

    /// Resolve a `PermanentHandle` to its top card handle. Returns `None`
    /// when the slot is missing OR has empty `card_sources` (zombie).
    /// All callers wrap in `Option::and_then` / `let Some(…) else`, so the
    /// zombie case is correctly treated as "no top card." Mirrors the same
    /// defensive form used by `Game::top_card_handle` in `effect_queue.rs`.
    pub fn permanent_top_card_handle(&self, handle: PermanentHandle) -> Option<CardHandle> {
        if crate::digixros::is_limbo_index(handle.index) {
            let pos = (handle.index - crate::digixros::LIMBO_INDEX_BASE) as usize;
            return self
                .game
                .digixros_leaving_limbo
                .get(pos)
                .and_then(|(_, _, perm)| perm.card_sources.last().map(|c| c.handle()));
        }
        self.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .and_then(|permanent| permanent.card_sources.last().map(|c| c.handle()))
    }

    pub fn find_battle_permanent_containing_card(
        &self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.game
            .player(player)
            .battle_area
            .iter()
            .position(|permanent| {
                permanent
                    .card_sources
                    .iter()
                    .chain(permanent.linked_cards.iter())
                    .any(|source| source.handle() == card)
            })
            .map(|index| PermanentHandle {
                player,
                index: index as u8,
            })
            // Fallback: the card may be parked in the DigiXros leaving/limbo slot
            // (its `WhenWouldLeaveBattleArea` window parked a reward). Resolve to
            // the limbo-encoded handle so a DNA-evo reward can still extract it.
            .or_else(|| self.game.find_limbo_permanent_containing_card(player, card))
    }

    /// Reborrow this mut context as a read-only context — for condition
    /// closures, which take `&EffectReadContext`.
    pub fn as_read(&self) -> EffectReadContext<'_> {
        EffectReadContext {
            game: self.game,
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            source_kind: self.source_kind,
            player: self.player,
            replacement_cause: None,
            replacement_source_controller: None,
            replacement_subject_controller: None,
            cost_target_card: self.cost_target_card,
            cost_target_from_hand: self.cost_target_from_hand,
            cost_is_digivolve: self.cost_is_digivolve,
            cost_target_permanents: Vec::new(),
        }
    }

    // ─── Memory mutations ─────────────────────────────────────────────

    // ─── Card draw / trash ────────────────────────────────────────────

    // ─── Field mutations ──────────────────────────────────────────────

    pub fn can_affect_permanent(&self, target: PermanentHandle) -> bool {
        !self.game.progress_excludes(target, Some(self.player))
            && !self
                .game
                .permanent_is_unaffected_by_effect(target, self.player, self.source_kind)
    }

    /// Snapshot of the current reveal pool. Scripts inspect this to decide
    /// follow-up moves.
    pub fn revealed(&self) -> &[crate::card_source::CardSource] {
        &self.game.revealed_cards
    }

    /// Sibling-fix helper: run `Game::soft_remove_if_emptied(carrier)` and,
    /// if it removed the slot, shift `other_handle` for the same-player
    /// index shift via `Game::shift_handle_after_soft_remove`. Returns the
    /// possibly-shifted handle. The carrier handle is irrelevant after a
    /// successful soft-remove, so callers that don't need it can discard.
    fn shift_handle_after_soft_remove_check(
        game: &mut crate::game::Game,
        carrier: PermanentHandle,
        other: PermanentHandle,
    ) -> PermanentHandle {
        if game.soft_remove_if_emptied(carrier) {
            crate::game::Game::shift_handle_after_soft_remove(carrier, other)
        } else {
            other
        }
    }

    fn own_tamer_target(&self, tamer: PermanentHandle) -> bool {
        if tamer.player != self.player {
            return false;
        }
        self.game
            .player(self.player)
            .battle_area
            .get(tamer.index as usize)
            .is_some_and(|perm| perm.is_tamer(&self.game.card_data))
    }

    pub fn resolve_provenance_token(
        &self,
        token: crate::trigger_context::ProvenanceToken,
    ) -> Option<crate::trigger_context::EventSubject> {
        self.game.resolve_provenance_token(token)
    }

    // ─── Modifier registration ────────────────────────────────────────

    // ─── Breeding-area mutations ──────────────────────────────────────

    // ─── Combat mutations (Phase 9 Task 2) ────────────────────────────
}

/// Can the `{anchor, partner}` battle-area pair legally DNA-digivolve into the
/// hand card at `hand_idx` (one of its printed DNA requirements satisfied by the
/// two materials)? Used by `may_dna_digivolve_now`'s faithful (requirement-checked)
/// path to mirror DCGO `CardSource.CanJogressFromTargetPermanent`.
pub(crate) fn dna_pair_can_reach_hand_card(
    game: &Game,
    controller: PlayerId,
    anchor: PermanentHandle,
    partner: PermanentHandle,
    hand_idx: usize,
) -> bool {
    dna_pair_cost_for_hand_card(game, controller, anchor, partner, hand_idx).is_some()
}

/// The printed DNA-digivolve memory cost the `{anchor, partner}` pair would pay to
/// DNA-digivolve into the hand card at `hand_idx`, or `None` if the pair does not
/// satisfy any of that card's DNA requirements. Mirrors DCGO `condition.cost`.
pub(crate) fn dna_pair_cost_for_hand_card(
    game: &Game,
    controller: PlayerId,
    anchor: PermanentHandle,
    partner: PermanentHandle,
    hand_idx: usize,
) -> Option<i32> {
    let hand_card = game.player(controller).hand.get(hand_idx)?;
    let meta = game.card_data.get(hand_card.data_index)?;
    let anchor_perm = game
        .player(anchor.player)
        .battle_area
        .get(anchor.index as usize)?;
    let partner_perm = game
        .player(partner.player)
        .battle_area
        .get(partner.index as usize)?;
    crate::dna_digivolve::matching_dna_cost(meta, anchor_perm, partner_perm, &game.card_data)
        .map(|c| c.memory_cost as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_data::CardData;
    use crate::rules::Rules;
    use std::collections::HashMap;

    fn min_db() -> HashMap<String, CardData> {
        let json = r#"{
            "BT1-001": {
                "card_id": "BT1-001", "card_name_eng": "Koromon",
                "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    #[test]
    fn memory_mutations() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let controller = game.turn_player();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, controller);
        ctx.set_memory(0);
        ctx.gain_memory(3);
        assert_eq!(ctx.memory(), 3);
        ctx.lose_memory(2);
        assert_eq!(ctx.memory(), 1);
    }

    /// "Lose X memory" moves the marker toward the LOSING player's
    /// opponent's side (general_rule.pdf p.7: "'Lose X memory' means moving
    /// the memory marker X spaces toward your opponent's side"; DCGO scripts
    /// run `card.Owner.AddMemory(-n)`; Python `Player.lose_memory` delegates
    /// to `add_memory(-amount)` with the same turn-player seesaw branch).
    /// A non-turn-player controller losing memory therefore moves the
    /// seesaw TOWARD the turn player — it must not raw-subtract from the
    /// turn player's side.
    #[test]
    fn lose_memory_is_controller_relative() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let non_turn_player = 1 - game.turn_player();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, non_turn_player);
        ctx.set_memory(0);
        ctx.lose_memory(2);
        assert_eq!(
            ctx.memory(),
            2,
            "non-turn player losing 2 memory moves the marker 2 toward the turn player"
        );
    }

    #[test]
    fn play_token_unknown_name_returns_none() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
        assert!(ctx.play_token(0, "no-such-token-lol").is_none());
    }

    // ── may_dna_digivolve_now faithful-path gating (G-FIX-BT12-EOT-DNA-PAYCOST) ──
    //
    // The EoT DNA-digivolve effects (BT12-021 Veemon / BT12-047 Wormmon) are a
    // NORMAL DNA digivolve: DCGO `CanJogressFromTargetPermanent(.., PayCost:true)`
    // requires the target's printed DNA requirements be met by the {anchor,
    // partner} pair AND charges the target's printed DNA cost. These tests pin
    // the gating/cost helpers that `may_dna_digivolve_now` now uses when
    // `ignore_requirements: false` — proving the pair must be DNA-legal and the
    // printed DNA cost (not 0) is what gets paid.
    #[test]
    fn dna_pair_gating_requires_a_legal_dna_route_and_returns_printed_cost() {
        use crate::card_data::{DnaCost, DnaRequirement};
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::{CardColor, CardKind};

        fn mat(id: &str, color: CardColor) -> CardData {
            let mut c = make_test_card(id, id);
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.colors = vec![color];
            c.dp = Some(4000);
            c
        }
        fn req(color: CardColor, level: u8) -> DnaRequirement {
            DnaRequirement {
                level,
                card_colors: vec![color],
                name_contains: String::new(),
                text_contains: String::new(),
            }
        }
        // RESULT digivolves via DNA from {Blue Lv.4 + Green Lv.4} for cost 2.
        let mut result = make_test_card("RESULT-DNA", "ResultDna");
        result.card_kind = CardKind::Digimon;
        result.level = Some(5);
        result.dna_costs = vec![DnaCost {
            requirement1: req(CardColor::Blue, 4),
            requirement2: req(CardColor::Green, 4),
            memory_cost: 2,
        }];

        let mut runner = DebugRunner::builder()
            .add_card(mat("MAT-B", CardColor::Blue))
            .add_card(mat("MAT-G", CardColor::Green))
            .add_card(mat("MAT-B2", CardColor::Blue))
            .add_card(result)
            .hand(0, &["RESULT-DNA"])
            .memory(10)
            .start();
        let blue = runner.place_on_field(0, "MAT-B", Some(0));
        let green = runner.place_on_field(0, "MAT-G", Some(0));
        let blue2 = runner.place_on_field(0, "MAT-B2", Some(0));

        // Legal pair (Blue + Green) → reachable, and the charged cost is the
        // target's PRINTED DNA cost (2), NOT the old free-of-charge 0.
        assert!(
            dna_pair_can_reach_hand_card(&runner.game, 0, blue, green, 0),
            "Blue+Green must satisfy RESULT's DNA requirement"
        );
        assert_eq!(
            dna_pair_cost_for_hand_card(&runner.game, 0, blue, green, 0),
            Some(2),
            "the faithful path pays RESULT's printed DNA cost (2), not 0"
        );

        // Illegal pair (Blue + Blue) → no valid DNA route; the EoT effect must
        // NOT offer RESULT as a target (the over-permissive bug this fixes).
        assert!(
            !dna_pair_can_reach_hand_card(&runner.game, 0, blue, blue2, 0),
            "Blue+Blue does NOT satisfy RESULT's Blue+Green DNA requirement — must be rejected"
        );
        assert_eq!(
            dna_pair_cost_for_hand_card(&runner.game, 0, blue, blue2, 0),
            None,
            "no cost for an illegal DNA pair"
        );
    }

    /// G-PLAY-TOKEN-FLOODGATE: a Digimon Token is a Digimon, so an effect that
    /// plays one must be blocked while the controller carries
    /// `CannotPlayDigimonByEffect` (BT9-033 Pillomon) — matching DCGO's
    /// `CanPlayAsNewPermanent` → `CanNotPutFieldClass(IsDigimon)` gate.
    #[test]
    fn play_token_blocked_by_cannot_play_digimon_by_effect() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        game.modifiers.add_player_modifier(
            0,
            crate::modifiers::PlayerModifierEntry::simple(
                crate::enums::ModifierType::CannotPlayDigimonByEffect,
                0,
                crate::enums::Expiry::Permanent,
                None,
                0,
            ),
        );
        let before = game.players[0].battle_area.len();
        let played = {
            let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
            ctx.play_token(0, "familiar")
        };
        assert!(
            played.is_none(),
            "token play must be blocked under CannotPlayDigimonByEffect"
        );
        assert_eq!(
            game.players[0].battle_area.len(),
            before,
            "no token permanent is created when the floodgate is installed"
        );
    }

    /// No-op control: without the floodgate, the same token play succeeds. Pins
    /// that the gate (not some unrelated failure) is what blocks above.
    #[test]
    fn play_token_allowed_without_floodgate() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let before = game.players[0].battle_area.len();
        let played = {
            let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
            ctx.play_token(0, "familiar")
        };
        assert!(
            played.is_some(),
            "token spawns normally when no floodgate is installed"
        );
        assert_eq!(
            game.players[0].battle_area.len(),
            before + 1,
            "one Familiar Token permanent is created"
        );
    }
}
