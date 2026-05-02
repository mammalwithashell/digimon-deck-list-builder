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

pub use selections::{CountCappedZone, DistinctByMode, EffectContextSelectorScope};

pub use crate::selection::{BreedingPermanentSelectionRef, SourceSelectionRef};

use crate::card_data::CardData;
use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::enums::{
    CardKind, DelayTrigger, EffectSourceKind, EffectTiming, Expiry, Keyword, ModifierType,
    PlaySource, PlayerId, StackPosition,
};
use crate::game::Game;
use crate::game_actions::PlayFromHandCostResult;
use crate::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::replacement::{ReplacementCause, ReplacementSubject};
use crate::rules::Rules;
use crate::scheduled_effects::ScheduledEffect;
use digimon_dsl::compiled::CompiledStep;

pub struct PartitionRequirement {
    pub label: &'static str,
    pub matches: Box<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>,
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
        }
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
            .and_then(|trigger| trigger.event_permanent)
    }

    pub fn event_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
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
            .and_then(|trigger| trigger.event_source_card)
    }

    pub fn event_host_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .and_then(|trigger| trigger.event_host_card)
    }

    pub fn event_host_permanent(&self) -> Option<PermanentHandle> {
        self.game.current_trigger_context.and_then(|trigger| {
            let handle = trigger.event_host_permanent?;
            live_event_permanent(self.game, handle, trigger.event_host_card)
        })
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
        self.game.current_deletion_cause
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayCostStatus {
    Paid,
    Unpaid,
    Pending,
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
        }
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

    pub fn place_self_as_delay_option_permanent(&mut self) {
        let Some(source_perm) = self.source_permanent else {
            return;
        };
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

        // The physical Option moves to its card owner/controller's battle area,
        // matching normal Delay placement from hand/trash.
        let owner = source_card.owner;
        let placed_card = source_card.handle();
        let trigger = DelayTrigger::EndOfYourNextTurn;
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
                permanent: handle,
                card: placed_card,
            },
        );
        self.game.drain_effect_queue();
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
            .and_then(|trigger| trigger.event_permanent)
    }

    pub fn event_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .and_then(|trigger| trigger.event_card)
    }

    pub fn event_card_name_contains(&self, needle: &str) -> bool {
        self.as_read().event_card_name_contains(needle)
    }

    pub fn event_source_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .and_then(|trigger| trigger.event_source_card)
    }

    pub fn event_host_card(&self) -> Option<CardHandle> {
        self.game
            .current_trigger_context
            .and_then(|trigger| trigger.event_host_card)
    }

    pub fn event_host_permanent(&self) -> Option<PermanentHandle> {
        self.game.current_trigger_context.and_then(|trigger| {
            let handle = trigger.event_host_permanent?;
            live_event_permanent(self.game, handle, trigger.event_host_card)
        })
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
        self.game.current_deletion_cause
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

    pub fn permanent_top_card_handle(&self, handle: PermanentHandle) -> Option<CardHandle> {
        self.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|permanent| permanent.top_card().handle())
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
        self.game.gain_memory(amount);
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

        self.game.player_mut(player).draw_many(count)
    }

    /// Trash the top N cards of a player's deck.
    pub fn trash_from_top(&mut self, player: PlayerId, count: u8) -> u8 {
        let p = self.game.player_mut(player);
        let mut trashed = 0;
        for _ in 0..count {
            if let Some(card) = p.deck.pop() {
                p.trash.push(card);
                trashed += 1;
            } else {
                break;
            }
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
            true
        } else {
            false
        }
    }

    fn fire_security_removed_observers(
        &mut self,
        defender: PlayerId,
        card: crate::card_source::CardSource,
        destination: crate::selection::SecurityRemovalDestination,
    ) {
        self.game
            .fire_effect_security_removal(defender, self.player, card, destination);
    }

    // ─── Field mutations ──────────────────────────────────────────────

    pub fn can_affect_permanent(&self, target: PermanentHandle) -> bool {
        !self.game.progress_excludes(target, Some(self.player))
            && !self
                .game
                .permanent_is_unaffected_by_effect(target, self.player, self.source_kind)
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
            let p = self.game.player_mut(owner);
            let stack = &mut p.battle_area[target.index as usize].card_sources;
            debug_assert!(stack.len() >= 2, "stack_size-guard failed");
            let popped_card = stack.pop().expect("stack_size-guarded pop");
            p.trash.push(popped_card);
            popped += 1;
        }

        popped
    }

    /// Materialize a token on `controller`'s battle area.
    ///
    /// Looks up `token_name` in `game.token_registry`, synthesizes a
    /// `CardSource` with `is_token = true`, wraps it in a `Permanent`, and
    /// pushes onto `controller.battle_area`. No play cost, no OnPlay
    /// observer fan-out (tokens enter via effect, not via `play_from_hand`).
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
        Some(PermanentHandle {
            player: controller,
            index: idx as u8,
        })
    }

    /// Suspend a permanent and fire `OnSuspend` observers.
    /// Delegates to `Game::suspend` — the canonical single-target chokepoint.
    pub fn suspend(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.suspend(target);
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

    /// Reveal up to `n` cards from the top of `player`'s deck. See
    /// `Game::reveal_top_deck`.
    pub fn reveal_top_deck(
        &mut self,
        player: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        self.game.reveal_top_deck(player, n)
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
        // Pop the top of security. Empty stack → nothing to do.
        let card = match self.game.player_mut(player).security.pop() {
            Some(c) => c,
            None => return None,
        };

        // `face_up_security` is keyed by card_index — clear it whether or
        // not the card was face-up; remove() is a no-op when absent.
        let card_index = card.card_index;
        self.game
            .player_mut(player)
            .face_up_security
            .remove(&card_index);

        // Park at end of hand and play through the established hand-free
        // path. The hand index is the new last position.
        self.game.player_mut(player).hand.push(card);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self.game.play_from_hand_with_cost_result(
            player,
            hand_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            false,
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
                self.game.player_mut(player).security.push(card);
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

        match self.game.play_from_hand_with_cost_result(
            player,
            hand_index,
            cost_delta,
            PlaySource::ByEffect,
            false,
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                // Rollback: pop the card out of hand and reinsert it at
                // its original index in `target.card_sources` so the
                // failure is a clean no-op for callers.
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
        let field_index = self.game.play_from_trash_with_cost(
            controller,
            trash_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
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
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
    ) -> bool {
        self.game
            .place_as_bottom_source_observed(source, target, self.player)
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
    /// # Panics
    ///
    /// Panics if `card` cannot be located in any zone — this represents a
    /// programming error (passing an invalid or already-moved handle).
    ///
    /// Used by: `<Save>`, `<Material Save N>`.
    pub fn place_card_under_permanent_bottom(&mut self, card: CardHandle, target: PermanentHandle) {
        let taken = self
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
            target_player.trash.push(taken);
            return;
        }
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
            self.game.player_mut(controller).trash.push(source);
            for pid in 0..self.game.players.len() {
                self.game.enqueue_triggered(
                    crate::enums::EffectTiming::OnDigivolutionCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            self.game.drain_effect_queue();
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
        // enqueue once per player so observers on either side of the field
        // pick it up.
        for pid in 0..self.game.players.len() {
            self.game.enqueue_triggered(
                crate::enums::EffectTiming::OnDigivolutionCardTrashed,
                crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
            );
        }
        self.game.drain_effect_queue();
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
        let owner = perm.player;
        let mut card = match self.game.player_mut(owner).deck.pop() {
            Some(c) => c,
            None => return,
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
    /// Panics if `card` is not present in `perm.card_sources`. Token cards
    /// (`is_token == true`) are still pushed to trash; the caller's gate is
    /// responsible for any token-aware filtering.
    pub fn trash_card_source(&mut self, perm: PermanentHandle, card: CardHandle) {
        let (removed, host_card) = {
            let permanent = self
                .game
                .player_mut(perm.player)
                .battle_area
                .get_mut(perm.index as usize)
                .expect("trash_card_source: permanent not found");
            let host_card = permanent.top_card().handle();
            let pos = permanent
                .card_sources
                .iter()
                .position(|c| c.handle() == card)
                .expect("trash_card_source: card not in this permanent's stack");
            (permanent.card_sources.remove(pos), host_card)
        };
        let source_card = removed.handle();
        let owner = removed.owner;
        self.game.player_mut(owner).trash.push(removed);
        self.game.enqueue_triggered(
            crate::enums::EffectTiming::OnDigivolutionCardTrashed,
            crate::selection::TriggerSource::SourceTrashedFromStack {
                player: perm.player,
                host: perm,
                host_card,
                card: source_card,
            },
        );
        self.game.drain_effect_queue();
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
        self.game.enqueue_triggered(
            crate::enums::EffectTiming::OnDigivolutionCardTrashed,
            crate::selection::TriggerSource::SourceTrashedFromStack {
                player: target.player,
                host: target,
                host_card,
                card: source_card,
            },
        );
        self.game.drain_effect_queue();
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
                .dna_digivolve_inner(target_a, target_b, hand_owner, hand_index, 0, false)
        } else {
            self.game.dna_digivolve_inner(
                target_a,
                target_b,
                hand_owner,
                hand_index,
                effective_cost,
                false,
            )
        }
    }

    // ─── Modifier registration ────────────────────────────────────────

    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        // Single source of truth for the gate lives in `add_modifier`.
        self.add_modifier(target, ModifierType::ChangeDp, value, expiry);
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
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(modifier, value, expiry, self.player),
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
        true
    }

    pub fn grant_keyword(&mut self, target: PermanentHandle, keyword: Keyword, expiry: Expiry) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game
            .modifiers
            .grant_keyword(target, keyword, expiry, self.player);
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
        self.game
            .place_on_security_observed(player, source, position, face_up, self.player)
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
        self.game.validate_attack_target(attacker, new_target)?;
        self.game.apply_attack_target_substitution(new_target);
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
        use crate::combat::AttackError;
        let Some(pa) = self.game.pending_attack.as_mut() else {
            return Err(AttackError::NoActiveAttack);
        };
        pa.cancelled = true;
        Ok(())
    }

    pub fn cancel_pending_attack(&mut self) {
        self.game.cancel_pending_attack_from_effect();
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
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
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
