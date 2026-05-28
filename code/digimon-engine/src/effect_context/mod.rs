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

mod selections;

pub use selections::{
    CountCappedZone, DistinctByMode, EffectContextSelectorScope, RevealBucketSelection,
};

/// Material-zone carrier resolver — branches battle-area vs. breeding-area
/// (`BREEDING_TARGET` sentinel) carriers. Used by the DSL `select_material*`
/// step lowering so its filter/bind closures read the same stack the engine
/// selection helper does.
pub(crate) use selections::material_carrier_permanent;

pub use crate::selection::{BreedingPermanentSelectionRef, SourceSelectionRef};

use crate::action::mask::effect_attack_target_action_ids;
use crate::action::space::{decode_attack, encode_attack, SECURITY_TARGET};
use crate::card_data::CardData;
use crate::card_source::CardHandle;
use crate::combat::{AttackError, AttackInitiator, AttackOpen, AttackResult, TargetConstraint};
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

fn observed_deletion_cause(game: &Game) -> Option<ReplacementCause> {
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
        }
    }

    pub fn refire_effect_from_permanent(
        &mut self,
        source: PermanentHandle,
        timing_key: &str,
        optional: bool,
    ) -> Result<(), EffectRefireError> {
        let Some(timing_filter) = TimingFilter::from_timing_key(timing_key) else {
            return Err(EffectRefireError::InvalidTiming(timing_key.to_string()));
        };
        let _ =
            self.refire_target_effect_inner(source, timing_filter, self.player, false, optional);
        Ok(())
    }

    /// Refire one of `target`'s registered `[On Play]` or `[When Digivolving]`
    /// effects without treating the target as newly played or digivolved.
    ///
    /// Carrier semantics: the refired body sees `source_permanent` as
    /// `target`, so "this Digimon" reads the target permanent. Source
    /// attribution remains this context's `source_card`, so Homeros-style
    /// "this card's effect" predicates read the refire grantor.
    ///
    /// Once-per-turn accounting uses the target effect's normal slot unless
    /// `bypass_once_per_turn` is true.
    pub fn refire_target_effect(
        &mut self,
        target: PermanentHandle,
        timing_filter: TimingFilter,
        selecting_player: PlayerId,
        bypass_once_per_turn: bool,
    ) -> bool {
        self.refire_target_effect_inner(
            target,
            timing_filter,
            selecting_player,
            bypass_once_per_turn,
            false,
        )
    }

    fn refire_target_effect_inner(
        &mut self,
        target: PermanentHandle,
        timing_filter: TimingFilter,
        selecting_player: PlayerId,
        bypass_once_per_turn: bool,
        optional: bool,
    ) -> bool {
        let mut effects: Vec<ReFireableEffect> = timing_filter
            .timing_keys()
            .iter()
            .flat_map(|timing_key| enumerate_refireable_effects(self.game, target, timing_key))
            .filter(|effect| bypass_once_per_turn || self.refire_effect_slot_available(effect))
            .collect();
        for effect in &mut effects {
            effect.attribution_source_card = Some(self.source_card);
            effect.attribution_source_kind = Some(self.source_kind);
            effect.bypass_once_per_turn = bypass_once_per_turn;
            effect.controller = self.player;
        }
        match effects.as_slice() {
            [] => false,
            [effect] if !optional => {
                self.game.run_refired_effect(effect.clone());
                true
            }
            _ => {
                self.game
                    .install_refire_effect_selection(selecting_player, effects, optional);
                true
            }
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

    /// Schedule a one-shot delayed effect to fire at a future timing
    /// boundary. Used by `CompiledStep::ScheduleDelayed` lowering (Phase 2f4
    /// Task 3) for card text like "at the end of your next turn, do X".
    ///
    /// The effect's `body`, `captured_bindings`, source card, and source
    /// permanent are stored on `Game::scheduled_effects`. When
    /// `scheduled_effects::fire_scheduled_for_timing(game, when)` drains the
    /// queue, a fresh `EffectContext` is constructed against the captured
    /// `(controller, source_card, source_permanent)` and the body runs
    /// through `run_steps` with the captured bindings replayed.
    pub fn schedule_delayed(
        &mut self,
        when: EffectTiming,
        body: Vec<CompiledStep>,
        captured_bindings: Bindings,
    ) {
        self.schedule_delayed_with_runtime(when, body, captured_bindings, StepRuntime::default());
    }

    pub fn schedule_delayed_with_runtime(
        &mut self,
        when: EffectTiming,
        body: Vec<CompiledStep>,
        captured_bindings: Bindings,
        runtime: StepRuntime,
    ) {
        let entry = ScheduledEffect {
            when,
            body,
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            source_kind: self.source_kind,
            controller: self.player,
            captured_bindings,
            scheduled_at_turn: self.game.turn_count,
            runtime,
        };
        self.game.scheduled_effects.push(entry);
    }

    /// PUPPETS-G003 — schedule a deletion of `permanent` at the end of the
    /// current turn, keyed to the permanent's *stable identity* rather than
    /// its (shifting) battle-area index.
    ///
    /// Used by effects whose text says "At turn end, delete the Digimon this
    /// effect played" (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki). Pass
    /// the `PermanentHandle` returned by a free-play step (`play_from_hand_free`
    /// / `play_union_bound_free` / etc.); this captures the top card's
    /// `ProvenanceToken` *now*, while the handle is still valid.
    ///
    /// At turn end `fire_scheduled_provenance_deletions` resolves the token:
    /// if the played permanent is still on the battle area it is deleted (as
    /// the controller's own effect); if it already left, the entry is a
    /// silent no-op. A handle that does not currently point at a live
    /// permanent is ignored (nothing to schedule).
    pub fn schedule_delete_at_end_of_turn(&mut self, permanent: PermanentHandle) {
        self.schedule_provenance_deletion(permanent, false);
    }

    /// PUPPETS-G016 — schedule a deletion of `permanent` at the end of the
    /// **opponent's** turn, keyed to the permanent's *stable identity*.
    ///
    /// Used by P-165 ShoeShoemon ("At the end of your opponent's turn, delete
    /// that token"). Analogous to `schedule_delete_at_end_of_turn` but the
    /// deletion fires from `rotate_turn_player` rather than
    /// `fire_end_of_your_turn`.
    pub fn schedule_delete_at_end_of_opponents_turn(&mut self, permanent: PermanentHandle) {
        self.schedule_provenance_deletion(permanent, true);
    }

    fn schedule_provenance_deletion(&mut self, permanent: PermanentHandle, opponents_turn: bool) {
        let Some(top) = self
            .game
            .player(permanent.player)
            .battle_area
            .get(permanent.index as usize)
            .map(|perm| perm.top_card().handle())
        else {
            return;
        };
        let token = self.game.provenance_token_for_card(top);
        let entry = crate::scheduled_effects::ScheduledProvenanceDeletion {
            token,
            controller: self.player,
        };
        if opponents_turn {
            self.game.scheduled_provenance_deletions_opp.push(entry);
        } else {
            self.game.scheduled_provenance_deletions.push(entry);
        }
    }

    pub fn place_self_as_delay_option_permanent(&mut self) {
        let source_card = if let Some(source_perm) = self.source_permanent {
            if !matches!(
                self.game.card_kind_for_handle(self.source_card),
                Some(CardKind::Option)
            ) {
                return;
            }
            let Some(source_card) =
                self.remove_source_card_from_permanent(source_perm, self.source_card)
            else {
                return;
            };
            source_card
        } else {
            if let Some(pending) = self.game.pending_security.take() {
                if pending.played
                    || pending.card.handle() != self.source_card
                    || pending.card.card_kind(&self.game.card_data) != CardKind::Option
                {
                    self.game.pending_security = Some(pending);
                    return;
                }
                pending.card
            } else {
                let Some(source_card) = self.remove_source_option_from_controller_zones() else {
                    return;
                };
                source_card
            }
        };

        // The physical Option moves to its card owner/controller's battle area,
        // matching normal Delay placement from hand/trash.
        let owner = source_card.owner;
        let placed_card = source_card.handle();
        let card_id = source_card.card_id(&self.game.card_data).to_string();
        let trigger = self
            .game
            .effects_for_card(&card_id, placed_card)
            .unwrap_or_default()
            .iter()
            .find_map(|effect| effect.delay_trigger)
            .unwrap_or(DelayTrigger::EndOfYourNextTurn);
        let mut permanent = Permanent::new(source_card, self.game.turn_count);
        permanent.option_state = crate::permanent::OptionState::Delayed {
            owner,
            trash_on_turn: self.game.compute_delay_trash_turn(owner, trigger),
            trigger,
            placed_on_turn: self.game.turn_count,
        };
        self.game.player_mut(owner).battle_area.push(permanent);

        let handle = PermanentHandle {
            player: owner,
            index: (self.game.player(owner).battle_area.len() - 1) as u8,
        };
        self.game.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            crate::selection::TriggerSource::OptionPlaced {
                player: owner,
                permanent: Some(handle),
                linked_host: None,
                card: placed_card,
            },
        );
        self.game.drain_effect_queue();
    }

    fn remove_source_option_from_controller_zones(
        &mut self,
    ) -> Option<crate::card_source::CardSource> {
        if !matches!(
            self.game.card_kind_for_handle(self.source_card),
            Some(CardKind::Option)
        ) {
            return None;
        }

        if let Some(pos) = self
            .game
            .player(self.player)
            .hand
            .iter()
            .position(|card| card.handle() == self.source_card)
        {
            return Some(self.game.player_mut(self.player).hand.remove(pos));
        }

        let pos = self
            .game
            .player(self.player)
            .trash
            .iter()
            .position(|card| card.handle() == self.source_card)?;
        Some(self.game.player_mut(self.player).trash.remove(pos))
    }

    fn remove_source_card_from_permanent(
        &mut self,
        source_perm: PermanentHandle,
        source_card: CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let permanent = self
            .game
            .player_mut(source_perm.player)
            .battle_area
            .get_mut(source_perm.index as usize)?;
        let pos = permanent
            .card_sources
            .iter()
            .position(|card| card.handle() == source_card)?;
        if pos + 1 == permanent.card_sources.len() {
            return None;
        }
        Some(permanent.card_sources.remove(pos))
    }

    /// Decline the pay cost for a queued triggered effect that parked during
    /// `pay_cost_fn`. The effect queue will discard the parked process tail
    /// after the current selection callback unwinds.
    pub fn decline_pending_pay_cost(&mut self) {
        self.game.decline_pending_pay_cost();
    }

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

    /// Cancel the parked leave-the-field event. The carrier stays on the
    /// field; the original deletion / return / etc. is suppressed.
    ///
    /// Writes `ReplacementOutcome::Cancelled` to `Game.parked_replacement.outcome`.
    /// Calling this outside a parked-replacement scope is a `debug_assert!`
    /// panic in dev builds; release builds silently no-op.
    ///
    /// Typical use: inside a `select_*` callback that runs as the body of a
    /// `WhenWouldBeDeleted` replacement-process closure (e.g., Save:
    /// "you may pick a Tamer to slide under instead of being deleted").
    pub fn cancel_leave(&mut self) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "cancel_leave called outside a replacement-process callback; \
             the outcome would be silently dropped"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
        }
    }

    /// Alias for [`Self::cancel_leave`] for replacement-process callbacks
    /// whose card text names the current replacement rather than "leaving".
    pub fn cancel_current_replacement(&mut self) {
        self.cancel_leave();
    }

    pub fn trash_top_security_and_cancel_current_replacement(&mut self, player: PlayerId) -> bool {
        if self.trash_top_security(player) {
            if self.game.parked_replacement.is_some() {
                self.cancel_current_replacement();
            }
            true
        } else {
            false
        }
    }

    pub fn place_sourceless_permanent_bottom_security_and_cancel_current_replacement(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
    ) -> bool {
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        if self
            .game
            .place_sourceless_permanent_on_security_bottom(player, target, self.player)
        {
            if self.game.parked_replacement.is_some() {
                self.cancel_current_replacement();
            }
            true
        } else {
            false
        }
    }

    /// (Track A) Variant of `place_sourceless_permanent_bottom_security_…` that
    /// targets any permanent at any position, then handles the current
    /// replacement window via `handle_replacement` rather than `cancel_leave`.
    pub fn place_permanent_on_security_and_handle_current_replacement(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        if self
            .game
            .place_permanent_on_security_without_leave_replacement(
                player,
                target,
                position,
                face_up,
                self.player,
            )
        {
            if self.game.parked_replacement.is_some() {
                self.handle_replacement();
            }
            true
        } else {
            false
        }
    }

    /// (Track E) Move the resolving effect's source permanent
    /// (`self.source_permanent`) into its owner's security stack at
    /// `position` with the requested orientation. Sugar over
    /// `Game::place_permanent_on_security_observed`.
    ///
    /// Sources below the top card are routed to each source's owner's trash
    /// (firing `OnDigivolutionCardTrashed` per source). Linked cards likewise
    /// go to trash with a single `OnLinkedCardTrashed` dispatch. The top card
    /// becomes a single security slot at the requested end of the stack;
    /// `face_up=true` adds the slot to `face_up_security`.
    ///
    /// Used by printed text "place this Digimon at the bottom of your
    /// security stack face down" (EX4-060), "place this Digimon as your top
    /// security card" (EX9-021), and similar self-placement riders. DCGO
    /// parity: `IPutSecurityPermanent(card.PermanentOfThisCard(), …)`.
    ///
    /// Returns `false` if there is no source permanent (e.g. an Option-card
    /// effect or rule-source effect), if the controller has
    /// `CannotAddSecurityByEffect`, or if either of the
    /// `WhenWouldLeaveBattleArea` / `WhenWouldPlaceInSecurity` replacement
    /// outcomes is non-`None` (cancelled / redirected / etc.).
    ///
    /// **Engine divergence:** DCGO bundles the entire permanent (top +
    /// sources + linked) under a single security slot. The Rust security
    /// model is flat; sources/linked go to trash. See
    /// `Game::place_permanent_on_security_observed` for the divergence note.
    pub fn place_self_at_security(
        &mut self,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        let owner = handle.player;
        self.game.place_permanent_on_security_observed(
            owner,
            handle,
            position,
            face_up,
            self.player,
        )
    }

    /// Flip the topmost still-face-down card in `player`'s security stack
    /// face-up without opening a player choice. If every security card is
    /// already face-up, this is a no-op.
    pub fn flip_security_face_up(&mut self, player: PlayerId) -> bool {
        let Some(card) = self.game.player(player).security.iter().rev().find(|card| {
            !self
                .game
                .player(player)
                .face_up_security
                .contains(&card.card_index)
        }) else {
            return false;
        };
        let card_index = card.card_index;
        self.game
            .player_mut(player)
            .face_up_security
            .insert(card_index);
        true
    }

    /// (Track E) Replacement-aware sibling of `place_self_at_security`. When
    /// invoked inside a parked replacement (e.g. a "would leave" replacement
    /// whose subject is the source permanent), runs the move and then
    /// cancels the parked replacement so the original event does not also
    /// fire.
    ///
    /// Mirrors the shape of
    /// `place_sourceless_permanent_bottom_security_and_cancel_current_replacement`.
    /// Used by EX4-060 (place self at security bottom face-down on
    /// "would leave battle area other than by your effects" replacement).
    pub fn place_self_at_security_and_cancel_current_replacement(
        &mut self,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        if self.place_self_at_security(position, face_up) {
            if self.game.parked_replacement.is_some() {
                self.cancel_current_replacement();
            }
            true
        } else {
            false
        }
    }

    /// (Track E) Option-card flavor of `place_self_at_security`. Consumes
    /// the `Game.pending_option` transient that carries the in-flight
    /// Option card mid-resolution (between pay-cost and dispose), routing
    /// it into its owner's security stack at `position` with the requested
    /// orientation. Used by ST20-15 Island of Adventure's [Main] tail
    /// "Then, place this card face up as the top security card."
    ///
    /// Distinguishing factors vs `place_self_at_security` (Digimon flavor):
    /// - The Option card has no source permanent during `OptionMain`
    ///   resolution; `self.source_permanent` is `None`. The card lives in
    ///   the `Game.pending_option` transient instead.
    /// - There is no source stack to bundle or trash — Options are single
    ///   cards.
    /// - Consuming `pending_option` automatically suppresses the post-
    ///   `OptionMain` dispose-trash: `advance_pending_option` short-circuits
    ///   on `pending_option.is_none()`, so the card lands in security
    ///   instead of routing through the standard Option dispose path.
    ///
    /// Routes through `WhenWouldPlaceInSecurity` replacement; bails (and
    /// restores `pending_option`) on any non-`None` outcome or installed
    /// pending selection. Gated by `CannotAddSecurityByEffect` (player-
    /// scoped against the acting player).
    ///
    /// DCGO parity: `ReplaceTopSecurityWithFaceUpOptionMainEffect` family
    /// (ST20-15 and similar Option-card self-placement riders).
    pub fn place_self_option_at_security(
        &mut self,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        use crate::enums::Zone;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Gate on `CannotAddSecurityByEffect` BEFORE consuming pending_option,
        // so a gated call leaves the resolution flow intact.
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        // Take pending_option. If absent, this method was invoked outside an
        // Option-card resolution and is a clean no-op.
        let Some(pending) = self.game.pending_option.take() else {
            return false;
        };

        // Snapshot fields needed across the replacement-call mutable borrow.
        let owner = pending.owner;
        let card_handle = pending.card.handle();
        let face_up_key = pending.card.card_index;
        let source_zone = pending.source_kind.zone();

        // Route through WhenWouldPlaceInSecurity. If a selection is installed
        // or the replacement returns a non-None outcome, restore pending and
        // bail — the original Option flow will then continue normally.
        let cause = self.game.infer_effect_cause(self.player);
        let subject = ReplacementSubject::Card(card_handle, source_zone);
        let outcome = self.game.try_replace(
            crate::enums::EffectTiming::WhenWouldPlaceInSecurity,
            subject,
            cause,
            Some(Zone::Security),
        );
        if self.game.pending_selection.is_some() || !matches!(outcome, ReplacementOutcome::None) {
            // Restore so the dispose flow can complete normally.
            self.game.pending_option = Some(pending);
            return false;
        }

        // Commit the move. Move pending.card into security at `position`.
        let card = pending.card;
        match position {
            crate::enums::StackPosition::Top => {
                self.game.player_mut(owner).security.push(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.game.player_mut(owner).security.insert(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.game.player(owner).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.game.rng.gen_range(0..=sec_len)
                };
                self.game.player_mut(owner).security.insert(idx, card);
            }
        }
        if face_up {
            self.game
                .player_mut(owner)
                .face_up_security
                .insert(face_up_key);
        }
        true
    }

    /// Mark the parked replacement as custom-handled — the process body has
    /// already mutated state and the original event should be skipped.
    /// Distinct from `cancel_leave` only at the doc level; both result in
    /// `commit_deferred_outcome` taking the no-op arm.
    ///
    /// Writes `ReplacementOutcome::CustomHandled` to the parked slot.
    /// Calling this outside a parked-replacement scope is a `debug_assert!`
    /// panic in dev builds; release builds silently no-op.
    pub fn handle_replacement(&mut self) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "handle_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::CustomHandled;
        }
    }

    /// Redirect the parked event to a different zone (e.g., Trash → Deck for
    /// Evade, Trash → Hand for return-to-hand replacement).
    ///
    /// Writes `ReplacementOutcome::Redirected(zone)` to the parked slot.
    /// Honored by `commit_deferred_outcome`'s existing redirect arms.
    /// Calling outside a parked-replacement scope is a `debug_assert!` panic
    /// in dev builds; release builds silently no-op.
    pub fn redirect_replacement(&mut self, zone: crate::enums::Zone) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "redirect_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Redirected(zone);
        }
    }

    /// Substitute a different subject for the parked event. `commit_deferred_outcome`
    /// recursively dispatches the original event against the substituted subject
    /// (e.g., Decoy: replace deletion-target with self).
    ///
    /// Writes `ReplacementOutcome::Substituted(subject)` to the parked slot.
    /// Calling outside a parked-replacement scope is a `debug_assert!` panic
    /// in dev builds; release builds silently no-op.
    pub fn substitute_replacement(&mut self, subject: crate::replacement::ReplacementSubject) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "substitute_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Substituted(subject);
        }
    }

    pub fn trash_delay_source(&mut self) -> bool {
        matches!(self.trash_delay_source_status(), DelayCostStatus::Paid)
    }

    pub fn trash_delay_source_status(&mut self) -> DelayCostStatus {
        let Some(source) = self.source_permanent else {
            return DelayCostStatus::Unpaid;
        };
        let Some(source_card) = self.permanent_top_card_handle(source) else {
            return DelayCostStatus::Unpaid;
        };
        self.game
            .delete_permanent_with_cause(source, ReplacementCause::Cost);
        if self.game.pending_selection.is_some() {
            return DelayCostStatus::Pending;
        }
        if self.delay_source_card_in_trash(source.player, source_card) {
            DelayCostStatus::Paid
        } else {
            DelayCostStatus::Unpaid
        }
    }

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
    }

    pub fn digivolve_replacement_subject_without_cost(
        &mut self,
        subject: ReplacementSubject,
        card: CardHandle,
    ) -> bool {
        let Some(target) = subject.permanent() else {
            return false;
        };
        if (target.index as usize) >= self.game.player(target.player).battle_area.len() {
            return false;
        }

        let Some(hand_index) = self
            .game
            .player(self.player)
            .hand
            .iter()
            .position(|source| source.handle() == card)
        else {
            return false;
        };

        let turn = self.game.turn_count;
        let card = self.game.player_mut(self.player).hand.remove(hand_index);
        self.game.player_mut(target.player).battle_area[target.index as usize]
            .digivolve(card, turn);

        self.game.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            crate::selection::TriggerSource::Permanent(target),
        );
        self.game.drain_effect_queue();

        for pid in 0..self.game.players.len() {
            self.game.enqueue_triggered(
                EffectTiming::OnDigivolve,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.game.drain_effect_queue();
        true
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

    pub fn gain_memory(&mut self, amount: i16) {
        let target = self.player;
        // Phase 6: CannotGainMemoryByEffect — suppress all memory gains by effect.
        if self
            .game
            .modifiers
            .player_has(target, ModifierType::CannotGainMemoryByEffect)
        {
            return;
        }
        // Phase 6: CannotGainMemoryExceptFromTamers — only Tamer-sourced gains are
        // allowed; block Digimon/Option-sourced gains.
        if self
            .game
            .modifiers
            .player_has(target, ModifierType::CannotGainMemoryExceptFromTamers)
            && !self.source_is_tamer()
        {
            return;
        }
        // Track C / D consult site (2026-05-08): permanent-scoped
        // `CannotAddMemory` — while any permanent in the acting player's
        // battle area carries this modifier, the controller's effects
        // can't add memory. Sibling of player-scoped
        // `CannotGainMemoryByEffect` for printed text anchored to a
        // specific Digimon.
        let battle_area_len = self.game.player(target).battle_area.len();
        for i in 0..battle_area_len {
            let h = PermanentHandle {
                player: target,
                index: i as u8,
            };
            if self.game.modifiers.has(h, ModifierType::CannotAddMemory) {
                return;
            }
        }
        self.game.gain_memory_for_player(target, amount);
    }

    pub fn lose_memory(&mut self, amount: i16) {
        let new_memory = self.game.memory - amount;
        self.game.set_memory(new_memory);
    }

    pub fn set_memory(&mut self, value: i16) {
        self.game.set_memory(value);
    }

    // ─── Card draw / trash ────────────────────────────────────────────

    pub fn draw(&mut self, player: PlayerId, count: u8) -> u8 {
        use crate::enums::EffectTiming;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Phase 6: if the drawing player has CannotDrawByEffect, suppress draw.
        // The flood gate fires FIRST (preserves Phase 6 semantics); if blocked,
        // no replacement window opens.
        if self
            .game
            .modifiers
            .player_has(player, ModifierType::CannotDrawByEffect)
        {
            return 0;
        }

        // Phase 7 Task 4: fire WhenWouldDraw once per draw call (not once
        // per card). Subject is the drawing player; no original_destination.
        let cause = self.game.infer_effect_cause(player);
        let subject = ReplacementSubject::Player(player);
        let outcome = self
            .game
            .try_replace(EffectTiming::WhenWouldDraw, subject, cause, None);
        if self.game.pending_selection.is_some() {
            return 0;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return 0;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(
                    false,
                    "Redirected not meaningful for WhenWouldDraw (player-scoped)"
                );
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for WhenWouldDraw v1");
            }
        }

        // Opaque-aware: replace draw_many with N calls to
        // draw_one_for_player so opaque opponents pull from RevealSource.
        // Errors fall through as "draw stopped" — same semantic as the
        // standard-mode draw_many returning fewer cards than requested.
        let mut drawn: u8 = 0;
        for _ in 0..count {
            match self.game.draw_one_for_player(player) {
                Ok(true) => drawn += 1,
                Ok(false) => break, // deck-out
                Err(e) => {
                    eprintln!(
                        "[opaque-deck] effect-driven draw error for player {}: {}",
                        player, e
                    );
                    break;
                }
            }
        }
        if drawn > 0 {
            self.game.mark_until_condition_dirty();
            self.game.reevaluate_until_condition_modifiers_if_dirty();
        }
        drawn
    }

    /// Trash the top N cards of a player's deck (mill effect).
    pub fn trash_from_top(&mut self, player: PlayerId, count: u8) -> u8 {
        let mut trashed = 0;
        for _ in 0..count {
            // Opaque-aware: opaque opponents materialize from RevealSource
            // tagged Mill; standard players pop from their ordered deck.
            let card = match self.game.take_from_deck_top_for_player(
                player,
                crate::opaque_deck::RevealKind::Mill,
            ) {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(e) => {
                    eprintln!(
                        "[opaque-deck] trash_from_top error for player {}: {}",
                        player, e
                    );
                    break;
                }
            };
            self.game.player_mut(player).trash.push(card);
            trashed += 1;
        }
        trashed
    }

    /// Move the top card of `player`'s security stack to their trash.
    /// No-op if the stack is empty. Returns true if a card was moved.
    ///
    /// Phase 7 Task 4: fires `WhenWouldBeTrashed` at entry. Subject carries
    /// the top security card's handle; cause inferred.
    pub fn trash_top_security(&mut self, player: PlayerId) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Opaque-aware: if top of security is a placeholder, materialize
        // before the replacement window so the WhenWouldBeTrashed lookup
        // sees real card identity (replacement effects are often keyed
        // on color/type).
        if let Some(top_idx) = self.game.player(player).security.len().checked_sub(1) {
            self.game.ensure_security_materialized(player, top_idx);
        }
        // Snapshot the top-of-security card handle before any state change.
        let top_handle = match self.game.player(player).security.last() {
            Some(c) => c.handle(),
            None => return false,
        };
        let cause = self.game.infer_effect_cause(player);
        let subject = ReplacementSubject::Card(top_handle, Zone::Security);
        let outcome = self.game.try_replace(
            EffectTiming::WhenWouldBeTrashed,
            subject,
            cause,
            Some(Zone::Trash),
        );
        if self.game.pending_selection.is_some() {
            return false;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(false, "Redirected not supported for WhenWouldBeTrashed v1");
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for WhenWouldBeTrashed v1");
            }
        }

        let p = self.game.player_mut(player);
        if let Some(card) = p.security.pop() {
            p.face_up_security.remove(&card.card_index);
            self.fire_security_removed_observers(
                player,
                card,
                crate::selection::SecurityRemovalDestination::Trash,
            );
            self.game.mark_until_condition_dirty();
            self.game.reevaluate_until_condition_modifiers_if_dirty();
            true
        } else {
            false
        }
    }

    /// Trash the bottom card of `player`'s security stack (index 0).
    ///
    /// Mirrors `trash_top_security` but removes `security[0]` instead of
    /// the last element. Replacement effects and observers fire identically
    /// to the top-trash path.
    pub fn trash_bottom_security(&mut self, player: PlayerId) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Opaque-aware: materialize the bottom security card if it's a
        // placeholder, so subsequent WhenWouldBeTrashed lookups and
        // observer firings see real card identity.
        if !self.game.player(player).security.is_empty() {
            self.game.ensure_security_materialized(player, 0);
        }
        // Snapshot the bottom-of-security card handle before any state change.
        let bottom_handle = match self.game.player(player).security.first() {
            Some(c) => c.handle(),
            None => return false,
        };
        let cause = self.game.infer_effect_cause(player);
        let subject = ReplacementSubject::Card(bottom_handle, Zone::Security);
        let outcome = self.game.try_replace(
            EffectTiming::WhenWouldBeTrashed,
            subject,
            cause,
            Some(Zone::Trash),
        );
        if self.game.pending_selection.is_some() {
            return false;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(false, "Redirected not supported for WhenWouldBeTrashed v1");
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for WhenWouldBeTrashed v1");
            }
        }

        let p = self.game.player_mut(player);
        if p.security.is_empty() {
            return false;
        }
        let card = p.security.remove(0);
        p.face_up_security.remove(&card.card_index);
        self.fire_security_removed_observers(
            player,
            card,
            crate::selection::SecurityRemovalDestination::Trash,
        );
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Trash a specific card — identified by the stable `handle` — from
    /// `player`'s security stack. Used by effects that let a player trash
    /// "any 1" of a security stack: the card is chosen via a `select_security`
    /// binding, which yields a `CardHandle`. Addressing the card by handle
    /// (rather than a positional index) means an intervening security-stack
    /// mutation cannot cause the wrong card to be trashed.
    ///
    /// No-op (returns false) if `handle` is not currently in `player`'s
    /// security stack. Mirrors `trash_top_security` / `trash_bottom_security`:
    /// same `WhenWouldBeTrashed` replacement window and security-removed
    /// observer fan-out, but addresses an arbitrary chosen card.
    /// G-TRASH-SELECTED-SECURITY.
    pub fn trash_security_card(&mut self, player: PlayerId, handle: CardHandle) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // The handle must currently be in `player`'s security stack.
        // Opaque-aware: placeholder card_index values are preserved
        // across materialization, so the handle iteration finds the
        // right slot whether or not it's still a placeholder. We then
        // materialize at that position so identity-dependent effects
        // (replacement lookups, observer firings) see real card data.
        let Some(initial_pos) = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == handle)
        else {
            return false;
        };
        self.game.ensure_security_materialized(player, initial_pos);
        let cause = self.game.infer_effect_cause(player);
        let subject = ReplacementSubject::Card(handle, Zone::Security);
        let outcome = self.game.try_replace(
            EffectTiming::WhenWouldBeTrashed,
            subject,
            cause,
            Some(Zone::Trash),
        );
        if self.game.pending_selection.is_some() {
            return false;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(false, "Redirected not supported for WhenWouldBeTrashed v1");
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for WhenWouldBeTrashed v1");
            }
        }

        // Re-find the card by handle — the replacement window may have mutated
        // the security stack.
        let p = self.game.player_mut(player);
        let Some(pos) = p.security.iter().position(|c| c.handle() == handle) else {
            return false;
        };
        let card = p.security.remove(pos);
        p.face_up_security.remove(&card.card_index);
        self.fire_security_removed_observers(
            player,
            card,
            crate::selection::SecurityRemovalDestination::Trash,
        );
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    fn fire_security_removed_observers(
        &mut self,
        defender: PlayerId,
        card: crate::card_source::CardSource,
        destination: crate::selection::SecurityRemovalDestination,
    ) {
        let observer_player = self.game.next_clockwise(defender);
        let cause =
            crate::trigger_context::EventCause::from(self.game.infer_effect_cause(defender));
        self.game.fire_effect_security_removal(
            defender,
            observer_player,
            self.player,
            cause,
            card,
            destination,
        );
    }

    // ─── Field mutations ──────────────────────────────────────────────

    pub fn can_affect_permanent(&self, target: PermanentHandle) -> bool {
        !self.game.progress_excludes(target, Some(self.player))
            && !self
                .game
                .permanent_is_unaffected_by_effect(target, self.player, self.source_kind)
    }

    fn cleanup_exposed_battle_area_digi_egg(&mut self, target: PermanentHandle) -> bool {
        let exposed = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_some_and(|perm| {
                perm.top_card().card_kind(&self.game.card_data) == CardKind::DigiEgg
            });
        if !exposed {
            return false;
        }

        self.game.delete_permanent_with_effects(target);
        true
    }

    pub fn delete_permanent(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        // Route through the Game-level fire-site so OnDeletion observers and
        // WhenWouldBeDeleted replacements run. `delete_permanent_with_effects`
        // infers cause from `effect_source_player` / `pending_attack` /
        // `security_resolution`.
        self.game.delete_permanent_with_effects(target);
    }

    pub fn trash_breeding_permanent(&mut self, target: PermanentHandle) -> bool {
        if target.index != crate::action::space::BREEDING_TARGET as u8 {
            return false;
        }
        if !self.can_affect_permanent(target) {
            return false;
        }

        let Some(perm) = self.game.player_mut(target.player).breeding_area.take() else {
            return false;
        };

        if !perm.card_sources.is_empty() && !perm.top_card().is_token {
            for card in perm.card_sources {
                let owner = card.owner;
                self.game.player_mut(owner).trash.push(card);
            }
        }
        for card in perm.linked_cards {
            let owner = card.owner;
            self.game.player_mut(owner).trash.push(card);
        }

        self.game.clear_permanent_full(target);
        self.game.modifiers.expire_player_on_permanent_leave(target);
        self.game.mark_until_condition_dirty();
        true
    }

    /// Pop up to `amount` cards off `target`'s digivolution stack,
    /// trashing each popped source into the target owner's trash.
    ///
    /// Rules:
    ///   * Never pops the base card — `Permanent` must always retain at
    ///     least one `CardSource`.
    ///   * `stop_at_level = Some(L)` — stop early if popping would leave
    ///     a top whose level is strictly less than `L`. For standard
    ///     De-Digivolve N use `Some(3)` (card text: "You can't trash
    ///     past level 3 cards").
    ///   * `stop_at_level = None` — no level floor; pop until the base.
    ///   * `amount = Some(N)` — cap pops at N.
    ///   * `amount = None` — unbounded (equivalent to `Some(u8::MAX)`).
    ///
    /// Returns the actual number of cards popped.
    pub fn de_digivolve(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        amount: Option<u8>,
    ) -> u8 {
        if !self.can_affect_permanent(target) {
            return 0;
        }

        use crate::enums::EffectTiming;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Phase 7 Task 4: fire WhenWouldBeDeDigivolved once at entry (not
        // per iteration of the popping loop). Substitute(Permanent) retargets
        // the loop at another permanent; v1 does not support "reduce N" via
        // mutable ctx — scripts that want to reduce N should cancel and
        // re-call with a lower amount.
        let cause = self.game.infer_effect_cause(target.player);
        let subject = ReplacementSubject::Permanent(target);
        let outcome =
            self.game
                .try_replace(EffectTiming::WhenWouldBeDeDigivolved, subject, cause, None);
        if self.game.pending_selection.is_some() {
            return 0;
        }
        let effective_target = match outcome {
            ReplacementOutcome::None => target,
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return 0;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(
                    false,
                    "Redirected not meaningful for WhenWouldBeDeDigivolved"
                );
                target
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => other,
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute for WhenWouldBeDeDigivolved"
                );
                target
            }
        };
        let target = effective_target;

        let max = amount.unwrap_or(u8::MAX);
        let mut popped: u8 = 0;

        while popped < max {
            let perm = match self
                .game
                .player(target.player)
                .battle_area
                .get(target.index as usize)
            {
                Some(p) => p,
                None => break,
            };

            if perm.stack_size() <= 1 {
                break;
            }

            let next_top_level = {
                let stack = perm.digivolution_cards();
                let next_top = &stack[stack.len() - 2];
                next_top.level(&self.game.card_data)
            };

            if let (Some(floor), Some(nt_level)) = (stop_at_level, next_top_level) {
                if nt_level < floor {
                    break;
                }
            }

            let owner = target.player;
            let (popped_card, host_card) = {
                let p = self.game.player_mut(owner);
                let stack = &mut p.battle_area[target.index as usize].card_sources;
                debug_assert!(stack.len() >= 2, "stack_size-guard failed");
                let popped_card = stack.pop().expect("stack_size-guarded pop");
                let host_card = stack
                    .last()
                    .map(|source| source.handle())
                    .unwrap_or_else(|| popped_card.handle());
                (popped_card, host_card)
            };
            let source_card = popped_card.handle();
            self.game.player_mut(owner).trash.push(popped_card);
            self.game.fire_digivolution_card_trashed(
                owner,
                target,
                host_card,
                source_card,
                crate::trigger_context::EventCause::from(self.game.infer_effect_cause(owner)),
            );
            popped += 1;

            if self.cleanup_exposed_battle_area_digi_egg(target) {
                break;
            }
        }

        popped
    }

    /// Materialize a token on `controller`'s battle area.
    ///
    /// Looks up `token_name` in `game.token_registry`, synthesizes a
    /// `CardSource` with `is_token = true`, wraps it in a `Permanent`, and
    /// pushes onto `controller.battle_area`. No play cost and no token
    /// OnPlay drain, but entered-field observers fire with `effect_initiated`
    /// so cards that watch effects playing Tokens can see the new permanent.
    ///
    /// Returns the spawned permanent's handle, or `None` if the token name
    /// is unknown or the field is full.
    pub fn play_token(
        &mut self,
        controller: crate::enums::PlayerId,
        token_name: &str,
    ) -> Option<crate::permanent::PermanentHandle> {
        use crate::card_source::CardSource;
        use crate::permanent::{Permanent, PermanentHandle};

        let def = self.game.token_registry.get(token_name)?;
        let target_card_id = def.card_id.clone();
        let data_index = self
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == target_card_id)?;
        debug_assert_eq!(
            self.game.card_data[data_index].card_kind,
            crate::enums::CardKind::Token,
            "token_registry entry must map to a CardKind::Token CardData row"
        );

        let slots = self.game.rules.field_slots as usize;
        if self.game.player(controller).battle_area.len() >= slots {
            return None;
        }

        let card_index = self.game.next_card_index();
        let mut card = CardSource::new_token(data_index, controller, card_index);
        card.card_index = card_index;
        let turn = self.game.turn_count;
        let perm = Permanent::new(card, turn);

        let player = self.game.player_mut(controller);
        player.battle_area.push(perm);
        let idx = player.battle_area.len() - 1;
        let entered = PermanentHandle {
            player: controller,
            index: idx as u8,
        };
        let entered_card = self.game.players[controller as usize].battle_area[idx]
            .top_card()
            .handle();
        let top_card = self.game.players[controller as usize].battle_area[idx].top_card();
        let emitted_card_id = top_card.card_id(&self.game.card_data).to_string();
        let cost_printed = self.game.card_data[top_card.data_index].play_cost as i16;
        let seq = self.game.next_event_seq();
        self.game.events.push(crate::events::GameEvent::Play {
            seq,
            player: controller,
            card_id: emitted_card_id,
            field_index: idx as u8,
            // Token spawn — no memory paid; tokens have play_cost=0
            // in CardData typically but read it explicitly to handle
            // any future token whose printed cost differs.
            cost_paid: 0,
            cost_printed,
            via_alt_path: None,
        });
        self.game.enqueue_triggered(
            crate::enums::EffectTiming::OnEnterFieldAnyone,
            crate::selection::TriggerSource::EnteredField {
                player: controller,
                permanent: entered,
                card: entered_card,
                effect_initiated: true,
            },
        );
        self.game.enqueue_triggered(
            crate::enums::EffectTiming::OnAllyPlayed,
            crate::selection::TriggerSource::EnteredField {
                player: controller,
                permanent: entered,
                card: entered_card,
                effect_initiated: true,
            },
        );
        self.game.drain_effect_queue();
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        Some(entered)
    }

    /// Suspend a permanent and fire `OnSuspend` observers.
    /// Delegates to `Game::suspend` — the canonical single-target chokepoint.
    pub fn suspend(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.suspend(target);
    }

    /// Pay the source permanent's suspend-self activation cost.
    ///
    /// Used as the closure body for [`crate::effect::EffectBuilder::activation_cost`]
    /// on Tamer triggered abilities like "by suspending this Tamer, gain 1
    /// memory" (BT4-097 / BT8-090 / BT13-101 family). Returns `false` if
    /// the source permanent is gone (extremely unlikely mid-trigger) or
    /// is already suspended — in which case the body silently aborts and
    /// the OPT slot is consumed by the queue dispatcher. Returns `true`
    /// after delegating to [`Self::suspend`] (which fires `OnSuspend`
    /// observers and the canonical single-target chokepoint).
    ///
    /// No-approximations note: this helper does NOT prompt — the player's
    /// "may you accept" prompt belongs to [`crate::effect::EffectBuilder::optional`]
    /// and runs BEFORE the cost. The cost is intrinsic to the trigger,
    /// not a player decision (Working Rule 17).
    pub fn suspend_self_as_cost(&mut self) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        let already_suspended = self
            .source_permanent()
            .map(|perm| perm.is_suspended)
            .unwrap_or(true);
        if already_suspended {
            return false;
        }
        self.suspend(handle);
        true
    }

    /// Install a player-scoped one-shot future-digivolve cost reducer
    /// (`G-COST-REDUCE-ALLY-DIGIVOLVE`).
    ///
    /// Used by BT3-103 Hidden Potential Discovered!'s `[Main]` clause:
    /// "For the turn, when one of your green Digimon would next digivolve,
    /// by suspending 1 of your Digimon, reduce the digivolution cost by 5."
    ///
    /// The reducer is pushed onto `Game::player_digivolve_cost_reducers`
    /// and is consulted at the top of each digivolve-from-hand cost path.
    /// `target_color` gates which digivolutions qualify; `single_fire`
    /// consumes the reducer on the first successful application; the
    /// reducer expires at end of the installing player's turn.
    ///
    /// When `suspend_cost` is `true`, applying the reduction prompts the
    /// player to suspend one of their own unsuspended Digimon — an
    /// interactive, player-visible cost surfaced through `pending_selection`
    /// (Working Rule §17). No auto-suspend.
    pub fn arm_player_digivolve_cost_reducer(
        &mut self,
        amount: i32,
        single_fire: bool,
        target_color: Option<crate::enums::CardColor>,
        suspend_cost: bool,
    ) {
        let reducer = crate::player_cost_reducer::PlayerDigivolveCostReducer {
            player: self.player,
            source_card: self.source_card,
            kind: crate::player_cost_reducer::PlayerCostReducerKind::Digivolve,
            expiry: crate::player_cost_reducer::PlayerCostReducerExpiry::EndOfTurn,
            amount,
            single_fire,
            target_color,
            suspend_cost,
        };
        self.game.player_digivolve_cost_reducers.push(reducer);
    }

    /// Pay the source permanent's return-self-to-deck-bottom activation
    /// cost.
    ///
    /// Used as the closure body for
    /// [`crate::effect::EffectBuilder::activation_cost`] on Tamer
    /// triggered abilities like "By returning this Tamer to the bottom
    /// of the deck..." (BT22-088 / BT22-094 / BT17-093 / EX11-071
    /// family). Moves the top card of the source permanent to the
    /// controller's deck bottom, trashes the rest of the digivolution
    /// stack per standard return-to-deck rules, and fires
    /// `OnLeaveField`. Returns `false` if the source permanent is gone
    /// (extremely unlikely mid-trigger but possible if a prior chain
    /// destroyed it).
    pub fn return_self_to_deck_bottom_as_cost(&mut self) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        if self.source_permanent().is_none() {
            return false;
        }
        // Use the top-card-only return path: the source's top card moves
        // to its owner's deck bottom; any remaining digivolution sources
        // are trashed by `Game::return_to_deck`. Mirrors the
        // `return_to_deck { include_sources: false, position: bottom }`
        // DSL step shape applied to `source`.
        self.game
            .return_to_deck(handle, crate::enums::StackPosition::Bottom)
    }

    /// Pay the source permanent's trash-self activation cost.
    ///
    /// Used as the closure body for [`crate::effect::EffectBuilder::activation_cost`]
    /// on `<Delay>` abilities whose printed cost is "by trashing this card"
    /// (BT21-093 Raging Serpentine family). Deletes the source permanent —
    /// for a `<Delay>` Option this moves the card to its owner's trash —
    /// routing through `delete_permanent` so `OnDeletion` observers and
    /// `WhenWouldBeDeleted` replacements fire identically to the
    /// `delete_permanent` step. Returns `false` if the source permanent has
    /// already left the field.
    ///
    /// No-approximations note: per Comprehensive Rules 16-16-2 the processing
    /// of a `<Delay>` is optional; the player's accept/decline prompt belongs
    /// to [`crate::effect::EffectBuilder::optional`] and runs BEFORE this cost.
    pub fn trash_self_as_cost(&mut self) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        if self.source_permanent().is_none() {
            return false;
        }
        self.delete_permanent(handle);
        true
    }

    /// Unsuspend a permanent and fire `OnUnsuspend` observers.
    /// Delegates to `Game::unsuspend` — the canonical single-target chokepoint.
    pub fn unsuspend(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.unsuspend(target);
    }

    /// Move a specific card from `player`'s deck to their hand.
    pub fn add_to_hand_from_deck(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_deck(player, card)
    }

    /// Move a specific card from `player`'s trash to their hand.
    pub fn add_to_hand_from_trash(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_trash(player, card)
    }

    /// Move a specific card from `player`'s security stack to their hand.
    pub fn add_to_hand_from_security(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(idx) = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        // Opaque-aware: materialize before moving to hand — the card's
        // identity must be real to land in hand sensibly (subsequent
        // hand reads, plays, etc. expect a real data_index).
        self.game.ensure_security_materialized(player, idx);
        let removed = self.game.player_mut(player).security.remove(idx);
        let owner = removed.owner;
        self.game
            .player_mut(player)
            .face_up_security
            .remove(&removed.card_index);
        self.fire_security_removed_observers(
            player,
            removed,
            crate::selection::SecurityRemovalDestination::Hand(owner),
        );
        true
    }

    /// Move the top card of `player`'s security stack to its owner's hand.
    pub fn add_top_security_to_hand(&mut self, player: PlayerId) -> bool {
        let Some(card) = self
            .game
            .player(player)
            .security
            .last()
            .map(|card| card.handle())
        else {
            return false;
        };
        self.add_to_hand_from_security(player, card)
    }

    /// Move the bottom card of `player`'s security stack to its owner's hand.
    pub fn add_bottom_security_to_hand(&mut self, player: PlayerId) -> bool {
        let Some(card) = self
            .game
            .player(player)
            .security
            .first()
            .map(|card| card.handle())
        else {
            return false;
        };
        self.add_to_hand_from_security(player, card)
    }

    /// Reveal up to `n` cards from the top of `player`'s deck. See
    /// `Game::reveal_top_deck`.
    pub fn reveal_top_deck(
        &mut self,
        player: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        self.game.reveal_top_deck(player, n)
    }

    pub fn reveal_top_digitama(
        &mut self,
        player: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        self.game.reveal_top_digitama(player, n)
    }

    /// Snapshot of the current reveal pool. Scripts inspect this to decide
    /// follow-up moves.
    pub fn revealed(&self) -> &[crate::card_source::CardSource] {
        &self.game.revealed_cards
    }

    /// Trash a specific hand card by index.
    ///
    /// Phase 7 Task 4: fires `WhenWouldBeTrashed` at entry. Subject is the
    /// hand card handle; cause inferred.
    pub fn trash_from_hand_by_index(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<crate::card_source::CardHandle> {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Snapshot the card handle before any mutation. Return early if
        // the index is invalid.
        let card_handle = {
            let p = self.game.player(player);
            if hand_index >= p.hand.len() {
                return None;
            }
            p.hand[hand_index].handle()
        };
        let cause = self.game.infer_effect_cause(player);
        let subject = ReplacementSubject::Card(card_handle, Zone::Hand);
        let outcome = self.game.try_replace(
            EffectTiming::WhenWouldBeTrashed,
            subject,
            cause,
            Some(Zone::Trash),
        );
        if self.game.pending_selection.is_some() {
            return None;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return None;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(false, "Redirected not supported for WhenWouldBeTrashed v1");
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for WhenWouldBeTrashed v1");
            }
        }

        self.game.trash_from_hand_by_index(player, hand_index)
    }

    /// Move a specific revealed card into `player`'s hand.
    pub fn add_to_hand_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_reveal(player, card)
    }

    /// Move the card currently resolving from security into its defender's hand.
    ///
    /// This consumes `Game.pending_security`, so the security dispose phase
    /// cannot also trash the card. If the card was already played from
    /// security, the pending state is restored and this is a no-op.
    pub fn add_pending_security_to_hand(&mut self) -> bool {
        let Some(pending) = self.game.pending_security.take() else {
            return false;
        };

        if pending.played {
            self.game.pending_security = Some(pending);
            return false;
        }

        let owner = pending.card.owner;
        self.game.player_mut(owner).hand.push(pending.card);
        true
    }

    /// Move a specific revealed card into `player`'s trash.
    pub fn trash_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.trash_from_reveal(player, card)
    }

    /// Move a specific revealed card back to `player`'s deck at `position`.
    pub fn return_to_deck_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        self.game.return_to_deck_from_reveal(player, card, position)
    }

    /// Place all cards currently in `game.revealed_cards` back onto `player`'s
    /// deck at `position`, in a player-chosen order.
    ///
    /// **Contract**: `ordered_vec[0]` is drawn first among the placed cards.
    ///
    /// - **Empty pool** → silent no-op; no `PendingSelection` installed.
    /// - **1 card** → still installs a 1-choice `OrderedPermutation` so the RL
    ///   agent sees the (trivial) ordering decision (no-approximations policy).
    /// - **N cards** → installs `select_ordered_permutation` over the remainder;
    ///   the callback places cards at `position` using the correct iteration
    ///   direction so `ordered_vec[0]` ends up drawn first:
    ///   - `Top`:    iterate `rev()`, push each (`deck.push`) — last pushed lands
    ///               at Vec-end (= deck top = drawn first).
    ///   - `Bottom`: iterate forward, insert each at index 0 (`deck.insert(0)`) —
    ///               each subsequent insert pushes the previous card deeper; final
    ///               state has `ordered_vec[0]` at the highest index among the
    ///               placed group (closest to top of the bottom-placed set).
    ///   - `Random`: iterate forward, call `return_to_deck_from_reveal(Random)`
    ///               for each — placement order is semantically irrelevant but the
    ///               permutation selection is still surfaced to the RL agent.
    pub fn place_remainder_on_deck(&mut self, player: PlayerId, position: StackPosition) {
        // Snapshot handles of every card currently in the reveal pool.
        let remainder: Vec<CardHandle> = self
            .game
            .revealed_cards
            .iter()
            .map(|cs| cs.handle())
            .collect();

        // Empty pool → silent no-op.
        if remainder.is_empty() {
            return;
        }

        debug_assert!(
            remainder.len() <= 10,
            "place_remainder_on_deck: reveal pool has {} cards; select_ordered_permutation is capped at 10",
            remainder.len()
        );

        self.select_ordered_permutation(
            remainder,
            "Place remaining cards on deck in any order",
            move |ctx, ordered_vec| {
                match position {
                    StackPosition::Top => {
                        // Reverse-iterate: last item is pushed first, so ordered_vec[0]
                        // is pushed last → lands at Vec-end (deck top) → drawn first.
                        for handle in ordered_vec.iter().rev() {
                            let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Top);
                            debug_assert!(placed, "place_remainder_on_deck: handle {:?} not found in revealed_cards at placement time", handle);
                        }
                    }
                    StackPosition::Bottom => {
                        // Forward-iterate with insert(0): ordered_vec[0] is inserted
                        // first at index 0; each subsequent insert pushes it one step
                        // further from index 0. Final: ordered_vec[0] is at the highest
                        // index among the placed group (closest to top within the
                        // bottom-placed set) → drawn first among them.
                        for handle in ordered_vec.iter() {
                            let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Bottom);
                            debug_assert!(placed, "place_remainder_on_deck: handle {:?} not found in revealed_cards at placement time", handle);
                        }
                    }
                    StackPosition::Random => {
                        // Each card is placed at a random position. The permutation
                        // selection is still surfaced — the ordering is strategically
                        // irrelevant but the RL action space must see it (§17).
                        for handle in ordered_vec.iter() {
                            let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Random);
                            debug_assert!(placed, "place_remainder_on_deck: handle {:?} not found in revealed_cards at placement time", handle);
                        }
                    }
                }
            },
        );
    }

    /// Shuffle `player`'s deck. Pair with `add_to_hand_from_deck` for
    /// "search and shuffle" effects.
    pub fn shuffle_deck(&mut self, player: PlayerId) {
        self.game.shuffle_deck(player);
    }

    /// Shuffle `player`'s security stack.
    pub fn shuffle_security(&mut self, player: PlayerId) {
        self.game.shuffle_security(player);
    }

    /// Play a card from `player`'s hand at `hand_index`, deducting memory
    /// according to `cost_delta`. OnPlay effects fire.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if the hand index is invalid, the battle area is full, or memory is
    /// insufficient.
    pub fn play_from_hand_with_cost(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let field_index = self.game.play_from_hand_with_cost(
            player,
            hand_index,
            cost_delta,
            PlaySource::ByEffect,
        )?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    /// Play a card from `player`'s hand at `hand_index` **without subtracting
    /// memory**. Used by effects that say "play this without paying its memory
    /// cost" (e.g. DSL `PlayFromHandFree` step lowerings).
    ///
    /// Thin alias over `play_from_hand_with_cost(_, _, CostDelta::Free)`:
    /// `CostDelta::Free.resolve(_) == 0` → `effective_cost = 0` →
    /// `pay_memory(0)` is a no-op, so memory is unchanged. OnPlay +
    /// OnEnterFieldAnyone triggers fire as normal.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None` if
    /// the hand index is invalid, the battle area is full, or the play was
    /// gated by a flood-gate (`CannotPlayDigimonByEffect`).
    pub fn play_from_hand_free(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<PermanentHandle> {
        self.play_from_hand_with_cost(player, hand_index, crate::enums::CostDelta::Free)
    }

    pub fn play_from_hand_free_suppress_on_play(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        match self
            .game
            .play_from_hand_with_cost_result_from_origin_suppress(
                player,
                hand_index,
                crate::enums::CostDelta::Free,
                PlaySource::ByEffect,
                false,
                PendingWouldPlayOrigin::Hand,
                suppress_on_play,
            ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending | PlayFromHandCostResult::Failed => None,
        }
    }

    pub fn play_from_trash_with_cost_suppress_on_play(
        &mut self,
        player: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let field_index = self.game.play_from_trash_with_cost_suppress(
            player,
            trash_index,
            cost_delta,
            PlaySource::ByEffect,
            suppress_on_play,
        )?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    pub fn play_from_hand_free_with_provenance(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<(PermanentHandle, crate::trigger_context::ProvenanceToken)> {
        let card = self.game.player(player).hand.get(hand_index)?.handle();
        let token = self.game.provenance_token_for_card(card);
        let permanent = self.play_from_hand_free(player, hand_index)?;
        Some((permanent, token))
    }

    /// Play the top card of `player`'s security stack **without paying
    /// memory**. Used by effects that say "play the top card of your
    /// security stack" (e.g. BT12-091; Phase 2f1 DSL `PlayFromSecurity`
    /// step lowerings).
    ///
    /// Distinct from [`Self::play_pending_security`] (the security-skill
    /// replay path that consumes the transient `Game.pending_security`
    /// during the attack-time security check). This method operates on the
    /// player's persistent `security` zone.
    ///
    /// ## Implementation strategy: hand-transit
    ///
    /// `Game::play_from_hand_with_cost(player, hand_index, CostDelta::Free)`
    /// already encapsulates the full placement path — battle-area capacity
    /// check, `CannotPlayDigimonByEffect` gate, `Permanent::new`, OnPlay
    /// trigger drain, OnEnterFieldAnyone broadcast, `Play` event emission.
    /// Re-introducing the placement body here would duplicate that logic
    /// and risk drift. Instead: pop the top of `player.security`, push it
    /// to the end of `player.hand`, and route through `play_from_hand_free`
    /// at that index. The card spends one tick in hand but never as a
    /// player-visible state — the hand is mutated and consumed inside this
    /// single method call before any selection prompt or event handler can
    /// observe it. The behavior is identical to the spec's suggested
    /// `place_card_in_battle_area` helper without forcing an engine-wide
    /// refactor of `play_from_hand_with_cost` to extract one.
    ///
    /// On rollback (battle area full, flood-gate, etc.) the card is
    /// restored to the top of `security` so this method is a clean no-op
    /// on failure — matching the precedent set by `play_from_hand_free`,
    /// which does not corrupt state on flood-gate-rejected plays.
    ///
    /// Also clears the popped card's entry from `face_up_security` —
    /// `face_up_security` is keyed by `card_index`, and a played card no
    /// longer lives in the security zone, so leaving the bit set would
    /// pollute the tensor's face-up bookkeeping.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if security is empty, the battle area is full, or the play was
    /// gated by a flood-gate.
    pub fn play_from_security(&mut self, player: PlayerId) -> Option<PermanentHandle> {
        let security_index = self
            .game
            .player(player)
            .security
            .iter()
            .position(|card| card.handle() == self.source_card)
            .or_else(|| self.game.player(player).security.len().checked_sub(1))?;
        self.play_from_security_index(player, security_index)
    }

    /// Play a SPECIFIC card from `player`'s security stack — identified by
    /// its `CardHandle` — without paying its cost. Used by DSL clauses that
    /// `select_security` a card and then play exactly that bound card
    /// (e.g. BT13-012 "search your security stack, and you may play 1 red
    /// or yellow Tamer card among it without paying its cost").
    ///
    /// Unlike `play_from_security` (which plays the trigger-context card or
    /// the security top), this resolves the security index of the bound
    /// handle and routes through the same `play_from_security_index`
    /// hand-transit path. Returns `None` if the handle is not in `player`'s
    /// security zone or the play is gated / fails.
    pub fn play_from_security_card(
        &mut self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        let security_index = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == card)?;
        self.play_from_security_index(player, security_index)
    }

    /// Play a specific card from the transient reveal pool without paying its
    /// memory cost. Used by effects like EX8-050 that reveal cards, allow one
    /// revealed card to be played, then move the remainder elsewhere.
    ///
    /// This mirrors the established security/material hand-transit strategy:
    /// remove the card from `revealed_cards`, park it at the end of `player`'s
    /// hand, and route through the normal play pipeline so floodgates,
    /// OnPlay, OnEnterField, and would-play replacement hooks stay aligned
    /// with every other effect-initiated play. The card is restored to the
    /// reveal pool if the play is immediately rejected or later cancelled.
    pub fn play_from_revealed_free(
        &mut self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        let reveal_index = self
            .game
            .revealed_cards
            .iter()
            .position(|revealed| revealed.handle() == card)?;
        let mut revealed = self.game.revealed_cards.remove(reveal_index);
        revealed.clear_reveal_overlay();

        self.game.player_mut(player).hand.push(revealed);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self.game.play_from_hand_with_cost_result_from_origin(
            player,
            hand_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            false,
            PendingWouldPlayOrigin::Reveal {
                index: reveal_index,
            },
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: revealed card was just pushed to hand");
                let insert_at = reveal_index.min(self.game.revealed_cards.len());
                self.game.revealed_cards.insert(insert_at, card);
                None
            }
        }
    }

    fn play_from_security_index(
        &mut self,
        player: PlayerId,
        security_index: usize,
    ) -> Option<PermanentHandle> {
        // Opaque-aware: materialize before play — a played card must
        // have a real data_index for cost/effect resolution to work.
        if security_index >= self.game.player(player).security.len() {
            return None;
        }
        self.game.ensure_security_materialized(player, security_index);
        let card = {
            let player_state = self.game.player_mut(player);
            if security_index >= player_state.security.len() {
                return None;
            }
            player_state.security.remove(security_index)
        };

        // `face_up_security` is keyed by card_index — clear it whether or
        // not the card was face-up; remove() is a no-op when absent.
        let card_index = card.card_index;
        let was_face_up = self
            .game
            .player_mut(player)
            .face_up_security
            .remove(&card_index);

        // Park at end of hand and play through the established hand-free
        // path. The hand index is the new last position.
        self.game.player_mut(player).hand.push(card);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self.game.play_from_hand_with_cost_result_from_origin(
            player,
            hand_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            false,
            PendingWouldPlayOrigin::SecurityTop { was_face_up },
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                // Rollback: pop the card back out of hand and restore it to
                // the top of security so the failure is observable as a
                // no-op. Restore face_up_security entry too in case the
                // caller depended on it.
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                // Note: we deliberately do NOT re-insert into
                // face_up_security on rollback — the card is back in the
                // security zone but its visibility-state was already
                // consumed by the abortive play attempt. Matches the
                // tradeoff `play_from_hand_with_cost` makes elsewhere on
                // gated rollbacks.
                let player_state = self.game.player_mut(player);
                let restore_at = security_index.min(player_state.security.len());
                player_state.security.insert(restore_at, card);
                None
            }
        }
    }

    /// Remove the source at `source_index` from `target`'s digivolution
    /// stack and play the underlying card into `target.player`'s battle
    /// area, deducting memory according to `cost_delta`. OnPlay effects
    /// fire as if the card had been played from hand.
    ///
    /// Card-text precedent: BT15-080 — "place this card's bottom material
    /// into battle area as a Digimon" (Phase 2f1 DSL `PlayFromMaterials`
    /// step lowering).
    ///
    /// ## Implementation strategy: hand-transit (mirrors `play_from_security`)
    ///
    /// `Game::play_from_hand_with_cost(player, hand_index, cost_delta)`
    /// already encapsulates the full placement path — battle-area capacity
    /// check, `CannotPlayDigimonByEffect` gate, `Permanent::new`, OnPlay
    /// trigger drain, OnEnterFieldAnyone broadcast, `Play` event emission.
    /// Re-introducing the placement body here would duplicate that logic
    /// and risk drift. Instead: pop the chosen `CardSource` out of
    /// `target`'s `card_sources`, push it to the end of the controller's
    /// `hand`, and route through `play_from_hand_with_cost` at that index.
    /// The card spends one tick in hand but never as a player-visible
    /// state — the hand is mutated and consumed inside this single method
    /// call before any selection prompt or event handler can observe it.
    /// Identical pattern to `play_from_security` (Task 3a).
    ///
    /// On rollback (battle area full, flood-gate, etc.) the source is
    /// restored to its **original index** in `target.card_sources` so the
    /// failure is observable as a no-op.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if `target` is invalid, `source_index` is out of bounds, the battle
    /// area is full, memory is insufficient, or the play was gated by a
    /// flood-gate.
    pub fn play_from_materials(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        self.play_from_materials_suppress_on_play(target, source_index, cost_delta, false)
    }

    pub fn play_from_materials_suppress_on_play(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        cost_delta: crate::enums::CostDelta,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            return self.play_from_breeding_materials_suppress_on_play(
                target,
                source_index,
                cost_delta,
                suppress_on_play,
            );
        }

        // Validate target permanent + source_index up-front using immutable
        // borrows.
        let player = target.player;
        {
            let p = self.game.player(player);
            let perm = p.battle_area.get(target.index as usize)?;
            if source_index >= perm.card_sources.len() {
                return None;
            }
        }

        // Extract the source. `Vec::remove` shifts subsequent sources down
        // one index — that's the desired behavior for material extraction
        // (the stack closes the gap left by the removed source).
        let source = self.game.player_mut(player).battle_area[target.index as usize]
            .card_sources
            .remove(source_index);

        // Park at the end of `player`'s hand and route through the standard
        // play-from-hand path. The hand index is the new last position.
        self.game.player_mut(player).hand.push(source);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self
            .game
            .play_from_hand_with_cost_result_from_origin_suppress(
                player,
                hand_index,
                cost_delta,
                PlaySource::ByEffect,
                false,
                PendingWouldPlayOrigin::Source {
                    permanent: target,
                    source_index,
                },
                suppress_on_play,
            ) {
            PlayFromHandCostResult::Played(field_index) => {
                let played = PermanentHandle {
                    player,
                    index: field_index as u8,
                };
                // Soft-remove the carrier slot if `play_from_materials` just
                // consumed its only source. Sibling of the digivolve-from-
                // material fix landed in PR #533. The carrier permanent has
                // empty `card_sources` post-extraction and would panic any
                // downstream `top_card()` reader; the helper drops the slot
                // and routes linked cards to trash per the same contract as
                // `Game::soft_remove_if_emptied`. See
                // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
                // `qa/archetype-qa/engine-gaps.md` (and the change
                // `fix-zombie-permanent-siblings`).
                //
                // The `played` handle is NOT affected by the soft-remove:
                // play_from_hand_with_cost_result_from_origin_suppress
                // pushes the new permanent at `battle_area.len()` AFTER the
                // source extraction, so any soft-remove of an earlier slot
                // has already shifted the played handle's index downward
                // and the returned `field_index` reflects the post-shift
                // position. But the soft-remove here happens AFTER the
                // play, removing the now-empty carrier, which may shift
                // `played.index` down by 1 if the carrier sat at a lower
                // index than the played permanent.
                let played = Self::shift_handle_after_soft_remove_check(self.game, target, played);
                Some(played)
            }
            PlayFromHandCostResult::Pending => {
                // Decision 2 in `design.md`: do NOT soft-remove on the
                // Pending branch. A parked selection may resume and either
                // commit the play (cleanup happens then via a separate
                // post-resume path) or fail (rollback restores the source
                // into the carrier). Soft-removing now would leave the
                // rollback path with no slot to restore into. The Layer 2
                // guards on `enqueue_from_permanent`,
                // `queued_effect_source_is_live`, and (via this change)
                // `find_event_gated_delay_permanent` /
                // `event_gated_delay_source` tolerate a transient zombie
                // carrier for the duration of the parked selection.
                None
            }
            PlayFromHandCostResult::Failed => {
                // Rollback: pop the card out of hand and reinsert it at
                // its original index in `target.card_sources` so the
                // failure is a clean no-op for callers. Soft-remove MUST
                // NOT have run before this — Decision 2 in `design.md`.
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                // The target permanent index is still valid here — only
                // hand was mutated by the failed play attempt; the
                // battle-area entry was left untouched.
                self.game.player_mut(player).battle_area[target.index as usize]
                    .card_sources
                    .insert(source_index, card);
                None
            }
        }
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

    fn play_from_breeding_materials_suppress_on_play(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        cost_delta: crate::enums::CostDelta,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let player = target.player;
        {
            let breeding = self.game.player(player).breeding_area.as_ref()?;
            if source_index >= breeding.card_sources.len()
                || source_index + 1 >= breeding.card_sources.len()
            {
                return None;
            }
        }

        let source = self
            .game
            .player_mut(player)
            .breeding_area
            .as_mut()?
            .card_sources
            .remove(source_index);

        self.game.player_mut(player).hand.push(source);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self
            .game
            .play_from_hand_with_cost_result_from_origin_suppress(
                player,
                hand_index,
                cost_delta,
                PlaySource::ByEffect,
                false,
                PendingWouldPlayOrigin::Source {
                    permanent: target,
                    source_index,
                },
                suppress_on_play,
            ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                self.game
                    .player_mut(player)
                    .breeding_area
                    .as_mut()?
                    .card_sources
                    .insert(source_index, card);
                None
            }
        }
    }

    /// Play a card from `player`'s trash at `trash_index`, deducting memory
    /// according to `cost_delta`. OnPlay effects fire.
    pub fn play_from_trash_with_cost(
        &mut self,
        player: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let field_index = self.game.play_from_trash_with_cost(
            player,
            trash_index,
            cost_delta,
            PlaySource::ByEffect,
        )?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    /// Play `card` from its controller's trash into the battle area, **without
    /// paying its memory cost** and **without suspending** the resulting
    /// permanent. ETB triggers (`OnPlay` + `OnEnterFieldAnyone`) fire as normal.
    ///
    /// ## Why a thin alias is sufficient (audit finding — Phase D Task 3)
    ///
    /// `Game::play_from_trash_with_cost(player, index, CostDelta::Free)` already
    /// covers all three requirements:
    ///   - **Free**: `CostDelta::Free` resolves to 0 → `pay_memory(0)` is a
    ///     no-op; memory is unchanged.
    ///   - **Unsuspended**: `Permanent::new()` sets `is_suspended = false` by
    ///     default; no extra flag needed.
    ///   - **ETB active**: `fire_on_play` + `OnEnterFieldAnyone` run at the end
    ///     of `play_from_trash_with_cost`, exactly as for hand plays.
    ///
    /// The only gap bridged here is the call-site convenience: callers hold a
    /// `CardHandle` (stable across zone moves), not a positional `trash_index`.
    /// This method locates the card in the controller's trash by handle.
    ///
    /// Returns `None` if the card is not in the controller's trash at call
    /// time (e.g., if another effect moved it elsewhere). This is the
    /// defensive behavior; callers like the deferred-replay drain in
    /// `combat::finalize_permanent_deletion` absorb `None` silently. The
    /// concrete failure mode this guards against: a `<Save>` + `<Fortitude>`
    /// interaction where Save relocates the card under a Tamer between
    /// Fortitude's queueing of the replay and the drain hook firing — at
    /// which point the card is no longer in trash and replaying it would
    /// panic.
    ///
    /// DCGO parity: `Fortitude.cs:54-63`
    ///   `PlayPermanentCards(payCost: false, isTapped: false,
    ///    root: SelectCardEffect.Root.Trash, activateETB: true)`
    ///
    /// Used by: `<Fortitude>` keyword auto-install (Phase D Task 8).
    pub fn play_from_trash_free_unsuspended(
        &mut self,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.play_from_trash_free_unsuspended_inner(card, false)
    }

    /// As [`Self::play_from_trash_free_unsuspended`], but suppresses the
    /// played Digimon's own `[On Play]` effects for this play event only
    /// (PUPPETS-G030). Used by BT5-106's [Security] clause — "Any [On Play]
    /// effects on Digimon played with this effect don't activate." The
    /// suppression is scoped strictly to the just-played permanent and this
    /// single play; other permanents' On Play and every other timing
    /// (`OnEnterFieldAnyone` / `OnAllyPlayed`) fire normally.
    pub fn play_from_trash_free_unsuspended_suppress_on_play(
        &mut self,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.play_from_trash_free_unsuspended_inner(card, true)
    }

    fn play_from_trash_free_unsuspended_inner(
        &mut self,
        card: CardHandle,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let controller = self.player;
        let trash_index = self
            .game
            .player(controller)
            .trash
            .iter()
            .position(|c| c.handle() == card);
        let trash_index = match trash_index {
            Some(i) => i,
            None => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[debug] play_from_trash_free_unsuspended: card {:?} not in \
                     player {}'s trash; another effect likely relocated it. \
                     Skipping replay.",
                    card, controller
                );
                return None;
            }
        };
        let field_index = self.game.play_from_trash_with_cost_suppress(
            controller,
            trash_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            suppress_on_play,
        )?;
        Some(PermanentHandle {
            player: controller,
            index: field_index as u8,
        })
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. See
    /// `Game::place_as_bottom_source`.
    ///
    /// **Phase B Progress gate — intentionally omitted.** DCGO's primitive
    /// `Permanent.AddDigivolutionCardsBottom` does not consult
    /// `CanNotBeAffected` on the receiving permanent, and DCGO scripts that
    /// place a source under an opponent's Digimon (e.g. EX10-059) do not
    /// gate the target on Progress either. Adding a card under a stack is
    /// not "affecting" the target in DCGO's semantics — the TopCard's
    /// status is unchanged. Gating here would over-restrict relative to
    /// DCGO and break parity with cards that intentionally route a source
    /// under an opponent's Progress attacker.
    ///
    /// `face_down: true` marks the inserted digivolution source face-down
    /// (Tamer-stash callers); `false` is ordinary face-up placement. Note
    /// that `face_down` is NOT honored for `CardSourceRef::Security` sources
    /// — those are always placed face-up (see the `Game::place_as_bottom_source`
    /// doc caveat).
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        self.game
            .place_as_bottom_source_observed(source, target, self.player, face_down)
    }

    pub fn place_permanent_as_bottom_sources(
        &mut self,
        source: PermanentHandle,
        target: PermanentHandle,
    ) -> bool {
        self.game.place_permanent_as_bottom_sources(source, target)
    }

    /// Move `target`'s **top stacked card** (the card immediately beneath its
    /// active top card — `card_sources[len - 2]`) to the bottom of its own
    /// digivolution stack. Returns `false` if the target has no stacked card
    /// beneath the top (i.e. `card_sources.len() < 2`), in which case the
    /// printed-cost cannot be paid; callers should gate the activating step
    /// with a `materials_count_gte: 1` (or equivalent) predicate.
    ///
    /// Closes `G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM` for the deterministic
    /// "top stacked card → bottom" cost shape that DCGO encodes as
    /// `card.PermanentOfThisCard().TopCard` followed by
    /// `AddDigivolutionCardsBottom`. Per the no-approximations policy this
    /// path does NOT surface a player choice over which source moves — the
    /// printed text identifies a singular "top" source.
    pub fn place_top_source_as_bottom(&mut self, target: PermanentHandle) -> bool {
        let stack_size = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|p| p.card_sources.len())
            .unwrap_or(0);
        if stack_size < 2 {
            return false;
        }
        let top_stacked_idx = stack_size - 2;
        self.place_as_bottom_source(
            crate::enums::CardSourceRef::Material(target, top_stacked_idx),
            target,
            false,
        )
    }

    /// Move `player`'s real breeding permanent into the battle area by effect.
    pub fn move_from_breeding_by_effect(&mut self, player: PlayerId) -> bool {
        self.game.move_from_breeding_by_effect(player)
    }

    /// Play/place `player`'s hand card into their real breeding area by effect.
    pub fn play_to_breeding_from_hand(&mut self, player: PlayerId, hand_index: usize) -> bool {
        self.game.play_to_breeding_from_hand(player, hand_index)
    }

    /// Move `card` from whatever zone it currently occupies to the **bottom**
    /// of `target`'s digivolution stack (`card_sources[0]`).
    ///
    /// The card is located by scanning all zones of all players in the
    /// following priority order:
    ///   1. Each player's `hand`
    ///   2. Each player's `trash`
    ///   3. Each player's `deck`
    ///   4. Each player's `security`
    ///   5. Each player's `battle_area` card stacks (all permanents)
    ///   6. Each player's `breeding_area` card stack
    ///   7. The game-level `revealed_cards` transient pool
    ///
    /// This covers every realistic source for `<Save>` (self just moved to
    /// trash during deletion) and `<Material Save N>` (cards are in another
    /// permanent's `card_sources`). Opponent deck / security are included for
    /// completeness but Save/MaterialSave callers will never route through
    /// them in normal play. `revealed_cards` is included to handle cards that
    /// are mid-reveal when a Save effect resolves.
    ///
    /// `face_down: true` marks the placed source face-down; `false` is
    /// ordinary face-up placement.
    ///
    /// # Panics
    ///
    /// Panics if `card` cannot be located in any zone — this represents a
    /// programming error (passing an invalid or already-moved handle).
    ///
    /// Used by: `<Save>`, `<Material Save N>`.
    pub fn place_card_under_permanent_bottom(
        &mut self,
        card: CardHandle,
        target: PermanentHandle,
        face_down: bool,
    ) {
        let mut taken = self
            .game
            .remove_card_from_any_zone(card)
            .unwrap_or_else(|| {
                panic!(
                    "place_card_under_permanent_bottom: card {:?} not found in any zone",
                    card
                )
            });

        let target_player = self.game.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            // Safe-fail: target permanent no longer exists; route to its
            // controller's trash rather than dropping the card on the floor.
            // Trashed cards are not face-down sources, so `face_down` is not
            // applied on this path.
            target_player.trash.push(taken);
            return;
        }
        taken.face_down = face_down;
        target_player.battle_area[target.index as usize].push_under(taken);
    }

    /// Place `tamer`'s top card at the bottom of `digimon`'s digivolution
    /// stack, replicating DCGO `MindLink.cs:71-79`:
    /// `IPlacePermanentToDigivolutionCards(new[] { tamer, selectedDigimon })`.
    ///
    /// The Tamer permanent itself is removed from battle area; its top
    /// CardSource becomes the new bottom of the target Digimon's stack.
    /// The face-down flag is NOT set (MindLink places face-up).
    ///
    /// ## DCGO parity (CardController.cs:3011-3061)
    ///
    /// `IPlacePermanentToDigivolutionCards.PlacePermanentToDigivolutionCards`
    /// takes `cardSource = DigivolutionPermanent.TopCard` (just the top),
    /// calls `DiscardEvoRoots()` on the rest of the stack (sending those
    /// sources to trash), removes the source permanent from field, then
    /// calls `getDigivolutionPermanent.AddDigivolutionCardsBottom(...)` to
    /// tuck the top card under the target.
    ///
    /// ## Index-stability strategy
    ///
    /// Removing `tamer` from `battle_area` shifts every higher-indexed
    /// permanent down by one. To keep the `digimon` handle valid we
    /// resolve `digimon`'s slot AFTER the removal (adjusting if it was
    /// past `tamer.index`). Both must share the same controller (DCGO
    /// `IsPermanentExistsOnOwnerBattleArea`); we assert that, since the
    /// MindLink filter already enforces it.
    ///
    /// ## Modifier cleanup
    ///
    /// `Game.modifiers.clear_permanent(tamer)` and
    /// `expire_player_on_permanent_leave(tamer)` are invoked before the
    /// removal, mirroring `Game::return_to_deck`'s and the deletion
    /// finalize's cleanup pattern. Player-scoped modifiers sourced from
    /// the Tamer (e.g., printed memory-gain effects) expire here.
    ///
    /// ## Source-stack handling
    ///
    /// Sources below the top of the Tamer's stack are routed to trash
    /// (mirroring DCGO `DiscardEvoRoots` — sources can't ride the move).
    /// Each such trash fires `OnDigivolutionCardTrashed` per player and
    /// drains the queue, mirroring `Game::return_to_deck`'s leave-field
    /// path. Linked cards on the Tamer (Tamers can host Option cards via
    /// `<Linked>`) are likewise trashed; if any were present, a single
    /// `OnLinkedCardTrashed` is fired per player and drained, mirroring
    /// `Game::finalize_permanent_deletion`'s linked-cascade pattern.
    ///
    /// Used by: `<Mind Link>` keyword auto-install (Phase F Task 5).
    pub fn attach_tamer_to_digimon(&mut self, tamer: PermanentHandle, digimon: PermanentHandle) {
        // Validate: shared controller (MindLink targets own permanents).
        // Promoted to `assert_eq!` so the precondition is enforced in
        // release builds too — this is a public primitive, and a release
        // caller violating it would silently misroute trash and target
        // lookup rather than panicking.
        assert_eq!(
            tamer.player, digimon.player,
            "attach_tamer_to_digimon: tamer and target must share a controller"
        );
        let controller = tamer.player;

        // Bounds check the tamer slot.
        let tamer_idx = tamer.index as usize;
        if tamer_idx >= self.game.player(controller).battle_area.len() {
            return;
        }

        // Cleanup tamer-scoped modifiers BEFORE removal (modifier registry
        // is keyed on PermanentHandle, which becomes invalid after the
        // index shift caused by `Vec::remove`).
        self.game.modifiers.clear_permanent(tamer);
        self.game.modifiers.expire_player_on_permanent_leave(tamer);

        // Remove the Tamer permanent from battle area. This shifts indices
        // for all higher-indexed permanents on the same player down by 1.
        let mut tamer_perm = self
            .game
            .player_mut(controller)
            .battle_area
            .remove(tamer_idx);

        // Trash any sources below the top (DCGO DiscardEvoRoots). The top
        // is the card that rides under the target.
        let Some(top) = tamer_perm.card_sources.pop() else {
            return;
        };
        // Below-top sources go to trash. Linked Option cards (Tamers can
        // host them via `<Linked>`) likewise go to trash — they cannot
        // ride the host into another permanent's digivolution stack.
        //
        // Mirror `Game::return_to_deck` (game_actions.rs:1497-1506): fire
        // `OnDigivolutionCardTrashed` per source, per player, draining
        // between each so observers see them one at a time.
        for source in tamer_perm.card_sources.drain(..) {
            let source_card = source.handle();
            self.game.player_mut(controller).trash.push(source);
            self.game.fire_digivolution_card_trashed(
                controller,
                tamer,
                top.handle(),
                source_card,
                crate::trigger_context::EventCause::Return,
            );
        }
        // Mirror `Game::finalize_permanent_deletion` (combat.rs:2429-2437):
        // route all linked cards to trash, then fire a single
        // `OnLinkedCardTrashed` per player if any were present.
        let had_linked = !tamer_perm.linked_cards.is_empty();
        for linked in tamer_perm.linked_cards.drain(..) {
            self.game.player_mut(controller).trash.push(linked);
        }
        if had_linked {
            for pid in 0..self.game.players.len() {
                self.game.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            self.game.drain_effect_queue();
        }

        // Resolve the target's NEW slot after the index shift. If the
        // digimon was at a higher index than the removed tamer, its
        // slot dropped by 1; otherwise it's unchanged.
        let digimon_idx = if (digimon.index as usize) > tamer_idx {
            (digimon.index as usize) - 1
        } else {
            digimon.index as usize
        };

        // Bounds check the (possibly shifted) target slot. If the target
        // is gone, route the lifted top card to the controller's trash
        // (safe-fail — same shape as `place_card_under_permanent_bottom`).
        let target_player = self.game.player_mut(controller);
        if digimon_idx >= target_player.battle_area.len() {
            target_player.trash.push(top);
            return;
        }
        // Insert at the bottom of the target's stack. `face_down` stays
        // `false` (MindLink places face-up).
        target_player.battle_area[digimon_idx].push_under(top);
    }

    /// Trash the current top Digimon of `perm` and promote the next-highest
    /// digivolution source to become the new visible top. The remainder of the
    /// stack is preserved intact.
    ///
    /// **Caller MUST gate on `perm.card_sources.len() >= 2` before invoking.**
    /// This primitive `debug_assert!`s that constraint and will panic in debug
    /// builds if violated. Production callers (the ArmorPurge auto-install
    /// body) gate before calling.
    ///
    /// After the call:
    ///   - `perm.card_sources.len()` decreases by 1.
    ///   - The previous `card_sources[last - 1]` entry (previously the
    ///     next-highest source) is now `top_card()`.
    ///   - The trashed top card is appended to `players[controller].trash`.
    ///   - `EffectTiming::OnDigivolutionCardTrashed` is enqueued and drained
    ///     so observers (e.g. Rocks-archetype source-trash listeners) see the
    ///     trashed top. DCGO parity: `ArmorPurge.cs:65-78` —
    ///     `StackSkillInfos(hashtable, EffectTiming.WhenTopCardTrashed)`.
    ///
    /// **Modifier note:** Modifiers in this engine are keyed by
    /// `PermanentHandle` (the full permanent), not by individual
    /// `CardSource`. Therefore, any modifiers currently attached to this
    /// permanent handle remain valid for the new top card — no modifier
    /// cleanup is performed here. This deviates from DCGO
    /// `RemoveDigivolveRootEffect` (which removes inherited effects registered
    /// by the trashed card specifically), but is the correct behavior for this
    /// engine because inherited effects are script-driven, not stored as
    /// `ModifierEntry` values.
    ///
    /// DCGO parity: `ArmorPurge.cs:50-65`
    ///   `RemoveFromAllArea(topCard)` + `AddTrashCard(topCard)` +
    ///   `RemoveDigivolveRootEffect(topCard, _permanent)`.
    ///
    /// Used by: `<Armor Purge>` keyword auto-install (Phase D Task 5).
    pub fn armor_purge_top(&mut self, perm: PermanentHandle) {
        let permanent = self
            .game
            .player_mut(perm.player)
            .battle_area
            .get_mut(perm.index as usize)
            .expect("armor_purge_top: permanent handle is invalid");
        debug_assert!(
            permanent.card_sources.len() >= 2,
            "armor_purge_top requires >= 1 source under the top card (stack len = {})",
            permanent.card_sources.len()
        );
        let top = permanent
            .card_sources
            .pop()
            .expect("len >= 2 invariant asserted above");
        let top_handle = top.handle();
        // The new top is now `permanent.card_sources.last()` automatically —
        // no extra work needed (previous next-highest is now visible).
        let controller = perm.player;
        self.game.player_mut(controller).trash.push(top);
        // Modifier cleanup: no per-card-source tracking exists in this engine;
        // permanent-handle modifiers remain valid for the promoted top card.
        // See doc comment above for the DCGO deviation note.

        // Fire OnDigivolutionCardTrashed for the trashed top card. Mirrors the
        // existing dispatch in `Game::return_to_hand` /
        // `Game::return_to_deck` (game_actions.rs:1345-1357 / 1481-1493) for
        // sources-below-top, and matches DCGO `ArmorPurge.cs:65-78` which
        // re-stacks `EffectTiming.WhenTopCardTrashed` after the trash. We
        // pick it up.
        self.game.fire_digivolution_card_trashed(
            controller,
            perm,
            self.game
                .player(perm.player)
                .battle_area
                .get(perm.index as usize)
                .map(|permanent| permanent.top_card().handle())
                .unwrap_or(top_handle),
            top_handle,
            crate::trigger_context::EventCause::Cost,
        );
    }

    /// `<Training>` (Phase F §F4 / RULES_CONTEXT 16-40 / DCGO `Training.cs:30`)
    /// helper: pop the controller's deck top and append it at the BOTTOM of
    /// `perm`'s digivolution stack (`card_sources[0]`), marked face-down.
    ///
    /// Empty-deck case: silent no-op. The Rust port chooses safer behavior
    /// than DCGO's `LibraryCards[0]` raw indexing — DCGO never reaches the
    /// indexing line in practice because the `SetUpActivateClass` framework
    /// only calls in once activation is committed; the Rust version accepts
    /// the activation, pays the suspend cost in the calling effect, and
    /// silently no-ops the card move when there's nothing to draw. Mirrors
    /// the documented "no-op on empty source" pattern in `Player::draw`.
    ///
    /// `perm` may be either a battle-area or breeding-area permanent of
    /// the controller; this helper does not enforce zone — the caller's
    /// activation gate (carrier-not-suspended) handles eligibility. The
    /// breeding-area branch is a separate `as_mut()` lookup since
    /// `breeding_area: Option<Permanent>` is not in `battle_area`.
    ///
    /// The new source carries `face_down=true` (mirrors DCGO
    /// `isFacedown: true`); face-down sources are filtered out of the
    /// `<Mind Link>` "no Tamer source" gate (DCGO `MindLink.cs:25`
    /// `!cardSource.IsFlipped`).
    ///
    /// Used by: `<Training>` keyword auto-install (Phase F Task 6).
    pub fn training_place_deck_top_under_self_face_down(&mut self, perm: PermanentHandle) {
        // Pop the controller's deck top. Empty-deck case is a silent no-op.
        // Opaque-aware: opaque opponents materialize from RevealSource
        // tagged Effect (Training peeks at top and re-routes — not draw/mill/security).
        let owner = perm.player;
        let mut card = match self.game.take_from_deck_top_for_player(
            owner,
            crate::opaque_deck::RevealKind::Effect,
        ) {
            Ok(Some(c)) => c,
            Ok(None) => return,
            Err(e) => {
                eprintln!(
                    "[opaque-deck] training_place_deck_top_under_self_face_down \
                     error for player {}: {}",
                    owner, e
                );
                return;
            }
        };
        // Mark face-down — DCGO `AddDigivolutionCardsBottom(..., isFacedown: true)`.
        card.face_down = true;

        // Locate the carrier in battle area; if it's not there, look in
        // breeding area. (Breeding-area permanents never co-exist with a
        // same-handle battle-area slot — `move_from_breeding` takes the
        // Option, so the disjoint check holds.)
        let player = self.game.player_mut(owner);
        if let Some(p) = player.battle_area.get_mut(perm.index as usize) {
            // Insert at bottom of stack (index 0).
            p.card_sources.insert(0, card);
            return;
        }
        if let Some(ref mut breeding) = player.breeding_area {
            breeding.card_sources.insert(0, card);
            return;
        }
        // Carrier no longer exists in either zone (defensive — the calling
        // effect's `condition` gates on `source_permanent()`, which already
        // requires the carrier to be live). Drop the card on the floor
        // rather than misroute; in practice unreachable.
    }

    /// Place the top card of `target.player`'s deck as the bottom digivolution
    /// source of `target`. Generalizes `training_place_deck_top_under_self_face_down`
    /// to an arbitrary target permanent (Tamer or Digimon, in either player's
    /// battle area).
    ///
    /// Returns `Some(card_handle)` on success or `None` if the controller's
    /// deck is empty (silent no-op on empty deck, mirroring `Player::draw`).
    ///
    /// `face_down: true` marks the placed source face-down (Tamer-stash
    /// callers); `false` is ordinary face-up placement.
    ///
    /// Used by: ST-23 BEATBREAK / ST-24 DATA SQUAD Tamer-stash placement cards
    /// (e.g. ST23-13 Tomoro Tenma & Kyo Sawashiro, ST24-09 Sunflowmon).
    pub fn place_deck_top_under_permanent(
        &mut self,
        target: PermanentHandle,
        face_down: bool,
    ) -> Option<CardHandle> {
        let card_handle = self.game.player(target.player).deck.last()?.handle();
        let ok = self.game.place_as_bottom_source_observed(
            crate::enums::CardSourceRef::DeckTop(target.player),
            target,
            self.player,
            face_down,
        );
        if ok {
            Some(card_handle)
        } else {
            None
        }
    }

    /// Trash a specific digivolution source from a permanent.
    ///
    /// Used by:
    /// - `<Fragment (N)>` keyword auto-install (Phase D Task 4) — picked sources
    ///   are trashed as the cancel-deletion cost.
    /// - `<Partition>` keyword auto-install (Phase D Task 9) — sources are
    ///   moved out of the deleted permanent's stack.
    /// - Hand-authored card scripts that "trash a digivolution card" as a cost
    ///   or effect.
    ///
    /// The card is removed from `perm.card_sources` (anywhere in the stack —
    /// not just the top — see `armor_purge_top` for the top-card-only variant)
    /// and pushed to the card owner's trash.
    ///
    /// Returns `true` iff the card was actually trashed. Returns `false` (no
    /// mutation, no observer dispatch, no `soft_remove` side-effects) for any
    /// rules-natural fizzle:
    ///   - the carrier slot is gone (soft-removed or deleted by a sibling
    ///     effect between handle capture and this call);
    ///   - the carrier exists but `card_sources` is empty (zombie slot
    ///     mid-cleanup — also avoids the `top_card()` panic on empty stack);
    ///   - the carrier's `card_sources` does not contain `card` (the captured
    ///     handle was invalidated by an intervening observer that trashed,
    ///     returned, or extracted the source).
    ///
    /// This soft-fail contract mirrors DCGO
    /// `ITrashDigivolutionCards.TrashDigivolutionCards()` (see
    /// `DCGO/Assets/Scripts/Script/CardController.cs:5181`): the trash
    /// primitive is declarative ("trash these if possible") and the outcome
    /// is observable from the return value. Callers that need to branch on
    /// the actually-trashed set check this bool; callers that don't care
    /// (e.g., the DSL `TrashSelectedSources` loop and the engine-side
    /// pre-validated helpers `<Fragment>` install, `trash_all_sources`,
    /// `trash_top_n_digivolution_cards_of_each`) discard it with `let _ =`.
    /// See change `fix-trash-card-source-stale-handle` and panic family
    /// `G-DSL-TRASH-SOURCES-STALE-HANDLE` in
    /// `qa/archetype-qa/panic-families.json`.
    ///
    /// Token cards (`is_token == true`) are still pushed to trash; the
    /// caller's gate is responsible for any token-aware filtering.
    pub fn trash_card_source(&mut self, perm: PermanentHandle, card: CardHandle) -> bool {
        let (removed, host_card) = {
            // Soft-fail: carrier missing (DCGO: `if (_permanent == null) yield break;`).
            let permanent = match self
                .game
                .player_mut(perm.player)
                .battle_area
                .get_mut(perm.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            // Soft-fail: empty stack (DCGO: `HasNoDigivolutionCards` yield-break).
            // Guards the `top_card()` call below from panicking on an empty stack.
            if permanent.card_sources.is_empty() {
                return false;
            }
            let host_card = permanent.top_card().handle();
            // Soft-fail: card not in stack (DCGO: target dropped by
            // `_trashTargetCards.Filter(c => _permanent.DigivolutionCards.Contains(c))`).
            let pos = match permanent
                .card_sources
                .iter()
                .position(|c| c.handle() == card)
            {
                Some(p) => p,
                None => return false,
            };
            (permanent.card_sources.remove(pos), host_card)
        };
        let source_card = removed.handle();
        let owner = removed.owner;
        self.game.player_mut(owner).trash.push(removed);
        self.game.fire_digivolution_card_trashed(
            perm.player,
            perm,
            host_card,
            source_card,
            crate::trigger_context::EventCause::from(self.game.infer_effect_cause(perm.player)),
        );
        // Soft-remove the carrier slot if the trash emptied it. Sibling
        // of the digivolve-from-material fix landed in PR #533. The
        // `fire_digivolution_card_trashed` observer dispatch above runs
        // BEFORE the slot removal, so observers see the source-trash
        // event attributed to the correct host. See
        // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
        // `qa/archetype-qa/engine-gaps.md`.
        if !self.game.soft_remove_if_emptied(perm) {
            let _ = self.cleanup_exposed_battle_area_digi_egg(perm);
        }
        true
    }

    /// Remove a single digivolution source card from `perm`'s stack and route
    /// it to its **owner's** hand. Mirrors `trash_card_source` but the
    /// destination is the owner's hand, not trash.
    ///
    /// Used by card effects that say "By returning N [card] from this
    /// Digimon's digivolution cards to its owner's hand" (e.g. BT12-031's
    /// Imperialdramon: Dragon Mode alt-cost).
    ///
    /// Owner-routing: the card is pushed to `removed.owner`'s hand (the
    /// `CardSource.owner` field), NOT the controller's — so a source owned by
    /// the opponent (rare, via control-transfer plays) returns to its true
    /// owner. `trash_card_source` reads `removed.owner` for the same reason.
    ///
    /// Stack invariant: the source is removed by `position(...)` (anywhere in
    /// the stack, not just the top), preserving the host permanent's top card
    /// and the ordering of the remaining sources.
    ///
    /// Observer dispatch: this is a *return-to-hand*, NOT a trash, so it does
    /// **not** fire `OnDigivolutionCardTrashed` (which would mis-attribute the
    /// move to source-trash listeners). No trash-specific observer fires.
    ///
    /// Returns `true` when the source handle was found and moved; `false` if
    /// the permanent slot is gone or the card is not in its stack.
    pub fn return_card_source_to_hand(&mut self, perm: PermanentHandle, card: CardHandle) -> bool {
        let removed = {
            let permanent = match self
                .game
                .player_mut(perm.player)
                .battle_area
                .get_mut(perm.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            let pos = match permanent
                .card_sources
                .iter()
                .position(|c| c.handle() == card)
            {
                Some(pos) => pos,
                None => return false,
            };
            permanent.card_sources.remove(pos)
        };
        let owner = removed.owner;
        self.game.player_mut(owner).hand.push(removed);
        let _ = self.cleanup_exposed_battle_area_digi_egg(perm);
        true
    }

    /// `Vec`-taking convenience wrapper over `return_card_source_to_hand`,
    /// keeping parity with `play_selected_sources_without_cost`. Each selected
    /// source ref is returned to its owner's hand. Returns `true` if every
    /// ref was successfully moved.
    pub fn return_selected_sources_to_hand(&mut self, selected: Vec<SourceSelectionRef>) -> bool {
        let mut all_ok = true;
        for source_ref in selected {
            if !self.return_card_source_to_hand(source_ref.permanent, source_ref.card) {
                all_ok = false;
            }
        }
        all_ok
    }

    /// Trash every digivolution source below `target`'s top card, preserving
    /// the live permanent and dispatching source-trash observers per source.
    pub fn trash_all_sources(&mut self, target: PermanentHandle) -> bool {
        let Some(permanent) = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        let source_handles: Vec<CardHandle> = permanent
            .card_sources
            .iter()
            .take(permanent.card_sources.len().saturating_sub(1))
            .map(|source| source.handle())
            .collect();

        for source in source_handles {
            let Some(permanent) = self
                .game
                .player(target.player)
                .battle_area
                .get(target.index as usize)
            else {
                return true;
            };
            let still_below_top = permanent
                .card_sources
                .iter()
                .take(permanent.card_sources.len().saturating_sub(1))
                .any(|candidate| candidate.handle() == source);
            if still_below_top {
                // Bool discarded: pre-validated above; no caller branches
                // on outcome (return is `true` regardless of per-source success).
                let _ = self.trash_card_source(target, source);
            }
        }
        true
    }

    /// Strip the top digivolution source from `target`'s stack and route the
    /// underlying card to its owner's trash. Returns `true` on
    /// success; `false` if the target handle is invalid or the stack is empty.
    ///
    /// Used by card effects that say "trash the top digivolution source of
    /// this Digimon" — the bool-returning, gate-friendly counterpart to
    /// `armor_purge_top` (which is reserved for the `<Armor Purge>` keyword
    /// auto-install body and panics on insufficient stack).
    ///
    /// Mirrors `armor_purge_top`'s observer dispatch: after the trash, fires
    /// `OnDigivolutionCardTrashed` and drains the queue, so observers
    /// (e.g. Rocks-archetype source-trash listeners) see the trashed top with
    /// source/host event context. The trashed card moves through the standard
    /// owner trash path.
    ///
    /// Reject-before-mutate discipline: invalid handle and empty stack both
    /// return `false` before any state change.
    pub fn trash_top_source(&mut self, target: PermanentHandle) -> bool {
        // Track C / D consult site (2026-05-08): `ImmuneFromStackTrashing`
        // on the host permanent blocks the inherited stack-peel mutation.
        // Distinct from `CannotBeDestroyed` (which protects the live top
        // card from deletion) — this protects the digivolution sources
        // sitting beneath the top from being peeled off and trashed.
        if self
            .game
            .modifiers
            .has(target, ModifierType::ImmuneFromStackTrashing)
        {
            return false;
        }
        // Validate target slot.
        let (removed, host_card) = {
            let permanent = match self
                .game
                .player_mut(target.player)
                .battle_area
                .get_mut(target.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            // Pop top of card_sources; bail clean if empty.
            let removed = match permanent.card_sources.pop() {
                Some(s) => s,
                None => return false,
            };
            let host_card = permanent
                .card_sources
                .last()
                .map(|card| card.handle())
                .unwrap_or_else(|| removed.handle());
            (removed, host_card)
        };
        let source_card = removed.handle();
        let owner = removed.owner;
        self.game.player_mut(owner).trash.push(removed);

        // Fire OnDigivolutionCardTrashed for the trashed top card. Mirrors
        // `armor_purge_top` (effect_context/mod.rs:~1604) and the
        // sources-below-top dispatch in `Game::return_to_hand` /
        // `Game::return_to_deck`. Enqueue once per player so observers on
        // either side of the field pick it up.
        self.game.fire_digivolution_card_trashed(
            target.player,
            target,
            host_card,
            source_card,
            crate::trigger_context::EventCause::from(self.game.infer_effect_cause(target.player)),
        );
        let _ = self.cleanup_exposed_battle_area_digi_egg(target);
        true
    }

    /// Trash the bottom-most face-down digivolution source from `target`,
    /// fire `OnDigivolutionCardTrashed`, and drain the observer queue. Returns
    /// `true` iff a face-down source was found at `card_sources[0]` and
    /// trashed; returns `false` with no mutation otherwise (no face-down
    /// bottom source, or `target` missing).
    ///
    /// This is the cost-form trash primitive for ST-23 BEATBREAK and ST-24
    /// DATA SQUAD cards whose printed text reads "by trashing the bottom
    /// face-down card from under any of your Tamers, ...".
    ///
    /// The trashed source routes to the source's own `owner` trash, matching
    /// the standard `OnDigivolutionCardTrashed` ownership semantics (mirrors
    /// `trash_card_source` / `trash_top_source`).
    ///
    /// Unlike `trash_top_source`, this helper does NOT honor
    /// `ImmuneFromStackTrashing`: that modifier guards against involuntary
    /// stack-peeling by opponent effects, whereas this is a voluntary
    /// activation cost the controller chooses to pay (the controller earlier
    /// stashed the face-down card under their own Tamer). The omission is by
    /// design — do not add the check.
    ///
    /// Used by: ST23-01 Kekkomon, ST23-03 Cougarmon, ST23-04 Murasamemon,
    /// ST23-08 Monarchlizamon, ST23-11 Wolvermon, ST23-12 Chiropmon,
    /// ST24-01 Koromon, ST24-06 RizeGreymon, ST24-10 Lilamon, ST24-11 Rosemon,
    /// ST24-12 Falcomon.
    pub fn trash_bottom_face_down_source(&mut self, target: PermanentHandle) -> bool {
        // Inspect `card_sources[0]`: it must exist AND be face-down. Reject
        // before any mutation otherwise (missing target, empty stack, or a
        // face-up bottom source — e.g. an un-stashed Tamer whose only source
        // is its own face-up card).
        let removed = {
            let permanent = match self
                .game
                .player_mut(target.player)
                .battle_area
                .get_mut(target.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            match permanent.card_sources.first() {
                Some(bottom) if bottom.face_down => {}
                _ => return false,
            }
            permanent.card_sources.remove(0)
        };
        let source_card = removed.handle();
        let owner = removed.owner;
        self.game.player_mut(owner).trash.push(removed);

        // Compute the host's CURRENT top card AFTER the removal — the
        // permanent still exists with its remaining sources / its own top
        // card. A Tamer always retains its own card as the top.
        //
        // The direct index (not `.get()`) is infallible by construction here:
        // the permanent was validated present at the top of this function, and
        // only a source — never the permanent — was removed.
        let host_card = self.game.player(target.player).battle_area[target.index as usize]
            .top_card()
            .handle();

        // Fire OnDigivolutionCardTrashed for the trashed bottom source, the
        // same observer dispatch as `trash_card_source` / `trash_top_source`.
        self.game.fire_digivolution_card_trashed(
            target.player,
            target,
            host_card,
            source_card,
            crate::trigger_context::EventCause::from(self.game.infer_effect_cause(target.player)),
        );
        true
    }

    pub fn play_selected_sources_without_cost(
        &mut self,
        selected: Vec<SourceSelectionRef>,
    ) -> bool {
        self.game
            .play_source_refs_from_effect_without_cost(selected)
    }

    /// Bounce a permanent to its owner's hand. See `Game::return_to_hand`.
    pub fn return_to_hand(
        &mut self,
        target: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        if !self.can_affect_permanent(target) {
            return None;
        }
        self.game.return_to_hand(target)
    }

    /// Return the resolving effect's own permanent (`self.source_permanent`)
    /// to its owner's hand. Sugar over `return_to_hand` for printed text like
    /// "return this Digimon to your hand". Returns the moved card's handle
    /// on success, `None` if the effect has no source permanent (e.g. an
    /// Option-card effect or a rule-source effect) or if the bounce is
    /// blocked by `CannotBeReturnedToHand` / `CannotBeAffected` modifiers.
    ///
    /// Owner-routed: `Game::return_to_hand` reads the moved card's `owner`
    /// field, so a permanent owned by player A but currently controlled by
    /// player B (e.g. via a control-transfer effect) returns to A's hand.
    pub fn bounce_self(&mut self) -> Option<crate::card_source::CardHandle> {
        let handle = self.source_permanent?;
        self.return_to_hand(handle)
    }

    /// Return a permanent's top card to its owner's deck. See `Game::return_to_deck`.
    pub fn return_to_deck(
        &mut self,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        if !self.can_affect_permanent(target) {
            return false;
        }
        self.game.return_to_deck(target, position)
    }

    /// Return a permanent's full stack to its owner's deck. See
    /// `Game::return_stack_to_deck`.
    pub fn return_stack_to_deck(
        &mut self,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        if !self.can_affect_permanent(target) {
            return false;
        }
        self.game.return_stack_to_deck(target, position)
    }

    /// Digivolve a card from `player`'s hand at `hand_index` onto `target`
    /// by effect. Bypasses the Main-phase check; optionally ignores color
    /// requirements (`ignore_color=true`); pays memory via `cost_delta`.
    ///
    /// Returns `true` on success. See `Game::effect_initiated_digivolve`.
    pub fn effect_initiated_digivolve(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> bool {
        self.game.effect_initiated_digivolve(
            player,
            hand_index,
            target,
            cost_delta,
            ignore_color,
            PlaySource::ByEffect,
        )
    }

    pub fn effect_initiated_digivolve_ignore_requirements(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
    ) -> bool {
        self.game.effect_initiated_digivolve_ignore_requirements(
            player,
            hand_index,
            target,
            cost_delta,
            PlaySource::ByEffect,
        )
    }

    pub fn effect_initiated_digivolve_with_provenance(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> Option<(PermanentHandle, crate::trigger_context::ProvenanceToken)> {
        let card = self.game.player(player).hand.get(hand_index)?.handle();
        let token = self.game.provenance_token_for_card(card);
        if self.effect_initiated_digivolve(player, hand_index, target, cost_delta, ignore_color) {
            Some((target, token))
        } else {
            None
        }
    }

    /// Digivolve a card from any supported source zone onto `target` by
    /// effect. See `Game::effect_initiated_digivolve_from_source`.
    pub fn effect_initiated_digivolve_from_source(
        &mut self,
        player: PlayerId,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> bool {
        self.game.effect_initiated_digivolve_from_source(
            player,
            source,
            target,
            cost_delta,
            ignore_color,
            PlaySource::ByEffect,
        )
    }

    pub fn effect_initiated_digivolve_from_source_ignore_requirements(
        &mut self,
        player: PlayerId,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
    ) -> bool {
        self.game
            .effect_initiated_digivolve_from_source_ignore_requirements(
                player,
                source,
                target,
                cost_delta,
                PlaySource::ByEffect,
            )
    }

    /// Merge two existing battle-area permanents into a single permanent
    /// topped with a card from hand. Effect-initiated DNA digivolve.
    ///
    /// Delegates to `Game::dna_digivolve_inner` for the merge + triggers.
    /// This wrapper handles the IR's two-knob shape (`cost: i32` separate
    /// from `ignore_requirements: bool`) and the pay-memory-bypass branch
    /// that fires when `ignore_requirements` is set and the printed cost
    /// would otherwise dip below the memory floor.
    ///
    /// ## Stacking order
    ///
    /// `target_a.card_sources ++ target_b.card_sources ++ [from_hand]`.
    /// `target_a` corresponds to `DnaCost::requirement1`. See
    /// `Game::dna_digivolve_inner` for the canonical contract.
    ///
    /// ## Triggers
    ///
    /// `WhenDigivolving` → `OnDnaDigivolve` → `OnDigivolve` (global),
    /// each followed by a queue drain. See
    /// `Game::dna_digivolve_inner` for the firing sequence.
    ///
    /// ## Semantics of `ignore_requirements`
    ///
    /// `ignore_requirements: true` skips the affordability floor — i.e. the
    /// merge runs even when subtracting `cost` from memory would dip below
    /// `rules.memory_range.0`. The `cost` argument is still subtracted —
    /// `ignore_requirements` is not the same as "free". For
    /// `cost: 0, ignore_requirements: true`, no memory mutation occurs.
    ///
    /// ## Defensive validation
    ///
    /// Returns `None` if:
    /// - `target_a == target_b`
    /// - either target's index is out of range on its player's battle area
    /// - `from_hand` is not present in any player's hand
    /// - `cost > 0` and `!ignore_requirements` and the controller cannot
    ///   pay the memory cost (early-out before any state mutation)
    pub fn effect_initiated_dna_digivolve(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<PermanentHandle> {
        if target_a == target_b {
            return None;
        }
        if (target_a.index as usize) >= self.game.player(target_a.player).battle_area.len() {
            return None;
        }
        if (target_b.index as usize) >= self.game.player(target_b.player).battle_area.len() {
            return None;
        }

        // Locate the from_hand card across all players' hands.
        let mut hand_owner: Option<PlayerId> = None;
        let mut hand_index: Option<usize> = None;
        for pid in 0..self.game.players.len() {
            if let Some(idx) = self.game.players[pid]
                .hand
                .iter()
                .position(|c| c.handle() == from_hand)
            {
                hand_owner = Some(pid as PlayerId);
                hand_index = Some(idx);
                break;
            }
        }
        let (hand_owner, hand_index) = (hand_owner?, hand_index?);
        if self
            .game
            .modifiers
            .player_has(hand_owner, ModifierType::CannotDigivolveDigimonByEffect)
        {
            return None;
        }

        let effective_cost: u16 = cost.max(0) as u16;

        // Memory: under ignore_requirements bypass the floor; otherwise let
        // dna_digivolve_inner pay normally.
        if ignore_requirements && effective_cost > 0 {
            self.game.pay_memory_unchecked(effective_cost);
            // Pass cost=0 to the inner so it doesn't double-pay.
            self.game
                .dna_digivolve_inner(target_a, target_b, hand_owner, hand_index, 0, false, true)
        } else {
            self.game.dna_digivolve_inner(
                target_a,
                target_b,
                hand_owner,
                hand_index,
                effective_cost,
                false,
                true,
            )
        }
    }

    /// Effect-initiated DNA digivolve where ONE material is a battle-area
    /// permanent (`target`) and the OTHER material is a card in hand
    /// (`hand_partner`). The merged permanent is topped with `result_from_hand`
    /// (also a hand card — the Omnimon-name result).
    ///
    /// This is the BT17-095 Clause B shape: "That Digimon and a card in the
    /// hand may DNA digivolve into a Digimon card with [Omnimon] in its name
    /// in the hand." `effect_initiated_dna_digivolve` cannot express it — that
    /// verb requires BOTH DNA materials to be on-field permanents. See
    /// G-DSL-DNA-FROM-HAND-PARTNER.
    ///
    /// ## Stacking order
    ///
    /// `target.card_sources ++ [hand_partner] ++ [result_from_hand]`.
    ///
    /// ## Triggers
    ///
    /// `WhenDigivolving` → `OnDnaDigivolve` → `OnDigivolve` (global), each
    /// followed by a queue drain, all carrying the `dna_origin` marker — the
    /// same firing sequence as `effect_initiated_dna_digivolve`.
    ///
    /// ## Defensive validation
    ///
    /// Returns `None` if:
    /// - `target`'s index is out of range on its player's battle area,
    /// - `hand_partner` and `result_from_hand` are not both in the SAME
    ///   player's hand (they must share a hand owner),
    /// - `hand_partner == result_from_hand`,
    /// - the hand owner has `CannotDigivolveDigimonByEffect`,
    /// - `cost > 0` and `!ignore_requirements` and the controller cannot pay
    ///   the memory cost.
    ///
    /// `ignore_requirements` bypasses the memory affordability floor exactly
    /// as in `effect_initiated_dna_digivolve` (the cost is still subtracted).
    pub fn effect_initiated_dna_digivolve_with_hand_partner(
        &mut self,
        target: PermanentHandle,
        hand_partner: CardHandle,
        result_from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<PermanentHandle> {
        if hand_partner == result_from_hand {
            return None;
        }
        if (target.index as usize) >= self.game.player(target.player).battle_area.len() {
            return None;
        }

        // Both hand cards must live in the SAME player's hand; locate it.
        let mut hand_owner: Option<PlayerId> = None;
        let mut partner_index: Option<usize> = None;
        let mut result_index: Option<usize> = None;
        for pid in 0..self.game.players.len() {
            let hand = &self.game.players[pid].hand;
            let p = hand.iter().position(|c| c.handle() == hand_partner);
            let r = hand.iter().position(|c| c.handle() == result_from_hand);
            if let (Some(p), Some(r)) = (p, r) {
                hand_owner = Some(pid as PlayerId);
                partner_index = Some(p);
                result_index = Some(r);
                break;
            }
        }
        let (hand_owner, partner_index, result_index) =
            (hand_owner?, partner_index?, result_index?);

        if self
            .game
            .modifiers
            .player_has(hand_owner, ModifierType::CannotDigivolveDigimonByEffect)
        {
            return None;
        }

        let effective_cost: u16 = cost.max(0) as u16;

        if ignore_requirements && effective_cost > 0 {
            self.game.pay_memory_unchecked(effective_cost);
            self.game.dna_digivolve_hand_partner_inner(
                target,
                hand_owner,
                partner_index,
                result_index,
                0,
                true,
            )
        } else {
            self.game.dna_digivolve_hand_partner_inner(
                target,
                hand_owner,
                partner_index,
                result_index,
                effective_cost,
                true,
            )
        }
    }

    pub fn effect_initiated_dna_digivolve_with_provenance(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<(PermanentHandle, crate::trigger_context::ProvenanceToken)> {
        let token = self.game.provenance_token_for_card(from_hand);
        let permanent = self.effect_initiated_dna_digivolve(
            target_a,
            target_b,
            from_hand,
            cost,
            ignore_requirements,
        )?;
        Some((permanent, token))
    }

    pub fn resolve_provenance_token(
        &self,
        token: crate::trigger_context::ProvenanceToken,
    ) -> Option<crate::trigger_context::EventSubject> {
        self.game.resolve_provenance_token(token)
    }

    // ─── Modifier registration ────────────────────────────────────────

    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        // Single source of truth for the gate lives in `add_modifier`.
        self.add_modifier(target, ModifierType::ChangeDp, value, expiry);
    }

    pub fn add_declarative_dp_modifier(
        &mut self,
        target: PermanentHandle,
        value: i32,
        expiry: Expiry,
    ) {
        self.add_declarative_modifier(target, ModifierType::ChangeDp, value, expiry);
    }

    pub fn add_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        // `*NextTurn` expiries installed during the about-to-end turn must
        // skip that turn-end (otherwise they expire one turn early). The
        // engine knows the current turn at install time, so compute it
        // here — every `add_modifier` / `add_dp_modifier` caller (DSL and
        // hand-written) gets correct "until end of next turn" semantics.
        let pending_skips = crate::modifiers::pending_skips_for_install(
            expiry,
            self.player,
            self.game.turn_player(),
        );
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(modifier, value, expiry, self.player)
                .with_pending_skips(pending_skips),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_declarative_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_declarative_modifier_with_payload(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        payload: crate::modifiers::ModifierPayload,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            )
            .with_payload(payload),
        );
        self.game.mark_until_condition_dirty();
    }

    /// Install a `Expiry::UntilCondition`-scoped modifier with a runtime
    /// eviction predicate. Mirrors `add_modifier` (honors the
    /// `can_affect_permanent` guard) but tags the entry with the
    /// supplied `UntilConditionFn`. The UntilCondition controller (PR
    /// #458) evaluates the predicate after every mutation event; once
    /// the predicate returns false, the entry is removed and the
    /// printed-semantics rule applies — `false → true` does NOT
    /// re-install. Used by Track H's `while_condition` aura lowering;
    /// also exposed for raw_rust card scripts that need the same gate.
    pub fn add_modifier_with_until_condition(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        predicate: crate::modifiers::UntilConditionFn,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(modifier, value, Expiry::UntilCondition, self.player)
                .with_until_condition(predicate),
        );
        self.game.mark_until_condition_dirty();
    }

    /// Track H §4 — grant a keyword scoped to `Expiry::UntilCondition`
    /// with a runtime predicate. Mirrors `add_modifier_with_until_condition`
    /// for keyword grants. The UntilCondition controller (PR #458)
    /// evicts on the first false transition; printed-semantics rule
    /// holds (no re-install on false → true). Used by Track H's
    /// `while_condition` aura lowering for keyword grants and by
    /// raw_rust card scripts.
    pub fn grant_keyword_with_until_condition(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        predicate: crate::modifiers::UntilConditionFn,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.grant_keyword_with_until_condition(
            target,
            keyword,
            predicate,
            self.player,
        );
        self.game.mark_until_condition_dirty();
    }

    /// Track H §3 — grant a triggered effect to `carrier` (DCGO
    /// `AddSkillClass.cs` analog). The body fires when the carrier's
    /// matching `timing` event drains, with `EffectContext::source_card`
    /// = grantor (this effect's source) and `EffectContext::source_permanent`
    /// = carrier — mirroring DCGO's
    /// `EffectSourceCard` / `EffectSourcePermanent` distinction.
    ///
    /// v1 dispatch covers `EffectTiming::OnDeletion`; calls with other
    /// timings install the entry on the registry but the dispatcher
    /// hook only consults `OnDeletion` for now (extend
    /// `Game::fire_granted_triggered_effects` and its callers as
    /// further timings come online).
    ///
    /// `expiry` follows the same semantics as `add_modifier`:
    /// `Permanent` for "until carrier leaves the field" (clears via
    /// `clear_permanent`); `EndOfTurn` / `EndOfYourTurn` /
    /// `EndOfOpponentsTurn` for turn-bound grants. The
    /// `can_affect_permanent` guard is honored so opponent-effect
    /// immunities suppress the install.
    pub fn grant_triggered_effect<F>(
        &mut self,
        carrier: PermanentHandle,
        timing: crate::enums::EffectTiming,
        expiry: Expiry,
        body: F,
    ) where
        F: Fn(&mut EffectContext<'_>) + Send + Sync + 'static,
    {
        if !self.can_affect_permanent(carrier) {
            return;
        }
        // Phase 4i — allocate a body id and register the closure in the
        // game-level body registry so the queue-based drainer can fetch
        // it at fire time. The same Arc is also stored on the entry for
        // direct/inline-fire compat (legacy `fire_granted_triggered_effects`
        // path remains, though Phase 4b's drain hook is now the primary
        // dispatcher).
        self.game.next_granted_effect_id = self.game.next_granted_effect_id.saturating_add(1);
        let body_id = self.game.next_granted_effect_id;
        let body_arc: crate::modifiers::GrantedEffectBody = std::sync::Arc::new(body);
        self.game
            .granted_effect_bodies
            .insert(body_id, body_arc.clone());
        self.game.modifiers.add_granted_triggered(
            carrier,
            crate::modifiers::GrantedTriggeredEffect {
                timing,
                source_card: self.source_card,
                source_player: self.player,
                expiry,
                body_id,
                body: body_arc,
            },
        );
    }

    pub fn add_effect_immunity_modifier(
        &mut self,
        target: PermanentHandle,
        source_kind: EffectSourceKind,
        controller: EffectControllerFilter,
        expiry: Expiry,
    ) -> bool {
        if !self.can_affect_permanent(target) {
            return false;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(ModifierType::CannotBeAffected, 0, expiry, self.player)
                .with_effect_immunity_filter(EffectImmunityFilter {
                    source_kind: Some(source_kind),
                    controller,
                }),
        );
        self.game.mark_until_condition_dirty();
        true
    }

    /// Grant the common narrow protection bundle for text like "can't be
    /// returned to hand or deck or de-digivolved by your opponent's effects."
    ///
    /// These are passive replacement modifiers with the default
    /// `OpponentEffect` cause filter, not broad `CannotBeAffected` immunity.
    pub fn grant_zone_return_immunity_to_opponent_effects(
        &mut self,
        target: PermanentHandle,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        for modifier in [
            ModifierType::CannotBeReturnedToHand,
            ModifierType::CannotBeReturnedToDeck,
            ModifierType::CannotBeDeDigivolved,
        ] {
            self.game.modifiers.add(
                target,
                ModifierEntry::passive_replacement(modifier, expiry, self.player),
            );
        }
        self.game.mark_until_condition_dirty();
    }

    /// PUPPETS-G024 — grant the narrow "can't have its DP reduced by your
    /// opponent's effects and isn't affected by ＜De-Digivolve＞ effects [by
    /// your opponent's effects]" protection bundle (BT16-055 Namakemon's
    /// high-security clause).
    ///
    /// Both protections are genuinely opponent-effect-scoped:
    ///   - `ImmuneFromDPMinus` is installed with an
    ///     `EffectImmunityFilter { controller: OpponentOnly }` so
    ///     `Game::effective_dp` suppresses only negative `ChangeDp`
    ///     deltas whose source is an opponent effect — the controller's
    ///     own DP-reduction still applies.
    ///   - `CannotBeDeDigivolved` is installed via the
    ///     `passive_replacement()` route so its
    ///     `default_passive_cause_filter` (`ReplacementCause::OpponentEffect`)
    ///     takes effect — own-side De-Digivolve still applies.
    ///
    /// Unrelated opponent effects (non-DP-reduction, non-De-Digivolve) are
    /// not affected — this is a category-scoped protection, not blanket
    /// `CannotBeAffected` immunity.
    pub fn grant_narrow_opponent_effect_protection(
        &mut self,
        target: PermanentHandle,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(ModifierType::ImmuneFromDPMinus, 0, expiry, self.player)
                .with_effect_immunity_filter(EffectImmunityFilter {
                    source_kind: None,
                    controller: EffectControllerFilter::OpponentOnly,
                }),
        );
        self.game.modifiers.add(
            target,
            ModifierEntry::passive_replacement(
                ModifierType::CannotBeDeDigivolved,
                expiry,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn ignore_option_color_requirement(&mut self, target_player: PlayerId, expiry: Expiry) {
        self.game.modifiers.add_player_modifier(
            target_player,
            PlayerModifierEntry::simple(
                ModifierType::IgnoreColorRequirement,
                0,
                expiry,
                None,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_declarative_player_modifier(
        &mut self,
        target_player: PlayerId,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        self.game.modifiers.add_player_modifier(
            target_player,
            PlayerModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn grant_keyword(&mut self, target: PermanentHandle, keyword: Keyword, expiry: Expiry) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game
            .modifiers
            .grant_keyword(target, keyword, expiry, self.player);
        self.game.mark_until_condition_dirty();
    }

    pub fn grant_declarative_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.grant_declarative_keyword(
            target,
            keyword,
            expiry,
            self.source_permanent,
            self.player,
        );
        self.game.mark_until_condition_dirty();
    }

    // ─── Breeding-area mutations ──────────────────────────────────────

    /// Move a card from `source` to `player`'s security stack. Does not
    /// fire `OnLoseSecurity` observers. See `Game::place_on_security`.
    ///
    /// Phase 6: gated by `CannotAddSecurityByEffect`. The gate checks the
    /// ACTING player (the effect owner, `self.player`), not the target —
    /// consistent with DCGO's per-player restriction semantics.
    pub fn place_on_security(
        &mut self,
        player: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        // Phase 6: if the acting player has CannotAddSecurityByEffect, suppress.
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        // Track C / D consult site (2026-05-08): permanent-scoped
        // `CannotAddSecurity` — sibling of player-scoped
        // `CannotAddSecurityByEffect`. Anchored to a specific permanent in
        // the acting player's battle area, mirroring the printed-text
        // shape "while this Digimon is in play, your effects can't add
        // security cards."
        let battle_area_len = self.game.player(self.player).battle_area.len();
        for i in 0..battle_area_len {
            let h = PermanentHandle {
                player: self.player,
                index: i as u8,
            };
            if self.game.modifiers.has(h, ModifierType::CannotAddSecurity) {
                return false;
            }
        }
        self.game
            .place_on_security_observed(player, source, position, face_up, self.player)
    }

    /// (Track E) Extract a digivolution source identified by its stable `CardHandle`
    /// from `carrier`'s stack and place it into `target_player`'s security
    /// stack at `position` with the requested orientation. Track E Tier 2
    /// Task 6 — sugar over `select_own_sources` -> `place_on_security` for
    /// printed text like "place 1 of this Digimon's digivolution cards on
    /// top of your security stack" (Puppets G027 shape).
    ///
    /// Looks up the source's current index from its stable `CardHandle` —
    /// resilient to intervening battle-area shifts that would invalidate a
    /// raw `usize` index. Routes through `place_on_security_observed`, so
    /// `WhenWouldPlaceInSecurity` replacements and `CannotAddSecurityByEffect`
    /// gates apply identically to a hand/trash placement.
    ///
    /// Returns `false` if the carrier handle is invalid or the source card
    /// is not present in the carrier's stack at call time.
    pub fn security_place_stacked_card(
        &mut self,
        carrier: PermanentHandle,
        source_card: CardHandle,
        target_player: PlayerId,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        // Resolve the stable CardHandle to its current index in the carrier's
        // stack. Bail clean if the carrier was deleted or the source was
        // already moved by an intervening effect.
        let source_index = {
            let Some(perm) = self
                .game
                .player(carrier.player)
                .battle_area
                .get(carrier.index as usize)
            else {
                return false;
            };
            match perm
                .card_sources
                .iter()
                .position(|c| c.handle() == source_card)
            {
                Some(idx) => idx,
                None => return false,
            }
        };
        self.place_on_security(
            target_player,
            crate::enums::CardSourceRef::Material(carrier, source_index),
            position,
            face_up,
        )
    }

    /// Convenience: extract the **top stacked card** (the source one below
    /// the visible top, i.e. `card_sources[len - 2]`) and place it in
    /// `target_player`'s security at `position` / `face_up`. Mirrors
    /// printed text like Puppets G027 "move the top stacked card to top
    /// security card." When the carrier has fewer than 2 card_sources
    /// (no stacked card below the top), returns `false` without mutating
    /// state.
    pub fn security_place_top_stacked_card(
        &mut self,
        carrier: PermanentHandle,
        target_player: PlayerId,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        let source_card = {
            let Some(perm) = self
                .game
                .player(carrier.player)
                .battle_area
                .get(carrier.index as usize)
            else {
                return false;
            };
            let len = perm.card_sources.len();
            if len < 2 {
                // No stacked card below the top.
                return false;
            }
            perm.card_sources[len - 2].handle()
        };
        self.security_place_stacked_card(carrier, source_card, target_player, position, face_up)
    }

    /// Force `opponent`'s hand size down to `target_count` by surfacing a
    /// multi-pick selection on their hand. The opponent is the selecting
    /// player — they choose which cards to trash, per the no-approximations
    /// policy (mass forced-trash effects with player choice on the affected
    /// side, e.g. BT19-075 MoonMillenniummon "your opponent trashes cards
    /// from their hand until they have N").
    ///
    /// Returns `true` if a selection was installed, `false` if the
    /// opponent's hand is already at or below `target_count` (no-op).
    ///
    /// **Selection semantics:** uses `select_count_capped_multi` with
    /// `max = current - target_count` and `is_optional_zero = false`
    /// (forcing the opponent to pick at least one card). The engine's
    /// existing count-capped multi-select allows the player to PASS once
    /// they've picked at least 1, so for cards that strictly require an
    /// exact count (vs. up-to count), callers should chain a
    /// `trash_opponent_hand_to_count` call as a continuation until the
    /// hand size meets the target. Most printed cards are forgiving here —
    /// "until they have N" cards typically allow the opponent to control
    /// the cadence as long as the floor is reached.
    pub fn trash_opponent_hand_to_count(&mut self, opponent: PlayerId, target_count: u8) -> bool {
        let current = self.game.player(opponent).hand.len();
        let target = target_count as usize;
        if current <= target {
            return false;
        }
        let to_trash = (current - target).min(u8::MAX as usize) as u8;

        self.as_selecting_player(opponent)
            .select_count_capped_multi(
                opponent,
                CountCappedZone::Hand,
                to_trash,
                "Choose cards to trash from your hand",
                /* is_optional_zero */ false,
                /* distinct_by */ None,
                /* filter */ |_g, _c| true,
                move |ctx, picks| {
                    // Trash each chosen card by stable handle. Hand indices
                    // shift after each trash, so re-resolve per pick.
                    for card_handle in picks {
                        let idx = ctx
                            .game
                            .player(opponent)
                            .hand
                            .iter()
                            .position(|c| c.handle() == card_handle);
                        if let Some(i) = idx {
                            ctx.trash_from_hand_by_index(opponent, i);
                        }
                    }
                },
            );
        true
    }

    /// Trim up to `n` digivolution-source cards (the cards beneath the
    /// visible top) from every battle-area permanent of `target_player`.
    /// Track E Tier 2 Task 8 — bulk stack-peel primitive for printed text
    /// like BT12-028 "trash the top digivolution card of each of your
    /// opponent's Digimon" and generalisations.
    ///
    /// Semantic note: "top digivolution card" in TCG parlance means the
    /// topmost source **below** the visible top (`card_sources[len-2]`),
    /// NOT the visible top itself (which is the Digimon, not a digivolution
    /// card). This helper trims sources from the top of the stack-below-top
    /// and never deletes the visible Digimon. Permanents with stack size
    /// less than 2 (no digivolution cards under the top) are skipped.
    ///
    /// Per source: routes through `trash_card_source`, which fires
    /// `OnDigivolutionCardTrashed` with the proper `SourceTrashedFromStack`
    /// trigger context (host permanent + host_card + extracted source
    /// card). Owner-routed: `trash_card_source` pushes each card to its
    /// owner's trash.
    ///
    /// Iterates by stable `PermanentHandle` snapshots re-resolved per
    /// pass so intervening battle-area shifts (caused by observer fan-out)
    /// don't skip permanents.
    pub fn trash_top_n_digivolution_cards_of_each(
        &mut self,
        target_player: PlayerId,
        n: u8,
    ) -> usize {
        if n == 0 {
            return 0;
        }
        let mut total = 0usize;
        for _ in 0..n {
            // For each pass, snapshot per-permanent (handle, source_to_peel)
            // BEFORE any mutation. Source-to-peel is the topmost
            // digivolution card — `card_sources[len - 2]` — by stable
            // CardHandle, so the source-trash dispatch can find it even if
            // the carrier's index shifted between passes.
            let mut targets: Vec<(PermanentHandle, CardHandle)> = Vec::new();
            for (i, perm) in self
                .game
                .player(target_player)
                .battle_area
                .iter()
                .enumerate()
            {
                let len = perm.card_sources.len();
                if len < 2 {
                    continue;
                }
                let source_card = perm.card_sources[len - 2].handle();
                targets.push((
                    PermanentHandle {
                        player: target_player,
                        index: i as u8,
                    },
                    source_card,
                ));
            }
            if targets.is_empty() {
                break;
            }
            for (handle, source_card) in targets {
                // Re-validate the carrier slot AND the source's continued
                // membership in this stack (an earlier permanent's
                // observer trigger could have moved this source). Skip
                // gracefully if state no longer matches.
                let still_present = self
                    .game
                    .player(target_player)
                    .battle_area
                    .get(handle.index as usize)
                    .map(|p| p.card_sources.iter().any(|c| c.handle() == source_card))
                    .unwrap_or(false);
                if !still_present {
                    continue;
                }
                // Bool discarded: caller counts attempts that passed the
                // pre-validation gate, not actuals. Aligns with the existing
                // contract — `total` reflects "trash attempts dispatched".
                let _ = self.trash_card_source(handle, source_card);
                total += 1;
            }
        }
        total
    }

    /// Drain `player`'s trash and append each card to its **owner's** deck
    /// bottom. Returns the handles of moved cards in their original trash
    /// order. Track E Tier 2 Task 7 — bulk move primitive used by printed
    /// text like BT17-077 Imperialdramon: Paladin Mode "return all cards
    /// in your trash to the bottom of the deck."
    ///
    /// Owner-routed: each card consults its `CardSource.owner` field, not
    /// the `player` parameter. In the common case where every card in a
    /// player's trash was originally owned by that player, this is a pure
    /// drain into the same player's deck. In the cross-player case (a card
    /// was effect-moved into the opposing trash by a prior effect), each
    /// card returns to its original owner's deck — matching the rules-default
    /// behavior for cards moving between owners' zones.
    ///
    /// Does NOT fire `OnReturn` per card — the existing engine doesn't have
    /// a `Game::return_to_deck` per-card observer dispatch and the printed
    /// cards consuming this primitive bind the moved set as an ordered set
    /// for downstream predicates rather than per-card observation. Treated
    /// as a bulk move; per-card observer fan-out can land as a follow-up.
    pub fn return_all_trash_to_deck_bottom(&mut self, player: PlayerId) -> Vec<CardHandle> {
        // Drain trash in order. Each card is appended to the start of its
        // owner's deck (deck bottom = index 0 by convention; deck top =
        // Vec end, the position drawn from first).
        let drained: Vec<crate::card_source::CardSource> =
            std::mem::take(&mut self.game.player_mut(player).trash);
        let mut handles = Vec::with_capacity(drained.len());
        for card in drained {
            handles.push(card.handle());
            let owner = card.owner;
            self.game.player_mut(owner).deck.insert(0, card);
        }
        handles
    }

    /// Move a SELECTED LIST of cards out of `player`'s trash to the bottom of
    /// the deck, in the given order (the first handle ends up deepest). Unlike
    /// `return_all_trash_to_deck_bottom`, this targets exactly the cards in
    /// `cards` (e.g. a `select_count_capped_multi` pick set) and leaves the
    /// rest of the trash untouched. Returns the handles actually moved (a
    /// handle not found in the trash is silently skipped).
    /// G-ZONE-TRASH-TO-DECK.
    pub fn return_trash_cards_to_deck_bottom(
        &mut self,
        player: PlayerId,
        cards: &[CardHandle],
    ) -> Vec<CardHandle> {
        let mut moved = Vec::with_capacity(cards.len());
        for &handle in cards {
            let Some(pos) = self
                .game
                .player(player)
                .trash
                .iter()
                .position(|c| c.handle() == handle)
            else {
                continue;
            };
            let card = self.game.player_mut(player).trash.remove(pos);
            let owner = card.owner;
            self.game.player_mut(owner).deck.insert(0, card);
            moved.push(handle);
        }
        moved
    }

    /// Move a SELECTED LIST of cards out of `player`'s trash to the **top** of
    /// the deck — the position `draw` pops first. The deck-bottom sibling is
    /// `return_trash_cards_to_deck_bottom`; by deck convention bottom is index
    /// 0 and top is the `Vec` end. The first handle in `cards` ends up on top
    /// (drawn first), the rest sit just beneath it, so selection order becomes
    /// draw order. Returns the handles actually moved, in selection order (a
    /// handle not found in the trash is silently skipped).
    /// G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
    pub fn return_trash_cards_to_deck_top(
        &mut self,
        player: PlayerId,
        cards: &[CardHandle],
    ) -> Vec<CardHandle> {
        let mut moved = Vec::with_capacity(cards.len());
        // Iterate in reverse and `push`: the first handle in `cards` is pushed
        // last, landing at the `Vec` end (= deck top, drawn first).
        for &handle in cards.iter().rev() {
            let Some(pos) = self
                .game
                .player(player)
                .trash
                .iter()
                .position(|c| c.handle() == handle)
            else {
                continue;
            };
            let card = self.game.player_mut(player).trash.remove(pos);
            let owner = card.owner;
            self.game.player_mut(owner).deck.push(card);
            moved.push(handle);
        }
        // `moved` was built in reverse; restore selection order for callers.
        moved.reverse();
        moved
    }

    /// Move a single SELECTED card out of `player`'s trash to the TOP of the
    /// deck (the position drawn from first). The card returns to its OWNER's
    /// deck — `player` only identifies whose trash zone currently holds it.
    /// Selected-trash analog of `return_trash_cards_to_deck_bottom`, but
    /// single-card and deck-TOP. Returns true if the card was found and moved.
    /// A handle not present in `player`'s trash is a silent no-op.
    /// G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
    pub fn move_trash_card_to_deck_top(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(pos) = self
            .game
            .player(player)
            .trash
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        let removed = self.game.player_mut(player).trash.remove(pos);
        let owner = removed.owner;
        // Deck top = Vec end (drawn first) per engine convention.
        self.game.player_mut(owner).deck.push(removed);
        true
    }

    /// (Track A) Move a battle-area permanent to a player's security stack
    /// through the normal leave-field replacement window. This is for
    /// effects that initiate a new move to security, not replacement bodies
    /// already handling an in-flight leave event.
    pub fn place_permanent_on_security(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        self.game
            .place_permanent_on_security(player, target, position, face_up, self.player)
    }

    /// Recover up to `count` cards from `player`'s deck to the top of security.
    pub fn recover_from_deck(&mut self, player: PlayerId, count: u8) -> u8 {
        let mut recovered = 0;
        for _ in 0..count {
            if self.place_on_security(
                player,
                crate::enums::CardSourceRef::DeckTop(player),
                crate::enums::StackPosition::Top,
                false,
            ) {
                recovered += 1;
            } else {
                break;
            }
        }
        recovered
    }

    // ─── Combat mutations (Phase 9 Task 2) ────────────────────────────

    /// Redirect the active attack to a new target. Fires
    /// `OnAttackTargetChange` globally. Spec §6.1.
    ///
    /// Callable from any effect closure running during an active attack —
    /// `OnAttack`, `WhenAttacking`, a replacement process (equivalent to
    /// `rctx.substitute(...)` via the replacement committer), or a
    /// selection callback.
    ///
    /// Errors:
    /// - `AttackError::NoActiveAttack` — `pending_attack` is `None`.
    /// - `AttackError::InvalidTarget` — target is not a legal attack target
    ///   (own attacker, non-Digimon, Delayed/Training Option, or wrong
    ///   player for a direct-player attack).
    ///
    /// No-op + `Ok(())` if `new_target` equals the current effective
    /// target (suppress redundant `OnAttackTargetChange` fan-out).
    pub fn redirect_attack(
        &mut self,
        new_target: crate::selection::AttackTarget,
    ) -> Result<(), crate::combat::AttackError> {
        use crate::combat::AttackError;
        let Some(pa) = self.game.pending_attack.as_ref() else {
            return Err(AttackError::NoActiveAttack);
        };
        let attacker = pa.attacker;
        self.game
            .validate_attack_redirect_target(attacker, new_target)?;
        self.game.apply_attack_target_substitution_with_reason(
            new_target,
            crate::trigger_context::AttackTargetChangeReason::EffectRedirect(Some(
                self.source_card,
            )),
        );
        Ok(())
    }

    pub fn select_redirect_attack_target(
        &mut self,
        targets: AttackTargetRestriction,
        optional: bool,
        prompt: &str,
    ) -> Result<(), crate::combat::AttackError> {
        use crate::combat::AttackError;
        let Some(pa) = self.game.pending_attack.as_ref() else {
            return Err(AttackError::NoActiveAttack);
        };
        let attacker = pa.attacker;
        let current_target = pa.effective_target;
        let opponent = self.game.next_clockwise(attacker.player);
        let mut valid_action_ids = Vec::new();

        if matches!(
            targets,
            AttackTargetRestriction::Any | AttackTargetRestriction::PlayerOnly
        ) {
            let target = AttackTarget::Player(opponent);
            if target != current_target
                && self
                    .game
                    .validate_attack_redirect_target(attacker, target)
                    .is_ok()
            {
                valid_action_ids.push(encode_attack(attacker.index as u16, SECURITY_TARGET));
            }
        }

        if matches!(
            targets,
            AttackTargetRestriction::Any | AttackTargetRestriction::DigimonOnly
        ) {
            for index in 0..self.game.player(opponent).battle_area.len() {
                let target = AttackTarget::Digimon(PermanentHandle {
                    player: opponent,
                    index: index as u8,
                });
                if target != current_target
                    && self
                        .game
                        .validate_attack_redirect_target(attacker, target)
                        .is_ok()
                {
                    valid_action_ids.push(encode_attack(attacker.index as u16, index as u16));
                }
            }
        }

        if valid_action_ids.is_empty() {
            return Ok(());
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let previous_phase = self.game.current_phase;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;

        self.game.current_phase = GamePhase::SelectTarget;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Target,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game, action_id| {
                let (decoded_attacker, decoded_target) = decode_attack(action_id);
                if decoded_attacker as u8 != attacker.index {
                    return;
                }
                let opponent = game.next_clockwise(attacker.player);
                let target = if decoded_target == SECURITY_TARGET {
                    AttackTarget::Player(opponent)
                } else {
                    AttackTarget::Digimon(PermanentHandle {
                        player: opponent,
                        index: decoded_target as u8,
                    })
                };
                if game
                    .validate_attack_redirect_target(attacker, target)
                    .is_ok()
                {
                    game.apply_attack_target_substitution_with_reason(
                        target,
                        crate::trigger_context::AttackTargetChangeReason::EffectRedirect(Some(
                            source_card,
                        )),
                    );
                }
            }),
            on_decline: if optional {
                Some(Box::new(|_game: &mut Game| {}) as DeclineCallback)
            } else {
                None
            },
        });
        Ok(())
    }

    /// Cancel the active attack. Sets `pending_attack.cancelled = true`;
    /// `advance_pending_attack` detects the flag and short-circuits to
    /// `Cleanup`. `EndOfAttack` still fires (cleanup symmetry);
    /// `EndOfBattle` does NOT (no DP comparison ran). Spec §6.2.
    ///
    /// Errors: `AttackError::NoActiveAttack` if `pending_attack` is `None`.
    ///
    /// **Late-cancel semantics.** If called after a `ctx.redirect_attack`
    /// in the same attack, the redirect's mutation to
    /// `effective_target` survives — `cancelled` short-circuits
    /// `advance_pending_attack` regardless. Cancel wins over redirect
    /// in the sense that no battle occurs; the redirected target is
    /// observable only via the `OnAttackTargetChange` observer that
    /// already fired.
    pub fn cancel_attack(&mut self) -> Result<(), crate::combat::AttackError> {
        self.game.cancel_pending_attack_from_effect_checked()
    }

    pub fn cancel_pending_attack(&mut self) {
        self.game.cancel_pending_attack_from_effect();
    }

    pub fn open_counter_window(&mut self) -> Result<bool, crate::combat::AttackError> {
        self.game.open_counter_window_from_effect_checked()
    }

    pub fn battle_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        self.game.battle_digimon(attacker, defender)
    }

    pub fn may_attack_now(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional(attacker, targets, without_suspending, true, prompt)
    }

    pub fn may_attack_now_optional(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional_with_upgrade(
            attacker,
            targets,
            without_suspending,
            optional,
            prompt,
            None,
        )
    }

    pub fn may_attack_now_optional_with_upgrade(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
    ) -> Result<(), AttackError> {
        let valid_action_ids =
            effect_attack_target_action_ids(self.game, attacker, targets, without_suspending);
        if valid_action_ids.is_empty() {
            return Ok(());
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let previous_phase = self.game.current_phase;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;

        self.game.current_phase = GamePhase::SelectTarget;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Target,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game, action_id| {
                let (decoded_attacker, decoded_target) = decode_attack(action_id);
                if decoded_attacker as u8 != attacker.index {
                    return;
                }
                let opponent = game.next_clockwise(attacker.player);
                let target = if decoded_target == SECURITY_TARGET {
                    AttackTarget::Player(opponent)
                } else {
                    AttackTarget::Digimon(PermanentHandle {
                        player: opponent,
                        index: decoded_target as u8,
                    })
                };
                game.begin_attack_open(AttackOpen {
                    attacker,
                    initiator: AttackInitiator::Effect {
                        source: Some(source_card),
                        optional,
                    },
                    suspend_attacker: !without_suspending,
                    target_constraint: TargetConstraint::Forced(target),
                    allow_cancel: optional,
                    cost_upgrade,
                });
            }),
            on_decline: if optional {
                Some(Box::new(|_game: &mut Game| {}) as DeclineCallback)
            } else {
                None
            },
        });
        Ok(())
    }

    pub fn force_opponent_attack(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.force_opponent_attack_with_upgrade(attacker, targets, without_suspending, prompt, None)
    }

    /// G-DSL-EOT-DNA-INLINE — surface an inline DNA digivolve choice at
    /// trigger fire time. Orchestrates the three-stage selection chain:
    /// (1) partner permanent from own field (anchor excluded), (2) target
    /// Digimon card from controller's hand, (3) call to the existing
    /// `effect_initiated_dna_digivolve` primitive.
    ///
    /// `anchor` is the source DNA material (typically the trigger source).
    /// The partner filter is re-wrapped internally to exclude the anchor
    /// handle, so callers need not encode that exclusion in the predicate.
    ///
    /// `optional` here is the eligibility "skip silently" gate — when
    /// either no eligible partner exists on own field OR no eligible target
    /// exists in hand, the step is a clean no-op regardless of this flag.
    /// The outer triggered clause's own `optional: true` provides the
    /// player-visible "may" via the trigger-order bundle. When `optional`
    /// is true, the partner selection prompt allows decline (the player
    /// can back out at the partner-pick stage).
    ///
    /// Backed by `effect_initiated_dna_digivolve`; carries identical
    /// trigger semantics (`WhenDigivolving → OnDnaDigivolve → OnDigivolve`
    /// with per-trigger drains).
    pub fn may_dna_digivolve_now(
        &mut self,
        anchor: PermanentHandle,
        partner_filter: std::sync::Arc<
            dyn Fn(&Game, PermanentHandle) -> bool + Send + Sync,
        >,
        target_filter: std::sync::Arc<dyn Fn(&Game, usize) -> bool + Send + Sync>,
        cost: u16,
        ignore_requirements: bool,
        optional: bool,
        partner_prompt: Option<&str>,
        target_prompt: Option<&str>,
    ) {
        // Defensive: anchor must still be on its player's battle area.
        if (anchor.index as usize) >= self.game.player(anchor.player).battle_area.len() {
            return;
        }

        // Quick install-time eligibility checks. If either side has zero
        // candidates the step is a silent no-op (matches DCGO's
        // `CanActivateCondition` returning false).
        let controller = self.player;
        let has_partner = {
            let battle_len = self.game.player(controller).battle_area.len();
            (0..battle_len).any(|i| {
                let h = PermanentHandle {
                    player: controller,
                    index: i as u8,
                };
                h != anchor && partner_filter(self.game, h)
            })
        };
        if !has_partner {
            return;
        }
        let has_target = {
            let hand_len = self.game.player(controller).hand.len();
            (0..hand_len).any(|i| target_filter(self.game, i))
        };
        if !has_target {
            return;
        }

        // Snapshot the partner/target predicates for the chained closures.
        let partner_filter_for_install = std::sync::Arc::clone(&partner_filter);
        let target_filter_for_inner = std::sync::Arc::clone(&target_filter);

        let partner_prompt = partner_prompt
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Choose a DNA digivolve partner".to_string());
        let target_prompt = target_prompt
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Choose a Digimon card from hand to DNA digivolve into".to_string());

        // Install partner selection. The anchor exclusion is enforced inline.
        self.select_own_permanent(
            &partner_prompt,
            optional,
            move |game, h| h != anchor && partner_filter_for_install(game, h),
            move |ctx, partner| {
                // Inner stage: install target hand selection.
                let target_filter_for_inner = std::sync::Arc::clone(&target_filter_for_inner);
                ctx.select_hand(
                    controller,
                    &target_prompt,
                    optional,
                    move |g, i| target_filter_for_inner(g, i),
                    move |ctx, hand_idx| {
                        // Final stage: resolve hand_idx to a CardHandle and
                        // delegate to the existing engine primitive.
                        let card = match ctx
                            .game
                            .player(controller)
                            .hand
                            .get(hand_idx)
                            .map(|c| c.handle())
                        {
                            Some(c) => c,
                            None => return,
                        };
                        ctx.effect_initiated_dna_digivolve(
                            anchor,
                            partner,
                            card,
                            cost as i32,
                            ignore_requirements,
                        );
                    },
                );
            },
        );
    }

    pub fn force_opponent_attack_with_upgrade(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        prompt: &str,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
    ) -> Result<(), AttackError> {
        let previous_override = self.override_selecting_player;
        self.override_selecting_player = Some(attacker.player);
        let result = self.may_attack_now_optional_with_upgrade(
            attacker,
            targets,
            without_suspending,
            false,
            prompt,
            cost_upgrade,
        );
        self.override_selecting_player = previous_override;
        result
    }

    /// Move the top of `player`'s digitama deck into the breeding area.
    ///
    /// Returns `true` if a hatch occurred — i.e. the breeding slot was
    /// empty and the digitama deck had at least one card.  Returns `false`
    /// if the breeding slot was already occupied or the digitama deck was
    /// empty.
    ///
    /// No `PermanentHandle` is returned: breeding-area permanents are
    /// addressed separately from battle-area permanents and do not use
    /// the same handle type.
    pub fn hatch(&mut self, player: PlayerId) -> bool {
        self.game.hatch(player)
    }
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

    #[test]
    fn play_token_unknown_name_returns_none() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
        assert!(ctx.play_token(0, "no-such-token-lol").is_none());
    }
}
