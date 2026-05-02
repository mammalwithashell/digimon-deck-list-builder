use std::collections::HashMap;

use crate::card_data::CardData;
use crate::card_source::{CardHandle, CardSource};
use crate::enums::{CardKind, DelayTrigger, PlayerId};

/// Lightweight handle to a Permanent on a player's field. Copy-able, used in closures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PermanentHandle {
    pub player: PlayerId,
    pub index: u8,
}

/// Additional state a Permanent carries when its top card is an Option.
/// For Digimon/Tamer/DigiEgg permanents this is always `Standard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionState {
    Standard,
    /// Delay Option placed on the field, awaiting its scheduled trigger.
    /// `trash_on_turn` is the **absolute turn number** (matching
    /// `Game.turn_count`) at which this Option self-trashes and fires its
    /// `DelayEffect`. The value is computed at delay-installation time from
    /// the `DelayTrigger` + the current turn (Task 3 installs; Task 3 drives
    /// the end-of-turn scan).
    Delayed {
        owner: PlayerId,
        trash_on_turn: u16,
        trigger: DelayTrigger,
    },
    Linked {
        host: PermanentHandle,
    },
    Training {
        owner: PlayerId,
    },
}

impl Default for OptionState {
    fn default() -> Self {
        OptionState::Standard
    }
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
    /// Whether this permanent is currently the attacker in an in-flight
    /// attack. Set by `begin_attack`, cleared by `cleanup_attack`. Used by
    /// condition closures for effects like Progress (immunity while
    /// attacking) and by UI affordances for the attack animation.
    /// Closes RUST_PYTHON_PARITY.md §2.2.
    pub is_attacking: bool,
    /// Per-source, per-effect activation counts this turn.
    /// Key: (source card handle, effect slot index within that card's
    /// `CardEffect::effects(handle)` vec). Value: number of activations.
    /// Reset in `new_turn`. Used to compute OPT (once-per-turn) state for
    /// the observation tensor and to gate future effect firing.
    pub effect_activations: HashMap<(CardHandle, u8), u8>,
    /// Phase 8: additional state when the top card is an Option.
    /// For Digimon/Tamer/DigiEgg permanents this is always `Standard`.
    pub option_state: OptionState,
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
            is_attacking: false,
            effect_activations: HashMap::new(),
            option_state: OptionState::Standard,
        }
    }

    /// Increment the activation count for a specific effect on this permanent.
    pub fn record_activation(&mut self, card: CardHandle, slot: u8) {
        let entry = self.effect_activations.entry((card, slot)).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// How many times a given effect has fired this turn.
    pub fn activation_count(&self, card: CardHandle, slot: u8) -> u8 {
        self.effect_activations
            .get(&(card, slot))
            .copied()
            .unwrap_or(0)
    }

    /// The top card of the digivolution stack.
    pub fn top_card(&self) -> &CardSource {
        self.card_sources
            .last()
            .expect("Permanent must have at least one card")
    }

    /// Mutable reference to top card.
    pub fn top_card_mut(&mut self) -> &mut CardSource {
        self.card_sources
            .last_mut()
            .expect("Permanent must have at least one card")
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
        matches!(
            self.top_card().card_kind(data),
            CardKind::Digimon | CardKind::Dual
        )
    }

    /// Whether the top card is an Option permanent.
    pub fn is_option(&self, data: &[CardData]) -> bool {
        self.top_card().card_kind(data) == CardKind::Option
    }

    /// Whether the top card is a Tamer.
    pub fn is_tamer(&self, data: &[CardData]) -> bool {
        self.top_card().card_kind(data) == CardKind::Tamer
    }

    /// Returns `true` if this permanent's digivolution stack contains at
    /// least one Tamer source that is NOT face-down. Used by the `<Mind Link>`
    /// candidate filter — DCGO `MindLink.cs:25`:
    /// `cardSource.IsTamer && !cardSource.IsFlipped`.
    ///
    /// The top card itself is included in the scan; the top of a Tamer
    /// permanent is by definition a non-face-down Tamer source, so a Tamer
    /// permanent is correctly excluded as a target by this helper (a Tamer
    /// is its own controller's Tamer; MindLink should not target Tamers).
    pub fn has_non_facedown_tamer_source(&self, data: &[CardData]) -> bool {
        self.card_sources
            .iter()
            .any(|src| src.is_tamer(data) && !src.face_down)
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

    /// Insert `card` at the bottom of the digivolution stack (position 0).
    /// The current top card remains on top. Matches DCGO's "place X as the
    /// bottom digivolution source" semantics.
    pub fn push_under(&mut self, card: crate::card_source::CardSource) {
        self.card_sources.insert(0, card);
    }

    /// Reset per-turn state.
    pub fn new_turn(&mut self) {
        self.attacks_this_turn = 0;
        self.effect_activations.clear();
    }
}
