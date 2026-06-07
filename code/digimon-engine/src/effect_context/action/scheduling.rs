//! Scheduling / delayed-effect mutations on `EffectContext` — extracted from `mod.rs` by mechanic.

#![allow(unused_imports)]
use crate::action::mask::*;
use crate::action::space::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;

impl<'a> EffectContext<'a> {
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
}
