use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind, PlayerId};
use serde::{Deserialize, Serialize};

/// Lightweight handle to a CardSource in the game. Copy-able, used in closures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardHandle(pub u16);

/// Per-instance metadata visible only while a card is in the reveal pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealOverlay {
    pub name: Option<String>,
    pub kind: Option<CardKind>,
}

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
    /// Temporary identity visible only to reveal-zone predicates.
    pub reveal_overlay: Option<RevealOverlay>,
    /// `true` when this source was placed face-down (DCGO `IsFlipped` analog
    /// for digivolution-stack sources). Set only by `<Training>` (Phase F
    /// Task 6); `false` for all other sources. Consulted by `<Mind Link>`'s
    /// "no Tamer source" filter (DCGO `MindLink.cs:25` — face-down Tamer
    /// sources do NOT count). Out of scope for face-up reveal mechanics.
    pub face_down: bool,
    /// `true` for opaque-deck security placeholders — cards laid into an
    /// opaque opponent's security stack at setup time whose identities
    /// are not yet known (they're revealed lazily when flipped via
    /// SecurityCheck, consuming a `RevealKind::Security` from the
    /// reveal source). When this is true, `data_index` is meaningless
    /// (set to 0 as a non-crashing default); `card_index` IS still
    /// unique per instance so face_up_security membership works.
    ///
    /// See `crate::opaque_deck` module docs for the lazy-reveal contract
    /// and `Game::materialize_opaque_security_placeholder` for the
    /// materialization helper.
    pub is_opaque_placeholder: bool,
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
            reveal_overlay: None,
            face_down: false,
            is_opaque_placeholder: false,
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
            reveal_overlay: None,
            face_down: false,
            is_opaque_placeholder: false,
        }
    }

    /// Create an opaque-deck security placeholder — a slot-holder for an
    /// opaque opponent's security card whose identity is not yet known.
    /// `data_index` is set to 0 (a defensive default; reading the card's
    /// data via `card_id()` etc. will return whatever card sits at
    /// index 0 in the data store, which is not meaningful for a
    /// placeholder — callers must check `is_opaque_placeholder` first).
    ///
    /// `card_index` should be unique per instance (use
    /// `Game::next_card_index`) so face_up_security tracking still works.
    pub fn new_opaque_security_placeholder(owner: PlayerId, card_index: u16) -> Self {
        Self {
            data_index: 0,
            owner,
            card_index,
            is_token: false,
            also_treated_as: Vec::new(),
            reveal_overlay: None,
            face_down: true, // security cards are face-down by default
            is_opaque_placeholder: true,
        }
    }

    /// Get this card's handle.
    pub fn handle(&self) -> CardHandle {
        CardHandle(self.card_index)
    }

    pub fn clear_reveal_overlay(&mut self) {
        self.reveal_overlay = None;
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

    /// Get all names this card is treated as — its printed name plus any
    /// static "also treated as" identity aliases (`CardData.also_treated_as`)
    /// and any effect-granted aliases (`CardSource.also_treated_as`).
    pub fn card_names<'a>(&'a self, data: &'a [CardData]) -> Vec<&'a str> {
        let mut names = vec![data[self.data_index].card_name.as_str()];
        for name in &data[self.data_index].also_treated_as {
            names.push(name.as_str());
        }
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

    pub fn is_digimon_card_for_search(&self, data: &[CardData]) -> bool {
        data[self.data_index].is_digimon_card_for_search()
    }

    pub fn is_option_card_for_search(&self, data: &[CardData]) -> bool {
        data[self.data_index].is_option_card_for_search()
    }

    pub fn digimon_level(&self, data: &[CardData]) -> Option<u8> {
        data[self.data_index].digimon_level()
    }

    pub fn digimon_dp(&self, data: &[CardData]) -> Option<i32> {
        data[self.data_index].digimon_dp()
    }

    pub fn digimon_colors<'a>(&self, data: &'a [CardData]) -> &'a [CardColor] {
        data[self.data_index].digimon_colors()
    }

    pub fn option_colors<'a>(&self, data: &'a [CardData]) -> &'a [CardColor] {
        data[self.data_index].option_colors()
    }

    pub fn option_use_cost(&self, data: &[CardData]) -> Option<u16> {
        data[self.data_index].option_use_cost()
    }

    pub fn digivolution_costs<'a>(&self, data: &'a [CardData]) -> &'a [crate::card_data::EvoCost] {
        data[self.data_index].digivolution_costs()
    }

    pub fn text_for_search_all_faces(&self, data: &[CardData]) -> String {
        data[self.data_index].text_for_search_all_faces()
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

    /// Check if this card's name contains the given substring
    /// (case-insensitive). Scans the printed name plus static and
    /// effect-granted "also treated as" identity aliases.
    pub fn contains_card_name(&self, name: &str, data: &[CardData]) -> bool {
        let name_lower = name.to_lowercase();
        let card_name = &data[self.data_index].card_name;
        if card_name.to_lowercase().contains(&name_lower) {
            return true;
        }
        for also in &data[self.data_index].also_treated_as {
            if also.to_lowercase().contains(&name_lower) {
                return true;
            }
        }
        for also in &self.also_treated_as {
            if also.to_lowercase().contains(&name_lower) {
                return true;
            }
        }
        false
    }
}
