use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind, PlayerId};

/// Lightweight handle to a CardSource in the game. Copy-able, used in closures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardHandle(pub u16);

/// A card instance in the game — one per physical card in a deck/hand/field/trash.
/// Links to static CardData for metadata.
#[derive(Debug, Clone)]
pub struct CardSource {
    /// Index into the shared CardData store.
    pub data_index: usize,
    /// Owner player.
    pub owner: PlayerId,
    /// Unique instance index within this game.
    pub card_index: u16,
    /// Whether this is a token (not from a deck).
    pub is_token: bool,
    /// "Also treated as" names granted by effects.
    pub also_treated_as: Vec<String>,
}

impl CardSource {
    /// Create a new card source from card data.
    pub fn new(data_index: usize, owner: PlayerId, card_index: u16) -> Self {
        Self {
            data_index,
            owner,
            card_index,
            is_token: false,
            also_treated_as: Vec::new(),
        }
    }

    /// Create a token card source.
    pub fn new_token(data_index: usize, owner: PlayerId, card_index: u16) -> Self {
        Self {
            data_index,
            owner,
            card_index,
            is_token: true,
            also_treated_as: Vec::new(),
        }
    }

    /// Get this card's handle.
    pub fn handle(&self) -> CardHandle {
        CardHandle(self.card_index)
    }

    // --- Accessors that require CardData lookup ---

    /// Get card_id from the data store.
    pub fn card_id<'a>(&self, data: &'a [CardData]) -> &'a str {
        &data[self.data_index].card_id
    }

    /// Get card name from the data store.
    pub fn card_name<'a>(&self, data: &'a [CardData]) -> &'a str {
        &data[self.data_index].card_name
    }

    /// Get all names this card is treated as.
    pub fn card_names<'a>(&'a self, data: &'a [CardData]) -> Vec<&'a str> {
        let mut names = vec![data[self.data_index].card_name.as_str()];
        for name in &self.also_treated_as {
            names.push(name.as_str());
        }
        names
    }

    pub fn card_kind(&self, data: &[CardData]) -> CardKind {
        data[self.data_index].card_kind
    }

    pub fn colors<'a>(&self, data: &'a [CardData]) -> &'a [CardColor] {
        &data[self.data_index].colors
    }

    pub fn level(&self, data: &[CardData]) -> Option<u8> {
        data[self.data_index].level
    }

    pub fn play_cost(&self, data: &[CardData]) -> u16 {
        data[self.data_index].play_cost
    }

    pub fn dp(&self, data: &[CardData]) -> Option<i32> {
        data[self.data_index].dp
    }

    pub fn is_digimon(&self, data: &[CardData]) -> bool {
        data[self.data_index].card_kind == CardKind::Digimon
    }

    pub fn is_tamer(&self, data: &[CardData]) -> bool {
        data[self.data_index].card_kind == CardKind::Tamer
    }

    pub fn is_option(&self, data: &[CardData]) -> bool {
        data[self.data_index].card_kind == CardKind::Option
    }

    pub fn is_digi_egg(&self, data: &[CardData]) -> bool {
        data[self.data_index].card_kind == CardKind::DigiEgg
    }

    pub fn traits<'a>(&self, data: &'a [CardData]) -> &'a [String] {
        &data[self.data_index].traits
    }

    /// Check if this card's name contains the given substring (case-insensitive).
    pub fn contains_card_name(&self, name: &str, data: &[CardData]) -> bool {
        let name_lower = name.to_lowercase();
        let card_name = &data[self.data_index].card_name;
        if card_name.to_lowercase().contains(&name_lower) {
            return true;
        }
        for also in &self.also_treated_as {
            if also.to_lowercase().contains(&name_lower) {
                return true;
            }
        }
        false
    }
}
