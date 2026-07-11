use std::collections::HashSet;

use rand::seq::SliceRandom;
use rand::Rng;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::enums::PlayerId;
use crate::opaque_deck::OpaqueDeckState;
use crate::permanent::Permanent;
use crate::rules::Rules;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeckCardCount {
    pub card_id: String,
    pub count: u16,
    pub is_digitama: bool,
}

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
    pub original_deck: Vec<OriginalDeckCardCount>,

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

    /// `Some` when this player is in **opaque-deck mode** — the engine
    /// does not know this player's deck order, only its composition.
    /// Draws and security pops consult `Game::reveal_source` instead of
    /// reading from `self.deck`. The `deck` field remains empty in
    /// opaque mode (kept as `Vec<CardSource>` only to avoid widening
    /// the type; `deck.is_empty()` does NOT imply deck-out when
    /// `opaque_deck_state.is_some()` — callers must check
    /// `opaque_deck_state` first).
    ///
    /// See `crate::opaque_deck` for the contract. Set only by
    /// `Game::new_with_opaque_opponent`.
    pub opaque_deck_state: Option<OpaqueDeckState>,
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
            original_deck: Vec::new(),
            face_up_security: HashSet::new(),
            last_security_reveal: None,
            commander_zone: None,
            commander_tax: 0,
            is_eliminated: false,
            opaque_deck_state: None,
        }
    }

    /// True when this player's deck composition is known but its order is
    /// not — draws must consult `Game::reveal_source` rather than `self.deck`.
    pub fn is_opaque_deck(&self) -> bool {
        self.opaque_deck_state.is_some()
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

    pub fn contains_card(&self, handle: crate::card_source::CardHandle) -> bool {
        self.hand.iter().any(|card| card.handle() == handle)
            || self.deck.iter().any(|card| card.handle() == handle)
            || self.trash.iter().any(|card| card.handle() == handle)
            || self.security.iter().any(|card| card.handle() == handle)
            || self
                .digitama_deck
                .iter()
                .any(|card| card.handle() == handle)
            || self
                .breeding_area
                .as_ref()
                .is_some_and(|perm| permanent_contains_card(perm, handle))
            || self
                .battle_area
                .iter()
                .any(|perm| permanent_contains_card(perm, handle))
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
    pub fn play_from_hand(&mut self, hand_index: usize, turn: u16, rules: &Rules) -> Option<usize> {
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
    ///
    /// Tolerates an empty `card_sources`: a permanent whose stack was fully
    /// drained mid-deletion (e.g. printed `<Save>` lifted the top card
    /// under a Tamer before the deferred finalizer ran) has nothing left
    /// to trash; the slot is simply removed.
    pub fn delete_permanent(&mut self, field_index: usize) {
        if field_index >= self.battle_area.len() {
            return;
        }
        let perm = self.battle_area.remove(field_index);
        // Empty-stack guard: nothing to inspect (no top card to check
        // for token semantic), nothing to trash. Just drop the slot.
        if perm.card_sources.is_empty() {
            // Linked cards still need to flow to trash so they don't
            // leak — but in practice a permanent with no top will have
            // had its linked cards already drained by the surrounding
            // deletion-finalizer. Defensive parity with the non-empty
            // branch below.
            for card in perm.linked_cards {
                self.trash.push(card);
            }
            return;
        }
        // Token semantic: remove from game. Drop the whole stack on the
        // floor — no trash entry, no zone ever again. Parity with
        // Python's `player.py::delete_permanent` is_token branch.
        if perm.top_card().is_token {
            return;
        }
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

    /// Reset only the [Once Per Turn] activation counters. "Per turn" means
    /// per GAME turn, so an [All Turns][Once Per Turn] effect re-arms at the
    /// start of EVERY turn — including the opponent's — while attack
    /// counters remain owner-turn-scoped (cleared via `new_turn`).
    pub fn reset_effect_activations(&mut self) {
        for perm in &mut self.battle_area {
            perm.effect_activations.clear();
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

fn permanent_contains_card(
    permanent: &crate::permanent::Permanent,
    handle: crate::card_source::CardHandle,
) -> bool {
    permanent
        .card_sources
        .iter()
        .any(|card| card.handle() == handle)
        || permanent
            .linked_cards
            .iter()
            .any(|card| card.handle() == handle)
}
