//! Player-driven game actions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! This is where `play_from_hand`, `digivolve_from_hand`, `initiate_dna_digivolve`,
//! and the `activate_*_main` [Main] effect dispatchers live. All three are invoked
//! by the action decoder and the Tauri/PyO3 bindings; none of them move here.

use crate::card_source::CardSource;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{EffectTiming, GamePhase, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind, TriggerSource};
use rand::seq::SliceRandom;

impl Game {
    /// Move from breeding to battle area for a player.
    pub fn move_from_breeding(&mut self, player_id: PlayerId) -> bool {
        let field_slots = self.rules.field_slots;
        let player = self.player_mut(player_id);
        if player.battle_area.len() >= field_slots as usize {
            return false;
        }
        if let Some(perm) = player.breeding_area.take() {
            player.battle_area.push(perm);
            true
        } else {
            false
        }
    }

    /// Play a card from hand to field for a player, paying the printed cost.
    ///
    /// Delegates to [`Self::play_from_hand_with_cost`] with
    /// `CostDelta::Reduce(0)` (pay the printed cost verbatim).
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> Option<usize> {
        self.play_from_hand_with_cost(player_id, hand_index, crate::enums::CostDelta::Reduce(0))
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
    ) -> Option<usize> {
        let turn = self.turn_count;
        let field_slots = self.rules.field_slots;

        // Borrow-check-friendly pre-checks: gather everything we need from
        // immutable borrows before taking a mutable borrow.
        let printed_cost = {
            let player = self.player(player_id);
            if hand_index >= player.hand.len() {
                return None;
            }
            if player.battle_area.len() >= field_slots as usize {
                return None;
            }
            player.hand[hand_index].play_cost(&self.card_data)
        };

        let effective_cost = cost_delta.resolve(printed_cost);

        // Pay the cost up-front. If unaffordable, do not remove the card.
        if !self.pay_memory(effective_cost) {
            return None;
        }

        // Now the cost is paid — commit the play.
        let player = self.player_mut(player_id);
        let card = player.hand.remove(hand_index);
        let perm = crate::permanent::Permanent::new(card, turn);
        player.battle_area.push(perm);
        let field_index = player.battle_area.len() - 1;

        // Emit Play event: permanent is on field, before OnPlay effects fire.
        let emitted_card_id = self.players[player_id as usize].battle_area[field_index]
            .top_card()
            .card_id(&self.card_data)
            .to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
            field_index: field_index as u8,
        });

        self.fire_on_play(player_id, field_index);

        // OnEnterFieldAnyone: global observer — fires in every player's battle
        // area after OnPlay resolves. Python mirror: OnEnterFieldAnyone timing.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnEnterFieldAnyone,
                crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
            );
        }
        self.drain_effect_queue();

        Some(field_index)
    }

    /// Play a card from `player`'s trash to field, paying the printed cost.
    ///
    /// Delegates to [`Self::play_from_trash_with_cost`] with
    /// `CostDelta::Reduce(0)` (pay the printed cost verbatim).
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_trash(&mut self, player_id: PlayerId, trash_index: usize) -> Option<usize> {
        self.play_from_trash_with_cost(player_id, trash_index, crate::enums::CostDelta::Reduce(0))
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
    ) -> Option<usize> {
        let turn = self.turn_count;
        let field_slots = self.rules.field_slots;

        let printed_cost = {
            let player = self.player(player_id);
            if trash_index >= player.trash.len() {
                return None;
            }
            if player.battle_area.len() >= field_slots as usize {
                return None;
            }
            player.trash[trash_index].play_cost(&self.card_data)
        };

        let effective_cost = cost_delta.resolve(printed_cost);

        if !self.pay_memory(effective_cost) {
            return None;
        }

        let player = self.player_mut(player_id);
        let card = player.trash.remove(trash_index);
        let perm = crate::permanent::Permanent::new(card, turn);
        player.battle_area.push(perm);
        let field_index = player.battle_area.len() - 1;

        let emitted_card_id = self.players[player_id as usize].battle_area[field_index]
            .top_card()
            .card_id(&self.card_data)
            .to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
            field_index: field_index as u8,
        });

        self.fire_on_play(player_id, field_index);

        // OnEnterFieldAnyone: global observer — fires in every player's battle
        // area after OnPlay resolves. Python mirror: OnEnterFieldAnyone timing.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnEnterFieldAnyone,
                crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
            );
        }
        self.drain_effect_queue();

        Some(field_index)
    }

    /// Move a specific card from `player`'s deck to their hand. Returns false
    /// if the handle isn't in the deck. Does NOT shuffle — callers that mirror
    /// the printed "search then shuffle" rule must call `shuffle_deck` after.
    pub fn add_to_hand_from_deck(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(removed) = self.player_mut(player_id).remove_from_deck_by_handle(card) else {
            return false;
        };
        self.player_mut(player_id).add_to_hand(removed);
        true
    }

    /// Move a specific card from `player`'s trash to their hand.
    pub fn add_to_hand_from_trash(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(removed) = self.player_mut(player_id).remove_from_trash_by_handle(card) else {
            return false;
        };
        self.player_mut(player_id).add_to_hand(removed);
        true
    }

    /// Reveal up to `n` cards from the top of `player`'s deck. Cards move
    /// into `self.revealed_cards` (transient reveal pool, cleared on turn
    /// rotation). Returns the list of revealed card handles in top-first
    /// order.
    ///
    /// Does not fire `OnDraw` or modify hand. Callers that want to then
    /// move a revealed card to hand/deck/trash use the reveal-pool
    /// follow-up helpers added in Task 9.
    pub fn reveal_top_deck(
        &mut self,
        player_id: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        let mut handles = Vec::new();
        for _ in 0..n {
            let p = self.player_mut(player_id);
            let Some(card) = p.deck.pop() else { break };
            handles.push(card.handle());
            self.revealed_cards.push(card);
        }
        handles
    }

    /// Shuffle `player`'s deck.
    pub fn shuffle_deck(&mut self, player_id: PlayerId) {
        // Split-borrow idiom: take deck out, shuffle, put back.
        let mut deck = std::mem::take(&mut self.player_mut(player_id).deck);
        deck.shuffle(&mut self.rng);
        self.player_mut(player_id).deck = deck;
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
        self.enqueue_triggered(
            EffectTiming::OnPlay,
            TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s hand slot
    /// `hand_index`. Returns `true` if a matching effect fired, `false` if no
    /// `EffectTiming::MainFromHand` effect on the card was legal.
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
        let (card_id, handle) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.hand.get(hand_index) {
                Some(c) => c,
                None => return false,
            };
            (card.card_id(&self.card_data).to_string(), card.handle())
        };

        let effect_impl = match self.effect_registry.get(&card_id) {
            Some(arc) => arc,
            None => return false,
        };
        let effects = effect_impl.effects(handle);

        for effect in &effects {
            if effect.timing != EffectTiming::MainFromHand {
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
            let Some(effect_impl) = self.effect_registry.get(&card_id) else {
                continue;
            };
            let effects = effect_impl.effects(source_handle);
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
                    let mut ctx = EffectContext::new(
                        self,
                        source_handle,
                        Some(perm_handle),
                        player_id,
                    );
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

        let effect_impl = match self.effect_registry.get(&card_id) {
            Some(arc) => arc,
            None => return false,
        };
        let effects = effect_impl.effects(handle);

        for effect in &effects {
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

    /// Trash a specific hand card by index. Returns the trashed card's handle
    /// on success, None if the index is out of range.
    pub fn trash_from_hand_by_index(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> Option<crate::card_source::CardHandle> {
        let player = self.player_mut(player_id);
        if hand_index >= player.hand.len() {
            return None;
        }
        let card = player.hand.remove(hand_index);
        let h = card.handle();
        player.trash.push(card);
        Some(h)
    }

    /// Move a specific revealed card (identified by `card` handle) into
    /// `player`'s hand. Returns false if the handle is not in
    /// `self.revealed_cards`.
    pub fn add_to_hand_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(pos) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let taken = self.revealed_cards.remove(pos);
        self.player_mut(player_id).hand.push(taken);
        true
    }

    /// Move a specific revealed card into `player`'s trash.
    pub fn trash_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(pos) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let taken = self.revealed_cards.remove(pos);
        self.player_mut(player_id).trash.push(taken);
        true
    }

    /// Move a specific revealed card back to `player`'s deck at `position`.
    /// Returns false if the handle is not in the reveal pool.
    pub fn return_to_deck_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        let Some(pos_idx) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let taken = self.revealed_cards.remove(pos_idx);
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, taken);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 { 0 } else { self.rng.gen_range(0..=deck_len) };
                self.player_mut(player_id).deck.insert(idx, taken);
            }
        }
        true
    }

    /// Digivolve: push a card onto a permanent's stack.
    pub fn digivolve_onto(
        &mut self,
        player_id: PlayerId,
        field_index: usize,
        card: CardSource,
    ) -> bool {
        let turn = self.turn_count;
        let player = self.player_mut(player_id);
        if field_index >= player.battle_area.len() {
            return false;
        }
        player.battle_area[field_index].digivolve(card, turn);
        true
    }

    /// Bounce a permanent to its owner's hand: the top card moves to hand,
    /// every card beneath it goes to the owner's trash (per DCGO leave-field
    /// rules). Linked cards go to trash. Returns the handle of the card that
    /// ended up in hand.
    ///
    /// Does not fire OnLeaveField observers — that's Phase 1 timing-dispatch
    /// infrastructure. Modifiers targeting the returned permanent are cleared.
    pub fn return_to_hand(
        &mut self,
        handle: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        let player = self.player_mut(handle.player);
        if (handle.index as usize) >= player.battle_area.len() {
            return None;
        }
        let perm = player.battle_area.remove(handle.index as usize);

        let mut sources = perm.card_sources;
        let top = sources.pop()?;
        let top_handle = top.handle();
        player.hand.push(top);

        for card in sources {
            player.trash.push(card);
        }
        for card in perm.linked_cards {
            player.trash.push(card);
        }

        self.modifiers.clear_permanent(handle);
        Some(top_handle)
    }

    /// Return a permanent's top card to its owner's deck at `position`.
    /// Sources under the top go to trash; linked_cards go to trash.
    /// Modifiers targeting the permanent are cleared. Returns true on
    /// success, false if the handle is invalid or the stack is empty.
    ///
    /// Does not fire OnLeaveField observers.
    pub fn return_to_deck(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        let player_id = handle.player;
        {
            let player = self.player_mut(player_id);
            if (handle.index as usize) >= player.battle_area.len() {
                return false;
            }
        }
        let mut perm = self.player_mut(player_id).battle_area.remove(handle.index as usize);

        let Some(top) = perm.card_sources.pop() else {
            return false;
        };

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(top);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, top);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 { 0 } else { self.rng.gen_range(0..=deck_len) };
                self.player_mut(player_id).deck.insert(idx, top);
            }
        }

        // Sources under the top go to trash.
        for card in perm.card_sources {
            self.player_mut(player_id).trash.push(card);
        }
        for card in perm.linked_cards {
            self.player_mut(player_id).trash.push(card);
        }

        self.modifiers.clear_permanent(handle);
        true
    }

    /// Full "digivolve from hand" action — Python parity for
    /// `action_digivolve(field_idx, hand_idx)`. Validates phase, indices,
    /// `CannotDigivolve` modifier, and evo-cost fit; pays memory; removes
    /// the card from hand; stacks it onto the target permanent; draws 1;
    /// fires `WhenDigivolving` triggers and drains the effect queue;
    /// finally calls `check_turn_end`.
    ///
    /// Deferred (see RUST_PYTHON_PARITY.md):
    /// - Cost reductions (`WhenWouldDigivolve`, `CHANGE_DIGIVOLUTION_COST`)
    /// - `digivolve_observer` mechanism (no Rust equivalent yet)
    /// - Contextual modifier predicates (`{'digivolving_card': card}`)
    pub fn digivolve_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        if field_index >= player.battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: field index {} out of range (battle_area size={})",
                field_index,
                player.battle_area.len()
            ));
            return false;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: permanent at field index {} blocked by CannotDigivolve modifier",
                field_index
            ));
            return false;
        }

        let card = player.hand[hand_index].clone();
        let perm = &player.battle_area[field_index];
        if !self.can_digivolve(&card, perm) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: card {} cannot digivolve onto {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                perm.top_card().card_id(&self.card_data),
            ));
            return false;
        }

        let base_level = perm.top_card().level(&self.card_data).unwrap();
        let base_colors = perm.top_card().colors(&self.card_data);
        let evo_costs = &self.card_data[card.data_index].evo_costs;
        let cost = evo_costs
            .iter()
            .filter(|ec| {
                ec.level == base_level
                    && crate::action::mask::evo_color(ec.card_color)
                        .map(|c| base_colors.contains(&c))
                        .unwrap_or(false)
            })
            .map(|ec| ec.memory_cost)
            .min()
            .expect("can_digivolve guarantees at least one matching evo_cost");

        if !self.pay_memory(cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: cannot pay memory cost {} (current memory={})",
                cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        let removed = self.player_mut(player_id).hand.remove(hand_index);
        self.player_mut(player_id).battle_area[field_index].digivolve(removed, turn);

        self.player_mut(player_id).draw();

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — fires in every player's battle area
        // after the evolving permanent's WhenDigivolving resolves. Distinct
        // from WhenDigivolving (self-timing on the evolving permanent).
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        self.check_turn_end();
        true
    }

    /// Digivolve a hand card onto the breeding-area permanent. Python
    /// parity for `action_digivolve_breeding(hand_idx)` — same flow as
    /// `digivolve_from_hand` minus the trigger/observer firing (breeding
    /// digivolve does NOT fire `WhenDigivolving`).
    pub fn digivolve_from_hand_onto_breeding(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        let Some(breeding) = player.breeding_area.as_ref() else {
            self.logger.log("[Rejected] digivolve_breeding: breeding area is empty");
            return false;
        };

        let card = player.hand[hand_index].clone();
        if !self.can_digivolve(&card, breeding) {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: card {} cannot digivolve onto breeding {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                breeding.top_card().card_id(&self.card_data),
            ));
            return false;
        }

        let base_level = breeding.top_card().level(&self.card_data).unwrap();
        let base_colors = breeding.top_card().colors(&self.card_data);
        let evo_costs = &self.card_data[card.data_index].evo_costs;
        let cost = evo_costs
            .iter()
            .filter(|ec| {
                ec.level == base_level
                    && crate::action::mask::evo_color(ec.card_color)
                        .map(|c| base_colors.contains(&c))
                        .unwrap_or(false)
            })
            .map(|ec| ec.memory_cost)
            .min()
            .expect("can_digivolve guarantees at least one matching evo_cost");

        if !self.pay_memory(cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: cannot pay memory cost {} (current memory={})",
                cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        let removed = self.player_mut(player_id).hand.remove(hand_index);
        let player_mut = self.player_mut(player_id);
        if let Some(breeding) = player_mut.breeding_area.as_mut() {
            breeding.digivolve(removed, turn);
        }
        player_mut.draw();

        // Breeding digivolve does NOT fire WhenDigivolving (Python parity).
        self.check_turn_end();
        true
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. The
    /// source card is taken from the zone specified by `source` (hand slot,
    /// trash slot, deck top, or reveal pool). Returns false if the source
    /// or target is invalid.
    ///
    /// On target-invalid after source-taken: the taken card is routed to
    /// the target player's trash as a safe-failure mode (source already
    /// mutated; no way to roll back).
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
    ) -> bool {
        // Take the card out of its source zone.
        let taken = match source {
            crate::enums::CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                if i >= player.hand.len() {
                    return false;
                }
                player.hand.remove(i)
            }
            crate::enums::CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                if i >= player.trash.len() {
                    return false;
                }
                player.trash.remove(i)
            }
            crate::enums::CardSourceRef::DeckTop(p) => {
                let Some(c) = self.player_mut(p).deck.pop() else {
                    return false;
                };
                c
            }
            crate::enums::CardSourceRef::Reveal(h) => {
                let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h) else {
                    return false;
                };
                self.revealed_cards.remove(idx)
            }
        };

        // Push under the target permanent.
        let target_player = self.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            // Source already mutated — safe-fail by routing to trash.
            target_player.trash.push(taken);
            return false;
        }
        target_player.battle_area[target.index as usize].push_under(taken);
        true
    }

    /// Install a `SelectMaterial` pending selection for DNA digivolve.
    /// Python parity for `_initiate_dna_digivolve(hand_idx)`. The
    /// second-material selection + actual digivolve execution is stubbed
    /// inside the callback and tracked as `TODO(dna-digivolve-execute)`.
    pub fn initiate_dna_digivolve(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        let card = player.hand[hand_index].clone();
        let evo_meta = &self.card_data[card.data_index];
        if evo_meta.dna_costs.is_empty() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: card {} has no DNA costs",
                card.card_id(&self.card_data)
            ));
            return false;
        }
        if !crate::dna_digivolve::has_valid_dna_targets(
            evo_meta,
            &player.battle_area,
            &self.card_data,
        ) {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: no valid DNA material pair for {}",
                card.card_id(&self.card_data)
            ));
            return false;
        }

        // Collect valid first-material battle_area indices: those that
        // appear in at least one valid pair (either ordering).
        let mut first_targets: Vec<u16> = Vec::new();
        for i in 0..player.battle_area.len() {
            for j in 0..player.battle_area.len() {
                if i == j {
                    continue;
                }
                if crate::dna_digivolve::can_dna_digivolve(
                    evo_meta,
                    &player.battle_area[i],
                    &player.battle_area[j],
                    &self.card_data,
                ) {
                    first_targets.push(i as u16);
                    break;
                }
            }
        }
        first_targets.sort();
        first_targets.dedup();
        if first_targets.is_empty() {
            self.logger.log(
                "[Rejected] initiate_dna_digivolve: no valid first-material indices after filter",
            );
            return false;
        }

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectMaterial;

        let selecting_player = player_id;
        let source_card = card.handle();

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Material,
            selecting_player,
            previous_phase,
            valid_action_ids: first_targets,
            is_optional: false,
            prompt: "Select first DNA material".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // TODO(dna-digivolve-execute): decode first material
                // index, install second-material selection, and on its
                // resolution remove both materials from battle_area,
                // create a new permanent carrying the evo card with
                // both materials in the stack, pay the DnaCost
                // memory_cost, fire WhenDigivolving. See
                // `action_decoder.py::_dna_select_second` +
                // `player.dna_digivolve` for the Python reference.
                let _ = (game, action_id);
            }),
            on_decline: None,
        });
        true
    }

    /// Move a card from `source` to `player_id`'s security stack at the given
    /// `position` (Top, Bottom, Random). If `face_up` is true, the card's
    /// `card_index` is inserted into `face_up_security` so subsequent reveals
    /// know it was placed face-up. Returns false if the source index is invalid.
    ///
    /// Does not fire `OnLoseSecurity` or any security-related observers.
    pub fn place_on_security(
        &mut self,
        player_id: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        // Take the card out of its source zone. Mirror the pattern from
        // place_as_bottom_source.
        let taken = match source {
            crate::enums::CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                if i >= player.hand.len() {
                    return false;
                }
                player.hand.remove(i)
            }
            crate::enums::CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                if i >= player.trash.len() {
                    return false;
                }
                player.trash.remove(i)
            }
            crate::enums::CardSourceRef::DeckTop(p) => {
                let Some(c) = self.player_mut(p).deck.pop() else {
                    return false;
                };
                c
            }
            crate::enums::CardSourceRef::Reveal(h) => {
                let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h) else {
                    return false;
                };
                self.revealed_cards.remove(idx)
            }
        };

        // face_up_security is HashSet<u16> keyed by card_index.
        let face_up_key = taken.card_index;

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, taken);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                // Split-borrow: read length from immutable borrow first, then
                // mutably insert — mirrors the pattern in return_to_deck.
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 { 0 } else { self.rng.gen_range(0..=sec_len) };
                self.player_mut(player_id).security.insert(idx, taken);
            }
        }

        if face_up {
            self.player_mut(player_id).face_up_security.insert(face_up_key);
        }
        true
    }

    /// Script-initiated digivolve: place the card at `hand_index` from
    /// `player_id`'s hand onto `target`, bypassing the phase check and
    /// optionally the color check. Memory is paid according to `cost_delta`.
    ///
    /// Unlike `digivolve_from_hand`, this does **not** check `GamePhase::Main`
    /// or fire `check_turn_end` — it is designed for use inside effect
    /// callbacks where those invariants don't apply. It also does **not**
    /// draw a card (that's a player-action benefit, not an effect mechanic).
    ///
    /// Returns `true` on success, `false` if validation fails (bad index,
    /// no matching evo cost, or insufficient memory).
    pub fn effect_initiated_digivolve(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> bool {
        // 1. Validate hand index and target index.
        {
            let player = self.player(player_id);
            if hand_index >= player.hand.len() {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: hand index {} out of range (hand size={})",
                    hand_index,
                    player.hand.len()
                ));
                return false;
            }
        }
        {
            let target_player = self.player(target.player);
            if (target.index as usize) >= target_player.battle_area.len() {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: target index {} out of range (battle_area size={})",
                    target.index,
                    target_player.battle_area.len()
                ));
                return false;
            }
        }

        // 2. Find a matching evo cost.
        let (evo_card_data_index, base_level, base_colors) = {
            let player = self.player(player_id);
            let card = &player.hand[hand_index];
            let target_player = self.player(target.player);
            let perm = &target_player.battle_area[target.index as usize];
            let Some(base_level) = perm.top_card().level(&self.card_data) else {
                self.logger.log(
                    "[Rejected] effect_initiated_digivolve: target top card has no level",
                );
                return false;
            };
            let base_colors = perm.top_card().colors(&self.card_data);
            (card.data_index, base_level, base_colors)
        };

        let evo_costs = &self.card_data[evo_card_data_index].evo_costs;
        let matching_cost = evo_costs.iter().find(|ec| {
            ec.level == base_level
                && (ignore_color
                    || crate::action::mask::evo_color(ec.card_color)
                        .map(|c| base_colors.contains(&c))
                        .unwrap_or(false))
        });
        let Some(matching) = matching_cost else {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: no matching evo cost (base_level={}, ignore_color={})",
                base_level, ignore_color
            ));
            return false;
        };
        let effective_cost = cost_delta.resolve(matching.memory_cost);

        // 3. Pay memory.
        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            return false;
        }

        // 4. Move the card from hand onto the target permanent's stack.
        let turn = self.turn_count;
        let card = self.player_mut(player_id).hand.remove(hand_index);
        self.player_mut(target.player).battle_area[target.index as usize].digivolve(card, turn);

        // 5. Fire WhenDigivolving triggers.
        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(target),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — fires in every player's battle area
        // after the evolving permanent's WhenDigivolving resolves. Distinct
        // from WhenDigivolving (self-timing on the evolving permanent).
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        true
    }
}
