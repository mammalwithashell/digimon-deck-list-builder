use std::collections::HashMap;

use crate::card_data::CardData;
use crate::card_source::{CardHandle, CardSource};
use crate::enums::{CardColor, CardKind, DelayTrigger, ModifierType, PlayerId};
use crate::modifiers::{ModifierPayload, ModifierRegistry};

/// Lightweight handle to a Permanent on a player's field. Copy-able, used in closures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PermanentHandle {
    pub player: PlayerId,
    pub index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainingBinding {
    pub handle: PermanentHandle,
    pub top_card: CardHandle,
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
        placed_on_turn: u16,
    },
    Linked {
        host: PermanentHandle,
    },
    OrdinaryFieldOption,
    OrphanedPlugIn {
        last_carrier_owner: PlayerId,
    },
    Training {
        owner: PlayerId,
        trained: Option<TrainingBinding>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthIdentity {
    pub card_name: String,
    pub card_names: Vec<String>,
    pub kind: CardKind,
    pub level: Option<u8>,
    pub colors: Vec<CardColor>,
    pub traits: Vec<String>,
    pub dp: Option<i32>,
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

    /// Decrement the activation count for a specific effect on this permanent.
    /// DCGO `ActivateClass.RemoveUse()` — refunds a once-per-turn use when the
    /// optional body executed nothing (`refund_opt` step,
    /// G-OPT-REFUND-ON-DECLINE). Saturates at zero.
    pub fn unrecord_activation(&mut self, card: CardHandle, slot: u8) {
        if let Some(entry) = self.effect_activations.get_mut(&(card, slot)) {
            *entry = entry.saturating_sub(1);
        }
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

    pub fn synth_identity(
        &self,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> SynthIdentity {
        let top = self.top_card();
        let mut identity = SynthIdentity {
            card_name: top.card_name(data).to_string(),
            card_names: top
                .card_names(data)
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            kind: top.card_kind(data),
            level: top.level(data),
            colors: top.colors(data).to_vec(),
            traits: top.traits(data).to_vec(),
            dp: top.dp(data),
        };

        for entry in modifiers.get(handle, ModifierType::TreatAsDigimon) {
            if let ModifierPayload::SynthIdentity {
                kind,
                level,
                colors,
                traits,
                dp,
            } = &entry.payload
            {
                identity.kind = *kind;
                identity.level = Some(*level);
                identity.colors = colors.clone();
                identity.traits = traits.clone();
                identity.dp = Some(*dp);
            }
        }
        for entry in modifiers.get(handle, ModifierType::ChangePermanentLevel) {
            match &entry.payload {
                ModifierPayload::LevelOverride { value, delta } => {
                    identity.level = if *delta {
                        identity
                            .level
                            .map(|level| (level as i32 + *value).clamp(0, u8::MAX as i32) as u8)
                    } else {
                        Some((*value).clamp(0, u8::MAX as i32) as u8)
                    };
                }
                ModifierPayload::None => {
                    identity.level = Some(entry.value.clamp(0, u8::MAX as i32) as u8);
                }
                _ => {}
            }
        }
        for entry in modifiers.get(handle, ModifierType::ChangeBaseCardName) {
            match &entry.payload {
                ModifierPayload::Name { value, base: _ } => {
                    identity.card_name = value.clone();
                    identity.card_names.clear();
                    identity.card_names.push(value.clone());
                }
                ModifierPayload::None if entry.value != 0 => {
                    identity.card_name = entry.value.to_string();
                    identity.card_names.clear();
                    identity.card_names.push(identity.card_name.clone());
                }
                _ => {}
            }
        }
        for entry in modifiers.get(handle, ModifierType::SourceNameAliases) {
            if let ModifierPayload::SourceNameAliases { level_lte } = &entry.payload {
                for card in self
                    .card_sources
                    .iter()
                    .take(self.card_sources.len().saturating_sub(1))
                {
                    if let Some(cap) = *level_lte {
                        if !matches!(card.level(data), Some(level) if level <= cap) {
                            continue;
                        }
                    }
                    if level_lte.is_none() && card.level(data).is_none() {
                        continue;
                    }
                    for name in card.card_names(data) {
                        if !identity
                            .card_names
                            .iter()
                            .any(|existing| existing.eq_ignore_ascii_case(name))
                        {
                            identity.card_names.push(name.to_string());
                        }
                    }
                }
            }
        }
        for entry in modifiers.get(handle, ModifierType::ChangeBaseCardColor) {
            if let ModifierPayload::Colors { value, base: _ } = &entry.payload {
                identity.colors = value.clone();
            }
        }
        for entry in modifiers.get(handle, ModifierType::ChangeTraits) {
            if let ModifierPayload::Traits { add, replace } = &entry.payload {
                if *replace {
                    identity.traits.clear();
                }
                for trait_name in add {
                    if !identity
                        .traits
                        .iter()
                        .any(|existing| existing.eq_ignore_ascii_case(trait_name))
                    {
                        identity.traits.push(trait_name.clone());
                    }
                }
            }
        }
        for modifier in [ModifierType::ChangeCardDP, ModifierType::ChangeOriginDP] {
            for entry in modifiers.get(handle, modifier) {
                match &entry.payload {
                    ModifierPayload::Dp { value, .. } => identity.dp = Some(*value),
                    ModifierPayload::None => identity.dp = Some(entry.value),
                    _ => {}
                }
            }
        }
        identity
    }

    pub fn level_for_rules(
        &self,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> Option<u8> {
        self.synth_identity(data, modifiers, handle).level
    }

    /// Base DP from the top card (before modifiers).
    pub fn base_dp(&self, data: &[CardData]) -> Option<i32> {
        self.top_card().dp(data)
    }

    pub fn base_dp_for_rules(
        &self,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> Option<i32> {
        self.synth_identity(data, modifiers, handle).dp
    }

    /// Whether the top card is a Digimon.
    pub fn is_digimon(&self, data: &[CardData]) -> bool {
        matches!(
            self.top_card().card_kind(data),
            CardKind::Digimon | CardKind::Dual
        )
    }

    pub fn is_digimon_for_rules(
        &self,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> bool {
        matches!(
            self.synth_identity(data, modifiers, handle).kind,
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

    pub fn colors_for_rules(
        &self,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> Vec<CardColor> {
        self.synth_identity(data, modifiers, handle).colors
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

    pub fn contains_card_name_for_rules(
        &self,
        name: &str,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> bool {
        let needle = name.to_lowercase();
        let identity = self.synth_identity(data, modifiers, handle);
        if identity
            .card_names
            .iter()
            .any(|card_name| card_name.to_lowercase().contains(&needle))
        {
            return true;
        }
        self.card_sources
            .iter()
            .any(|card| card.contains_card_name(name, data))
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

    pub fn has_trait_for_rules(
        &self,
        trait_name: &str,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> bool {
        let needle = trait_name.to_lowercase();
        let identity = self.synth_identity(data, modifiers, handle);
        identity
            .traits
            .iter()
            .any(|trait_name| trait_name.to_lowercase() == needle)
            || self.has_trait(trait_name, data)
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
