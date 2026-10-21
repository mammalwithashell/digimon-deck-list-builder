//! Memory gain/loss/pay (Tier 1).

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

impl Game {
    /// Pay memory cost. Returns `true` if affordable (memory stays above
    /// `rules.memory_range.0`).
    ///
    /// Does **not** end the turn even if memory crosses zero. Python's rule:
    /// the turn only ends when `check_turn_end()` is called (typically after
    /// all OnPlay/WhenDigivolving/etc. effects have resolved). Callers should
    /// invoke `check_turn_end()` at the natural resolution boundary.
    pub fn pay_memory(&mut self, cost: u16) -> bool {
        self.pay_memory_for(self.turn_player(), cost)
    }

    /// Check a payment against the paying player's side of the memory gauge.
    pub fn can_pay_memory_for(&self, player: PlayerId, cost: u16) -> bool {
        let signed_cost = if player == self.turn_player() { -(cost as i32) } else { cost as i32 };
        let after = self.memory as i32 + signed_cost;
        after >= self.rules.memory_range.0 as i32 && after <= self.rules.memory_range.1 as i32
    }

    /// Pay without ending the turn, including effect plays by the inactive player.
    pub fn pay_memory_for(&mut self, player: PlayerId, cost: u16) -> bool {
        if !self.can_pay_memory_for(player, cost) { return false; }
        if cost == 0 { return true; }
        let signed_cost = if player == self.turn_player() { -(cost as i32) } else { cost as i32 };
        let new_memory = (self.memory as i32 + signed_cost) as i16;
        let delta = new_memory - self.memory;
        self.memory = new_memory;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
            source_card_id: None,
            source_card_name: None,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Whether `player` can gain memory from an effect right now — DCGO
    /// `Player.CanAddMemory(effect)`. `false` while the player has
    /// `CannotGainMemoryByEffect`, while they have
    /// `CannotGainMemoryExceptFromTamers` and the effect is not a Tamer
    /// effect, or while any permanent in their battle area carries
    /// `CannotAddMemory`. `EffectContext::gain_memory` consults this gate,
    /// and the DSL `can_gain_memory` predicate reads it for printed text
    /// conditioned on memory the effect gains (BT10-057).
    pub fn player_can_gain_memory_by_effect(
        &self,
        player: PlayerId,
        source_is_tamer: bool,
    ) -> bool {
        if self
            .modifiers
            .player_has(player, ModifierType::CannotGainMemoryByEffect)
        {
            return false;
        }
        if !source_is_tamer
            && self
                .modifiers
                .player_has(player, ModifierType::CannotGainMemoryExceptFromTamers)
        {
            return false;
        }
        let battle_area_len = self.player(player).battle_area.len();
        !(0..battle_area_len).any(|i| {
            self.modifiers.has(
                PermanentHandle {
                    player,
                    index: i as u8,
                },
                ModifierType::CannotAddMemory,
            )
        })
    }

    /// Gain memory for the active player.
    pub fn gain_memory(&mut self, amount: i16) {
        self.gain_memory_for_player(self.turn_player(), amount);
    }

    /// Gain memory for a specific player. The memory counter is stored from
    /// the current turn player's perspective, so non-turn-player gains move
    /// the counter toward the opponent's side.
    pub fn gain_memory_for_player(&mut self, player: PlayerId, amount: i16) {
        self.gain_memory_for_player_sourced(player, amount, None);
    }

    /// Gain memory for a specific player, attributing the change to an effect
    /// source `(card_id, card_name)`. Used by `EffectContext::gain_memory` /
    /// `lose_memory` so the emitted `MemoryChange` names the responsible card
    /// (e.g. a tamer's start-of-turn `+memory`). Structural / cost-payment
    /// paths call `gain_memory_for_player` (source `None`).
    pub fn gain_memory_for_player_sourced(
        &mut self,
        player: PlayerId,
        amount: i16,
        source: Option<(String, String)>,
    ) {
        let before = self.memory;
        let signed_amount = if player == self.turn_player() {
            amount
        } else {
            -amount
        };
        self.memory = (self.memory + signed_amount)
            .clamp(self.rules.memory_range.0, self.rules.memory_range.1);
        let delta = self.memory - before;
        let (source_card_id, source_card_name) = match source {
            Some((id, name)) => (Some(id), Some(name)),
            None => (None, None),
        };
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
            source_card_id,
            source_card_name,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Lose memory for a specific player. Rules manual p.7: "'Lose X
    /// memory' means moving the memory marker X spaces toward your
    /// opponent's side" — the exact mirror of a gain, relative to the
    /// losing player (DCGO: `Owner.AddMemory(-n)`).
    pub fn lose_memory_for_player(&mut self, player: PlayerId, amount: i16) {
        self.gain_memory_for_player(player, -amount);
    }

    /// Set memory to a specific value.
    pub fn set_memory(&mut self, value: i16) {
        let before = self.memory;
        self.memory = value.clamp(self.rules.memory_range.0, self.rules.memory_range.1);
        let delta = self.memory - before;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
            source_card_id: None,
            source_card_name: None,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }
}
