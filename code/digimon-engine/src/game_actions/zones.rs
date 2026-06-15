//! Zone movement (hand/deck/reveal/security/shuffle) operations (Tier 2) — `impl Game`.

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
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
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
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Move a specific card from `player_id`'s security stack to their hand.
    /// Returns false if the handle is not in that player's security stack.
    pub fn add_to_hand_from_security(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(idx) = self
            .player(player_id)
            .security
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        // Opaque-aware: materialize before moving to hand so the
        // resulting hand card has a real identity.
        self.ensure_security_materialized(player_id, idx);
        let removed = self.player_mut(player_id).security.remove(idx);
        let owner = removed.owner;
        self.player_mut(player_id)
            .face_up_security
            .remove(&removed.card_index);
        self.player_mut(owner).add_to_hand(removed);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
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
            // Opaque-aware: opaque players' reveal-from-top consumes
            // RevealKind::Effect (peek effects, not draws/security/mill).
            // For standard players this is a plain `deck.pop()`.
            let card = match self
                .take_from_deck_top_for_player(player_id, crate::opaque_deck::RevealKind::Effect)
            {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(e) => {
                    eprintln!(
                        "[opaque-deck] reveal_top_deck error for player {}: {}",
                        player_id, e
                    );
                    break;
                }
            };
            handles.push(card.handle());
            let card_id = card.card_id(&self.card_data).to_string();
            let card_name = card.card_name(&self.card_data).to_string();
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Reveal {
                seq,
                player: player_id,
                card_id,
                card_name,
                source_zone: crate::events::RevealZone::DeckTop,
            });
            self.revealed_cards.push(card);
        }
        handles
    }

    pub fn reveal_top_digitama(
        &mut self,
        player_id: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        let mut handles = Vec::new();
        for _ in 0..n {
            let p = self.player_mut(player_id);
            let Some(card) = p.digitama_deck.pop() else {
                break;
            };
            handles.push(card.handle());
            let card_id = card.card_id(&self.card_data).to_string();
            let card_name = card.card_name(&self.card_data).to_string();
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Reveal {
                seq,
                player: player_id,
                card_id,
                card_name,
                source_zone: crate::events::RevealZone::DeckTop,
            });
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

    /// Shuffle `player_id`'s security stack without changing its contents.
    pub fn shuffle_security(&mut self, player_id: PlayerId) {
        let mut security = std::mem::take(&mut self.player_mut(player_id).security);
        security.shuffle(&mut self.rng);
        self.player_mut(player_id).security = security;
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
        let mut taken = self.revealed_cards.remove(pos);
        taken.clear_reveal_overlay();
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
        let mut taken = self.revealed_cards.remove(pos);
        taken.clear_reveal_overlay();
        self.player_mut(player_id).trash.push(taken);
        true
    }
}
