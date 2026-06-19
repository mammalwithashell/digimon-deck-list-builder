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
        if cost == 0 {
            return true;
        }
        let new_memory = self.memory - cost as i16;
        if new_memory < self.rules.memory_range.0 {
            return false;
        }
        let delta = new_memory - self.memory;
        self.memory = new_memory;
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
        true
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
