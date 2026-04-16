use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::enums::{CardKind, PlayerId};

/// Lightweight handle to a Permanent on a player's field. Copy-able, used in closures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PermanentHandle {
    pub player: PlayerId,
    pub index: u8,
}

/// A card (or digivolution stack) on the field.
#[derive(Debug, Clone)]
pub struct Permanent {
    /// Digivolution stack: [base, evo1, evo2, ...]. Last is top card.
    pub card_sources: Vec<CardSource>,
    /// Option cards linked sideways.
    pub linked_cards: Vec<CardSource>,
    /// Whether this permanent is suspended (tapped).
    pub is_suspended: bool,
    /// Turn number when this was played to the field.
    pub turn_played: u16,
    /// Turn number of last digivolution.
    pub turn_digivolved: u16,
    /// Number of attacks made this turn.
    pub attacks_this_turn: u8,
}

impl Permanent {
    /// Create a new permanent with a single card (played from hand or hatched).
    pub fn new(card: CardSource, turn: u16) -> Self {
        Self {
            card_sources: vec![card],
            linked_cards: Vec::new(),
            is_suspended: false,
            turn_played: turn,
            turn_digivolved: 0,
            attacks_this_turn: 0,
        }
    }

    /// The top card of the digivolution stack.
    pub fn top_card(&self) -> &CardSource {
        self.card_sources.last().expect("Permanent must have at least one card")
    }

    /// Mutable reference to top card.
    pub fn top_card_mut(&mut self) -> &mut CardSource {
        self.card_sources.last_mut().expect("Permanent must have at least one card")
    }

    /// Owner of this permanent (from top card).
    pub fn owner(&self) -> PlayerId {
        self.top_card().owner
    }

    /// Level of the top card.
    pub fn level(&self, data: &[CardData]) -> Option<u8> {
        self.top_card().level(data)
    }

    /// Base DP from the top card (before modifiers).
    pub fn base_dp(&self, data: &[CardData]) -> Option<i32> {
        self.top_card().dp(data)
    }

    /// Whether the top card is a Digimon.
    pub fn is_digimon(&self, data: &[CardData]) -> bool {
        self.top_card().card_kind(data) == CardKind::Digimon
    }

    /// Whether the top card is a Tamer.
    pub fn is_tamer(&self, data: &[CardData]) -> bool {
        self.top_card().card_kind(data) == CardKind::Tamer
    }

    /// Whether the top card is a DigiEgg (in breeding area).
    pub fn is_digi_egg(&self, data: &[CardData]) -> bool {
        self.top_card().card_kind(data) == CardKind::DigiEgg
    }

    /// All cards in the digivolution stack (alias).
    pub fn digivolution_cards(&self) -> &[CardSource] {
        &self.card_sources
    }

    /// Number of cards in the stack (including top).
    pub fn stack_size(&self) -> usize {
        self.card_sources.len()
    }

    /// Check if any card in the stack has a name containing the given substring.
    pub fn contains_card_name(&self, name: &str, data: &[CardData]) -> bool {
        // Check top card first (most common case)
        if self.top_card().contains_card_name(name, data) {
            return true;
        }
        // Check digivolution sources
        for card in &self.card_sources {
            if card.contains_card_name(name, data) {
                return true;
            }
        }
        false
    }

    /// Check if any card in the stack has a given trait.
    pub fn has_trait(&self, trait_name: &str, data: &[CardData]) -> bool {
        let trait_lower = trait_name.to_lowercase();
        for card in &self.card_sources {
            for t in card.traits(data) {
                if t.to_lowercase() == trait_lower {
                    return true;
                }
            }
        }
        false
    }

    /// Digivolve: push a new card on top of the stack.
    pub fn digivolve(&mut self, card: CardSource, turn: u16) {
        self.card_sources.push(card);
        self.turn_digivolved = turn;
    }

    /// Reset per-turn state.
    pub fn new_turn(&mut self) {
        self.attacks_this_turn = 0;
    }
}
