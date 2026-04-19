use std::collections::HashSet;

use rand::seq::SliceRandom;
use rand::Rng;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::enums::PlayerId;
use crate::permanent::Permanent;
use crate::rules::Rules;

/// A player in the game with all their zones.
#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub hand: Vec<CardSource>,
    pub deck: Vec<CardSource>,
    pub digitama_deck: Vec<CardSource>,
    pub security: Vec<CardSource>,
    pub trash: Vec<CardSource>,
    pub battle_area: Vec<Permanent>,
    pub breeding_area: Option<Permanent>,

    /// `CardSource.card_index` of security cards currently face-up (visible
    /// to this player and encoded into the observation tensor). Security is
    /// laid face-down by default; effects that reveal security flip entries
    /// in here. Matches Python's `Player.face_up_security`.
    pub face_up_security: HashSet<u16>,

    /// Snapshot of the most recently revealed security card, set by
    /// `Game::resolve_security_card` before firing `SecuritySkill` effects.
    /// Consumed by `OnSecurityCheck` observer effects so they can inspect
    /// the revealed card even after `pending_security` has been cleared.
    /// Mirrors Python's `_last_security_card` / `_last_security_was_face_up`
    /// pair (RUST_PYTHON_PARITY §2.5l).
    pub last_security_reveal: Option<crate::selection::SecurityRevealSnapshot>,

    // Commander/multiplayer fields
    pub commander_zone: Option<CardSource>,
    pub commander_tax: u16,
    pub is_eliminated: bool,
}

impl Player {
    /// Create a new player with empty zones.
    pub fn new(id: PlayerId) -> Self {
        Self {
            id,
            hand: Vec::new(),
            deck: Vec::new(),
            digitama_deck: Vec::new(),
            security: Vec::new(),
            trash: Vec::new(),
            battle_area: Vec::new(),
            breeding_area: None,
            face_up_security: HashSet::new(),
            last_security_reveal: None,
            commander_zone: None,
            commander_tax: 0,
            is_eliminated: false,
        }
    }

    /// Draw one card from deck to hand. Returns false if deck is empty (deck-out).
    pub fn draw(&mut self) -> bool {
        if let Some(card) = self.deck.pop() {
            self.hand.push(card);
            true
        } else {
            false
        }
    }

    /// Remove the first card in `deck` matching `handle`. Returns the removed
    /// card if found.
    pub fn remove_from_deck_by_handle(
        &mut self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let pos = self.deck.iter().position(|c| c.handle() == handle)?;
        Some(self.deck.remove(pos))
    }

    /// Remove the first card in `trash` matching `handle`.
    pub fn remove_from_trash_by_handle(
        &mut self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let pos = self.trash.iter().position(|c| c.handle() == handle)?;
        Some(self.trash.remove(pos))
    }

    /// Append `card` to hand.
    pub fn add_to_hand(&mut self, card: crate::card_source::CardSource) {
        self.hand.push(card);
    }

    /// Draw multiple cards. Returns number actually drawn.
    pub fn draw_many(&mut self, count: u8) -> u8 {
        let mut drawn = 0;
        for _ in 0..count {
            if self.draw() {
                drawn += 1;
            } else {
                break;
            }
        }
        drawn
    }

    /// Shuffle the main deck.
    pub fn shuffle_deck(&mut self, rng: &mut impl Rng) {
        self.deck.shuffle(rng);
    }

    /// Shuffle the digitama deck.
    pub fn shuffle_digitama_deck(&mut self, rng: &mut impl Rng) {
        self.digitama_deck.shuffle(rng);
    }

    /// Set up the security stack by moving cards from the top of the deck.
    pub fn setup_security(&mut self, count: u8) {
        for _ in 0..count {
            if let Some(card) = self.deck.pop() {
                self.security.push(card);
            }
        }
    }

    /// Hatch: move top card from digitama deck to breeding area.
    /// Returns true if successful (digitama deck not empty, breeding area empty).
    pub fn hatch(&mut self, turn: u16) -> bool {
        if self.breeding_area.is_some() {
            return false;
        }
        if let Some(egg) = self.digitama_deck.pop() {
            self.breeding_area = Some(Permanent::new(egg, turn));
            true
        } else {
            false
        }
    }

    /// Move from breeding area to battle area.
    /// Returns true if successful (breeding area occupied and has room).
    pub fn move_from_breeding(&mut self, rules: &Rules) -> bool {
        if self.battle_area.len() >= rules.field_slots as usize {
            return false;
        }
        if let Some(perm) = self.breeding_area.take() {
            self.battle_area.push(perm);
            true
        } else {
            false
        }
    }

    /// Play a card from hand to the field (creates a new Permanent).
    /// Returns the index in battle_area, or None if hand index is invalid or field is full.
    pub fn play_from_hand(
        &mut self,
        hand_index: usize,
        turn: u16,
        rules: &Rules,
    ) -> Option<usize> {
        if hand_index >= self.hand.len() {
            return None;
        }
        if self.battle_area.len() >= rules.field_slots as usize {
            return None;
        }
        let card = self.hand.remove(hand_index);
        let perm = Permanent::new(card, turn);
        self.battle_area.push(perm);
        Some(self.battle_area.len() - 1)
    }

    /// Remove a permanent from the battle area and send all its cards to trash.
    pub fn delete_permanent(&mut self, field_index: usize) {
        if field_index >= self.battle_area.len() {
            return;
        }
        let perm = self.battle_area.remove(field_index);
        for card in perm.card_sources {
            self.trash.push(card);
        }
        for card in perm.linked_cards {
            self.trash.push(card);
        }
    }

    /// Number of cards in hand.
    pub fn hand_size(&self) -> usize {
        self.hand.len()
    }

    /// Number of permanents on the field (battle area).
    pub fn field_count(&self) -> usize {
        self.battle_area.len()
    }

    /// Number of security cards.
    pub fn security_count(&self) -> usize {
        self.security.len()
    }

    /// Total DP across all Digimon on the field (for reward shaping).
    pub fn total_field_dp(&self, data: &[CardData]) -> i32 {
        self.battle_area
            .iter()
            .filter_map(|p| p.base_dp(data))
            .sum()
    }

    /// Reset per-turn state for all permanents.
    pub fn new_turn(&mut self) {
        for perm in &mut self.battle_area {
            perm.new_turn();
        }
    }

    /// Unsuspend all permanents (start of turn).
    pub fn unsuspend_all(&mut self) {
        for perm in &mut self.battle_area {
            perm.is_suspended = false;
        }
        if let Some(ref mut perm) = self.breeding_area {
            perm.is_suspended = false;
        }
    }
}
