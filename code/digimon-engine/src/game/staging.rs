//! Test-fixture staging (Tier 1).

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
    /// Place a full digivolution stack (bottom-to-top) on `player`'s field
    /// with explicit suspend state and turn-played value. Returns the new
    /// battle-area index. Panics on an unknown card id.
    pub fn stage_place_field_stack(
        &mut self,
        player: PlayerId,
        card_ids: &[&str],
        suspended: bool,
        turn_played: u16,
    ) -> usize {
        self.stage_place_field_stack_with_links(player, card_ids, suspended, turn_played, &[])
    }

    /// Place a full digivolution stack (bottom-to-top) on `player`'s field
    /// with explicit suspend state, turn-played value, and optional linked
    /// cards. Returns the new battle-area index. Panics on an unknown card id.
    pub fn stage_place_field_stack_with_links(
        &mut self,
        player: PlayerId,
        card_ids: &[&str],
        suspended: bool,
        turn_played: u16,
        linked_card_ids: &[&str],
    ) -> usize {
        assert!(
            !card_ids.is_empty(),
            "stage_place_field_stack requires at least one card id"
        );
        let mut sources = Vec::with_capacity(card_ids.len());
        for card_id in card_ids {
            let data_idx = self
                .card_data
                .iter()
                .position(|c| c.card_id == *card_id)
                .unwrap_or_else(|| panic!("stage_place_field_stack: unknown card_id {card_id}"));
            let next_idx = self.next_card_index();
            let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
            card.card_index = next_idx;
            sources.push(card);
        }
        // Bottom-to-top: first id is the bottom source, last is the top
        // card. `Permanent::new` takes the top card; remaining go beneath.
        let top = sources.pop().expect("non-empty checked above");
        let mut perm = crate::permanent::Permanent::new(top, turn_played);
        // Insert the lower sources beneath the top, preserving order.
        for (i, src) in sources.into_iter().enumerate() {
            perm.card_sources.insert(i, src);
        }
        for linked_id in linked_card_ids {
            let data_idx = self
                .card_data
                .iter()
                .position(|c| c.card_id == *linked_id)
                .unwrap_or_else(|| {
                    panic!("stage_place_field_stack_with_links: unknown linked card_id {linked_id}")
                });
            let next_idx = self.next_card_index();
            let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
            card.card_index = next_idx;
            perm.linked_cards.push(card);
        }
        perm.is_suspended = suspended;
        perm.turn_played = turn_played;
        self.players[player as usize].battle_area.push(perm);
        self.players[player as usize].battle_area.len() - 1
    }

    /// Inject a single card directly into a named zone for `player`.
    /// `zone` ∈ {"hand", "deck_top", "egg_deck_top", "security_top",
    /// "security_top_face_up", "trash"}. Returns `Err` on an unknown card id
    /// or unknown zone.
    pub fn stage_inject_card(
        &mut self,
        player: PlayerId,
        card_id: &str,
        zone: &str,
    ) -> Result<(), String> {
        let data_idx = self
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .ok_or_else(|| format!("stage_inject_card: unknown card_id {card_id}"))?;
        let next_idx = self.next_card_index();
        let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
        card.card_index = next_idx;
        let p = &mut self.players[player as usize];
        match zone {
            "hand" => p.put_in_hand(card),
            // Deck top = end of the vec (draw pops from the end).
            "deck_top" => p.put_on_deck_top(card),
            // Egg deck top = end of the vec (hatch pops from the end).
            "egg_deck_top" => p.put_on_digi_egg_deck_top(card),
            // Security top = end of the vec.
            "security_top" => p.put_on_security_top(card),
            "security_top_face_up" => {
                let cidx = card.card_index;
                p.put_on_security_top(card);
                p.face_up_security.insert(cidx);
            }
            "trash" => p.put_in_trash(card),
            other => return Err(format!("stage_inject_card: unknown zone {other}")),
        }
        Ok(())
    }

    /// Set whether a security card at `top_index` (0 = top of security) is
    /// face-up.
    pub fn stage_set_security_face_up(
        &mut self,
        player: PlayerId,
        top_index: usize,
        face_up: bool,
    ) -> Result<(), String> {
        let p = &mut self.players[player as usize];
        let len = p.security.len();
        if top_index >= len {
            return Err(format!(
                "stage_set_security_face_up: top_index {top_index} out of range (len {len})"
            ));
        }
        let vec_idx = len - 1 - top_index;
        let cidx = p.security[vec_idx].card_index;
        if face_up {
            p.face_up_security.insert(cidx);
        } else {
            p.face_up_security.remove(&cidx);
        }
        Ok(())
    }

    /// Make `player` the active turn player, preserving the seesaw and
    /// turn-rotation invariants. Reorders `turn_order` so `player` leads,
    /// points `turn_player_idx` at it, resets `memory_pair` to (active,
    /// next), and realigns any still-pending mulligan order.
    pub fn stage_set_first_player(&mut self, player: PlayerId) {
        if let Some(pos) = self.turn_order.iter().position(|&p| p == player) {
            self.turn_order.rotate_left(pos);
        }
        self.turn_player_idx = 0;
        let next = if self.turn_order.len() >= 2 {
            self.turn_order[1]
        } else {
            self.turn_order[0]
        };
        self.memory_pair = (player, next);
        if !self.mulligan_pending.is_empty() {
            self.mulligan_pending = self.turn_order.clone();
        }
    }

    /// Place a digivolution stack (bottom-to-top) into `player`'s breeding
    /// area, replacing whatever is there. Panics on an unknown card id.
    pub fn stage_place_in_breeding(&mut self, player: PlayerId, card_ids: &[&str]) {
        assert!(
            !card_ids.is_empty(),
            "stage_place_in_breeding requires at least one card id"
        );
        let mut sources = Vec::with_capacity(card_ids.len());
        for card_id in card_ids {
            let data_idx = self
                .card_data
                .iter()
                .position(|c| c.card_id == *card_id)
                .unwrap_or_else(|| panic!("stage_place_in_breeding: unknown card_id {card_id}"));
            let next_idx = self.next_card_index();
            let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
            card.card_index = next_idx;
            sources.push(card);
        }
        let top = sources.pop().expect("non-empty checked above");
        let mut perm = crate::permanent::Permanent::new(top, self.turn_count);
        for (i, src) in sources.into_iter().enumerate() {
            perm.card_sources.insert(i, src);
        }
        self.players[player as usize].breeding_area = Some(perm);
    }

    /// Empty a named zone for `player` so staged contents fully replace
    /// whatever was dealt at construction. `zone` ∈ {"hand", "deck",
    /// "egg_deck", "security", "trash", "battle_area", "breeding"}. Returns
    /// `Err` on an unknown zone.
    pub fn stage_clear_zone(&mut self, player: PlayerId, zone: &str) -> Result<(), String> {
        let p = &mut self.players[player as usize];
        match zone {
            "hand" => p.hand.clear(),
            "deck" => p.deck.clear(),
            "egg_deck" => p.digitama_deck.clear(),
            "security" => {
                p.security.clear();
                p.face_up_security.clear();
            }
            "trash" => p.trash.clear(),
            "battle_area" => p.battle_area.clear(),
            "breeding" => p.breeding_area = None,
            other => return Err(format!("stage_clear_zone: unknown zone {other}")),
        }
        Ok(())
    }

    /// Validate that a staged board is internally consistent enough for
    /// the turn machine to operate on. Returns `Err(reason)` on a
    /// rule-illegal staged state so callers can fail loud.
    pub fn stage_validate(&self) -> Result<(), String> {
        if self.turn_order.is_empty() {
            return Err("turn_order is empty".to_string());
        }
        if self.turn_player_idx >= self.turn_order.len() {
            return Err(format!(
                "turn_player_idx {} out of range for turn_order len {}",
                self.turn_player_idx,
                self.turn_order.len()
            ));
        }
        if self.current_phase != GamePhase::Mulligan && !self.mulligan_pending.is_empty() {
            return Err(format!(
                "phase is {:?} but {} player(s) still owe a mulligan decision; \
                 finalize mulligan before setting the phase",
                self.current_phase,
                self.mulligan_pending.len()
            ));
        }
        for (pid, player) in self.players.iter().enumerate() {
            for (i, perm) in player.battle_area.iter().enumerate() {
                if perm.card_sources.is_empty() {
                    return Err(format!(
                        "player {pid} battle_area[{i}] has an empty card stack"
                    ));
                }
            }
        }
        Ok(())
    }
}
