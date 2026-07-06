//! Play / put-into-play operations (Tier 2) — `impl Game`, split by mechanic.

#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
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
use rand::seq::SliceRandom;

impl Game {
    /// Play a card from hand to field for a player, paying the printed cost.
    ///
    /// Delegates to [`Self::play_from_hand_with_cost`] with
    /// `CostDelta::Reduce(0)` (pay the printed cost verbatim) and
    /// `PlaySource::ByHand` (standard player-action play).
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> Option<usize> {
        self.play_from_hand_with_cost(
            player_id,
            hand_index,
            crate::enums::CostDelta::Reduce(0),
            PlaySource::ByHand,
        )
    }

    /// Generalization of `play_from_hand` — computes memory cost via the given
    /// `CostDelta` and plays the card. The caller's `CostDelta::Reduce(0)` is
    /// equivalent to paying the printed cost.
    ///
    /// Flow (matches Python):
    /// 1. Validate hand index and field capacity.
    /// 2. Read the card's printed play cost from `card_data`.
    /// 3. Apply `cost_delta.resolve(printed_cost)` to get the effective cost.
    /// 4. Call `pay_memory(effective_cost)`; if unaffordable, abort with `None`
    ///    and leave state unchanged.
    /// 5. Remove the card from hand, create a Permanent on the field.
    /// 6. Fire `OnPlay` effects via the registry.
    ///
    /// Returns `Some(field_index)` on success, `None` if the hand index is
    /// invalid, the battle area is full, or memory is insufficient.
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_hand_with_cost(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> Option<usize> {
        self.play_from_hand_with_cost_result(player_id, hand_index, cost_delta, source, true)
            .into_option()
    }

    /// Play a card from `player`'s trash to field, paying the printed cost.
    ///
    /// Delegates to [`Self::play_from_trash_with_cost`] with
    /// `CostDelta::Reduce(0)` (pay the printed cost verbatim) and
    /// `PlaySource::ByEffect` (trash plays are always effect-driven).
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_trash(&mut self, player_id: PlayerId, trash_index: usize) -> Option<usize> {
        self.play_from_trash_with_cost(
            player_id,
            trash_index,
            crate::enums::CostDelta::Reduce(0),
            PlaySource::ByEffect,
        )
    }

    /// Play a card from `player`'s trash. Like `play_from_hand_with_cost` but
    /// reads and removes from `player.trash`. Returns `Some(field_index)` on
    /// success, `None` if trash_index is invalid, battle area full, or memory
    /// insufficient.
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_trash_with_cost(
        &mut self,
        player_id: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> Option<usize> {
        self.play_from_trash_with_cost_suppress(player_id, trash_index, cost_delta, source, false)
    }

    /// As [`Self::play_from_trash_with_cost`], but threads a `suppress_on_play`
    /// flag (PUPPETS-G030). When `true`, the just-played permanent's own
    /// `[On Play]` effects are skipped for this play event only. Used by
    /// BT5-106's [Security] clause.
    pub fn play_from_trash_with_cost_suppress(
        &mut self,
        player_id: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        suppress_on_play: bool,
    ) -> Option<usize> {
        if self
            .modifiers
            .player_has(player_id, ModifierType::CannotPlayFromTrash)
        {
            return None;
        }
        let field_slots = self.rules.field_slots;

        let card_kind = {
            let player = self.player(player_id);
            if trash_index >= player.trash.len() {
                return None;
            }
            if player.battle_area.len() >= field_slots as usize {
                return None;
            }
            let card = &player.trash[trash_index];
            card.card_kind(&self.card_data)
        };

        // Phase 6: CannotPlayDigimonByEffect — when source is ByEffect and the
        // card is a Digimon, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Digimon
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayDigimonByEffect)
        {
            return None;
        }

        // Phase 6: CannotPlayTamerByEffect — when source is ByEffect and the
        // card is a Tamer, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Tamer
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayTamerByEffect)
        {
            return None;
        }

        let card = self.player_mut(player_id).trash.remove(trash_index);
        self.player_mut(player_id).hand.push(card);
        let hand_index = self.player(player_id).hand.len() - 1;

        match self.play_from_hand_with_cost_result_from_origin_suppress(
            player_id,
            hand_index,
            cost_delta,
            source,
            false,
            PendingWouldPlayOrigin::Trash { index: trash_index },
            suppress_on_play,
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(field_index),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .player_mut(player_id)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                self.player_mut(player_id).trash.insert(trash_index, card);
                None
            }
        }
    }

    /// Fire OnPlay effects for the permanent at `(player, field_index)`.
    /// Called by play_from_hand; can also be called directly by tests.
    ///
    /// Thin wrapper over the effect-queue drainer. Single-trigger cases fire
    /// in one step exactly like the old atomic loop; multi-trigger cases
    /// park on a `TriggerOrder` selection for the controller to order.
    pub fn fire_on_play(&mut self, player_id: PlayerId, field_index: usize) {
        if field_index >= self.players[player_id as usize].battle_area.len() {
            return;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        self.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
        // See G-DSL-OUTER-TAIL-NESTED-PARK fix note in
        // `fire_on_link_after_option_placed`.
        self.maybe_drain_effect_queue();
    }

    /// Fire the full play-event trigger bundle for the permanent at
    /// `(player_id, field_index)`: the played card's own `[On Play]`
    /// (timing `OnPlay`), plus the broadcast observers `OnEnterFieldAnyone`
    /// (anyone-reactive) and `OnAllyPlayed` (own-ally-reactive).
    ///
    /// All three trigger sources are enqueued BEFORE the queue drains —
    /// the helper wraps the four engine calls in
    /// `enter_deferred_drain()` / `exit_deferred_drain_and_flush()` so the
    /// played card's `[On Play]` and any observer `[All Turns]` triggers
    /// share a single drain. When the resulting bundle has ≥2 triggers
    /// for the active chooser, the drainer surfaces a `TriggerOrder`
    /// selection — restoring DCGO-aligned simultaneous-trigger ordering
    /// that the previous inline-`fire_on_play`-then-enqueue-observers
    /// pattern broke (the inline `fire_on_play` drained immediately,
    /// leaving observers in a follow-up drain that could not be reordered
    /// against the played card's own `[On Play]`).
    ///
    /// `suppress_on_play: true` skips ONLY the played card's own `[On Play]`
    /// broadcast — used by BT5-106's `[Security]` clause per
    /// PUPPETS-G030. Observer broadcasts (`OnEnterFieldAnyone` /
    /// `OnAllyPlayed`) are always enqueued.
    ///
    /// Also folds in the post-broadcast `mark_until_condition_dirty()` +
    /// `reevaluate_until_condition_modifiers_if_dirty()` calls every
    /// play-event call site needs; the helper is the single source of
    /// truth for "I just played a Digimon — fire all the play-event
    /// triggers and finalize state."
    pub fn fire_play_event_triggers(
        &mut self,
        player_id: PlayerId,
        field_index: usize,
        effect_initiated: bool,
        suppress_on_play: bool,
    ) {
        if field_index >= self.players[player_id as usize].battle_area.len() {
            return;
        }
        let entered = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        let entered_card = self.players[player_id as usize].battle_area[field_index]
            .top_card()
            .handle();

        // Judge-quiz Q28: the [On Play] suppression is itself an EFFECT of
        // the suppressor on the played Digimon. When the suppressor's
        // identity is recorded, a played permanent protected from that
        // effect (Gankoomon X's "none of your Digimon are affected by your
        // opponent's Digimon's effects") still fires its [On Play]. Tick
        // first so continuous protections materialize on the new entrant.
        let suppressor = self.on_play_suppressor.take();
        let suppress_on_play = if suppress_on_play {
            match suppressor {
                Some((sp, sk)) => {
                    self.tick_declarative_effects();
                    !self.permanent_is_unaffected_by_effect(entered, sp, sk)
                }
                // No recorded suppressor (raw-rust / legacy callers):
                // preserve the unconditional skip.
                None => true,
            }
        } else {
            false
        };
        self.enter_deferred_drain();
        if !suppress_on_play {
            // `fire_on_play` uses `maybe_drain_effect_queue` internally,
            // which no-ops while `draining_deferred > 0` — so OnPlay
            // triggers enqueue but don't drain until the outer
            // `exit_deferred_drain_and_flush` below.
            //
            // Expose the effect-driven play source to the OnPlay trigger
            // context so a `played_by_effect` predicate can gate "if played
            // by an effect, …" (BT25-080). The flag is consumed when the
            // `Permanent` OnPlay trigger context is built; clear it right
            // after enqueue so no later trigger inherits it.
            self.pending_play_effect_initiated = effect_initiated;
            self.fire_on_play(player_id, field_index);
            self.pending_play_effect_initiated = false;
        }
        self.enqueue_triggered(
            EffectTiming::OnEnterFieldAnyone,
            TriggerSource::EnteredField {
                player: player_id,
                permanent: entered,
                card: entered_card,
                effect_initiated,
            },
        );
        self.enqueue_triggered(
            EffectTiming::OnAllyPlayed,
            TriggerSource::EnteredField {
                player: player_id,
                permanent: entered,
                card: entered_card,
                effect_initiated,
            },
        );
        self.exit_deferred_drain_and_flush();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s hand slot
    /// `hand_index`. Returns `true` if a matching effect fired, `false` if no
    /// `EffectTiming::MainFromHand`/`OptionMain` effect on the card was legal.
    ///
    /// Consumes `HAND_EFFECT` action bits (30-59) that the mask emits. Memory
    /// cost, card movement, and any side effects are handled inside the
    /// effect's `process` closure — mirroring Python's
    /// `_execute_hand_main_effect`. First-match-wins: once an effect fires we
    /// stop iterating, matching the mask's own first-match-wins emission.
    ///
    /// Hand/Trash per-turn activation counters (§4.5c-residual 🟡) are not
    /// tracked here; see docs/RUST_PYTHON_PARITY.md §4.5c.
    pub fn activate_hand_main(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        let (card_id, handle, card_kind) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.hand.get(hand_index) {
                Some(c) => c,
                None => return false,
            };
            (
                card.card_id(&self.card_data).to_string(),
                card.handle(),
                card.card_kind(&self.card_data),
            )
        };

        // Use `effects_for_card` rather than the raw registry so that
        // keyword-derived auto-installed effects are visible here. The
        // action mask uses the same accessor — without this, the mask
        // could emit a Hand [Main] bit for an auto-installed keyword
        // that this dispatcher could not honor.
        let effects = match self.effects_for_card(&card_id, handle) {
            Some(e) => e,
            None => return false,
        };

        let main_timing_matches = |timing: EffectTiming| {
            timing == EffectTiming::MainFromHand
                || (matches!(card_kind, CardKind::Option | CardKind::Dual)
                    && timing == EffectTiming::OptionMain)
        };

        for effect in effects.iter() {
            if !main_timing_matches(effect.timing) {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(self, handle, None, player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, None, player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
    }

    /// Activate a `[Main]` effect on the permanent at `player_id`'s battle-area
    /// slot `field_index`. Returns `true` if a matching effect fired.
    ///
    /// Consumes `FIELD_EFFECT` bits at sub-slot `FIELD_EFFECT_SLOT_FOR_MAIN`
    /// (per-permanent base + 2). Walks the digivolution stack bottom-up,
    /// applying the inherited-vs-top filter used by
    /// [`Game::source_dp_contribution`] so a given Field [Main] effect only
    /// fires on the same source/position the mask emitted from. Honors OPT via
    /// [`Permanent::activation_count`] and records activation on success so a
    /// subsequent mask rebuild sees the bit suppressed.
    ///
    /// Mirrors Python's `_execute_field_main_effect`.
    pub fn activate_field_main(&mut self, player_id: PlayerId, field_index: usize) -> bool {
        // Phase F Task 6 — breeding-area Training dispatch.
        //
        // `field_index == BREEDING_TARGET (=14)` routes to the breeding-area
        // path. The mask only emits this bit for `<Training>`-bearing
        // breeding-area carriers (RULES_CONTEXT 16-40 — only Training
        // activates from breeding); the dispatcher here independently
        // re-checks the gate (printed Training keyword on top + carrier not
        // suspended) and runs only the `<Training>` effect (filtered by
        // `effect.name == "<Training>"`) so a stale or hand-rolled
        // `MainOnField` on the same card cannot leak through.
        if field_index == crate::action::space::BREEDING_TARGET as usize {
            return self.activate_breeding_main_training(player_id);
        }

        // PUPPETS-G009 — standard `<Delay>` `[Main]`-phase activation. A
        // parked `DelayTrigger::MainPhaseActivated` Option whose placing turn
        // has passed is activated by trashing it as the cost and running its
        // stored `<Delay>` body. Dispatched before the ordinary `MainOnField`
        // scan because the Delay body lives at `EffectTiming::DelayEffect`,
        // not `MainOnField`.
        let delay_handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.delayed_option_main_activation_available(delay_handle) {
            return self.activate_delayed_option_main(delay_handle);
        }

        // Snapshot per-source identity without holding the battle_area borrow
        // across the effect closure invocations (which need `&mut self`).
        let (perm_handle, sources) = {
            let Some(player) = self.players.get(player_id as usize) else {
                return false;
            };
            let Some(perm) = player.battle_area.get(field_index) else {
                return false;
            };
            let stack_size = perm.card_sources.len();
            let handle = PermanentHandle {
                player: player_id,
                index: field_index as u8,
            };
            let mut infos: Vec<(bool, String, crate::card_source::CardHandle)> =
                Vec::with_capacity(stack_size);
            for (i, source) in perm.card_sources.iter().enumerate() {
                let is_under = i + 1 < stack_size;
                infos.push((
                    is_under,
                    source.card_id(&self.card_data).to_string(),
                    source.handle(),
                ));
            }
            (handle, infos)
        };

        for (is_under, card_id, source_handle) in sources {
            // Use `effects_for_card` rather than the raw registry so that
            // keyword-derived auto-installed effects (e.g. printed
            // `MaterialSave(N)`) are visible here. The action mask uses the
            // same accessor — without this, the mask would emit a Field
            // [Main] bit for an auto-installed keyword that this dispatcher
            // could not honor.
            let Some(effects) = self.effects_for_card(&card_id, source_handle) else {
                continue;
            };
            for (slot, effect) in effects.iter().enumerate() {
                if effect.timing != EffectTiming::MainOnField {
                    continue;
                }
                if is_under != effect.inherited {
                    continue;
                }
                if effect.max_per_turn > 0 {
                    let perm = &self.players[player_id as usize].battle_area[field_index];
                    if perm.activation_count(source_handle, slot as u8) >= effect.max_per_turn {
                        continue;
                    }
                }
                if let Some(cond) = &effect.condition {
                    let ctx =
                        EffectReadContext::new(self, source_handle, Some(perm_handle), player_id);
                    if !cond(&ctx) {
                        continue;
                    }
                }
                // Python records activation before invoking the callback so a
                // panic inside the process still counts toward OPT. Mirror that.
                if let Some(perm) = self.players[player_id as usize]
                    .battle_area
                    .get_mut(field_index)
                {
                    perm.record_activation(source_handle, slot as u8);
                }
                if let Some(process) = &effect.process {
                    let mut ctx =
                        EffectContext::new(self, source_handle, Some(perm_handle), player_id);
                    process(&mut ctx);
                }
                return true;
            }
        }
        false
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s trash slot
    /// `trash_index`. Returns `true` if a matching effect fired.
    ///
    /// Consumes `TRASH_EFFECT` action bits (1150-1194). Mirrors Python's
    /// `_execute_trash_main_effect`: memory cost and any card movement happen
    /// inside the effect's process closure, and there is no per-turn
    /// activation counter (§4.5c-residual 🟡).
    pub fn activate_trash_main(&mut self, player_id: PlayerId, trash_index: usize) -> bool {
        let (card_id, handle) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.trash.get(trash_index) {
                Some(c) => c,
                None => return false,
            };
            (card.card_id(&self.card_data).to_string(), card.handle())
        };

        // Use `effects_for_card` rather than the raw registry so that
        // keyword-derived auto-installed effects are visible here. The
        // action mask uses the same accessor — without this, the mask
        // could emit a Trash [Main] bit for an auto-installed keyword
        // that this dispatcher could not honor.
        let effects = match self.effects_for_card(&card_id, handle) {
            Some(e) => e,
            None => return false,
        };

        for effect in effects.iter() {
            if effect.timing != EffectTiming::MainFromTrash {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(self, handle, None, player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, None, player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
    }
}
