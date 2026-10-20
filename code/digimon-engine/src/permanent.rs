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

#[derive(Clone, PartialEq, Eq)]
pub struct SynthIdentity {
    pub card_name: String,
    pub card_names: Vec<String>,
    pub kind: CardKind,
    /// Original category retained by an additive identity grant.
    pub retained_kind: Option<CardKind>,
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
#[derive(Clone)]
pub struct Permanent {
    /// Monotonic face incarnation within this carrier. A temporary
    /// digivolution must not revive an older pending face effect when undone.
    pub(crate) face_generation: u64,
    /// Opaque optional harness marker; omitted from Debug and replay hashes.
    pub(crate) native_play_incarnation: Option<std::sync::Arc<()>>,
    /// Stable identity for battle/replacement and queued observer events. Stack edits retain it;
    /// a newly played permanent starts without one and DNA clears it.
    pub(crate) battle_identity: Option<u64>,
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
    /// §17-1-3-2-3: this permanent was placed in the breeding area by an
    /// effect (`Game::play_to_breeding_from_hand`), so a non-Digimon top here
    /// is exempt from the rule check's breeding-area trash target. `false`
    /// for a hatched Digi-Egg and for every battle-area permanent: the move
    /// from the breeding area clears it (the battle area's analogue for
    /// Options is `option_state`).
    pub placed_by_effect: bool,
}

impl std::fmt::Debug for Permanent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Permanent")
            .field("battle_identity", &self.battle_identity)
            .field("card_sources", &self.card_sources)
            .field("linked_cards", &self.linked_cards)
            .field("is_suspended", &self.is_suspended)
            .field("turn_played", &self.turn_played)
            .field("turn_digivolved", &self.turn_digivolved)
            .field("attacks_this_turn", &self.attacks_this_turn)
            .field("is_attacking", &self.is_attacking)
            .field("effect_activations", &self.effect_activations)
            .field("option_state", &self.option_state)
            .finish()
    }
}

impl Permanent {
    /// Create a new permanent with a single card (played from hand or hatched).
    ///
    /// The field is a public area; cards are placed face-up in it (§3-4-2).
    /// A card that was a face-down digivolution card (§4-7-9) is a new card
    /// once it moves (§3-1-3-1-2), so the new permanent's card is face-up
    /// whatever zone it came from. (Only a card already ON the field can end
    /// up a face-down top -- a source exposed when the cards above it leave --
    /// and the rule check trashes that one, §17-1-3-2-4.)
    pub fn new(mut card: CardSource, turn: u16) -> Self {
        card.face_down = false;
        Self {
            face_generation: 0,
            battle_identity: None,
            native_play_incarnation: None,
            card_sources: vec![card],
            linked_cards: Vec::new(),
            is_suspended: false,
            turn_played: turn,
            turn_digivolved: 0,
            attacks_this_turn: 0,
            is_attacking: false,
            effect_activations: HashMap::new(),
            option_state: OptionState::Standard,
            placed_by_effect: false,
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

    /// Indices into `card_sources` of the face-down cards UNDER the top card,
    /// lowest first. The one notion behind every "by trashing the bottom
    /// face-down card(s) from under your Tamers" cost: its eligible cards are
    /// these, and "the bottom face-down card" is the first -- the LOWEST
    /// face-down card, whether or not face-up cards sit below it (card text of
    /// BT26-005 / BT25-035; DCGO `BT26_005.cs`
    /// `DigivolutionCards.LastOrDefault(IsFlipped)`; the rules manual has no
    /// narrower definition). The payability check, the Tamer picks and the
    /// trash all read it.
    pub fn face_down_source_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let under = self.card_sources.len().saturating_sub(1);
        self.card_sources[..under]
            .iter()
            .enumerate()
            .filter(|(_, source)| source.face_down)
            .map(|(index, _)| index)
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
            retained_kind: None,
            level: top.level(data),
            colors: top.digimon_colors(data).to_vec(),
            traits: top.traits(data).to_vec(),
            dp: top.dp(data),
        };

        for entry in modifiers.get(handle, ModifierType::TreatAsDigimon) {
            let (kind, level, colors, traits, dp, additive) = match &entry.payload {
                ModifierPayload::SynthIdentity { kind, level, colors, traits, dp } => (kind, level, colors, traits, dp, false),
                ModifierPayload::AdditiveSynthIdentity { kind, level, colors, traits, dp } => (kind, level, colors, traits, dp, true),
                _ => continue,
            };
            identity.retained_kind = if additive { Some(top.card_kind(data)).filter(|original| original != kind) } else { None };
            identity.kind = *kind;
            // An additive DP-only grant does not erase printed properties.
            // Level zero is the DSL's existing omitted-level sentinel; a newer
            // treatment resets that field to its printed value (Q6019).
            identity.level = if additive && *level == 0 { top.level(data) } else { Some(*level) };
            if !additive || !colors.is_empty() { identity.colors = colors.clone(); }
            if !additive || !traits.is_empty() { identity.traits = traits.clone(); }
            identity.dp = Some(*dp);
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
        // Additive color treatment (G-ENGINE-ADDITIVE-COLOR-TREATMENT). Runs
        // AFTER the replace-style `ChangeBaseCardColor` pass and append-dedups:
        //   * `Colors { value }` — the listed colors are gained.
        //   * `None` — "treated as also having the colors of its digivolution
        //     cards": every NON-FLIPPED source beneath the top card contributes
        //     its colors, read live at synth time so a source placed/trashed
        //     mid-effect is reflected at once. DCGO `ChangeCardColorClass`
        //     (BT8_084.cs: `if (cardSource1.IsFlipped) continue;`); official
        //     BT8-084 Q&A: white + red + green sources = a 3-color card.
        for entry in modifiers.get(handle, ModifierType::AddColor) {
            let gained: Vec<CardColor> = match &entry.payload {
                ModifierPayload::Colors { value, .. } => value.clone(),
                ModifierPayload::None => self
                    .card_sources
                    .iter()
                    .rev()
                    .skip(1)
                    .filter(|src| !src.face_down)
                    .flat_map(|src| src.colors(data).iter().copied())
                    .collect(),
                _ => Vec::new(),
            };
            for color in gained {
                if !identity.colors.contains(&color) {
                    identity.colors.push(color);
                }
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
        if modifiers.has(handle, ModifierType::DontHaveDp) {
            identity.dp = None;
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

    /// Whether this permanent counts as a Digimon for RULES purposes.
    ///
    /// `CardKind::Token` is included, and that is not a convenience: every token
    /// this engine can materialize IS a printed Digimon. `TokenDef` carries no
    /// kind field at all, and `TokenDef::to_card_data` hardcodes
    /// `card_kind: CardKind::Token` for all of them, so `Token` here means "a
    /// Digimon whose card has no printed number", not "some other card type".
    /// The Petrification Token prints as `(Digimon/white/3000 DP/...)`.
    ///
    /// Leaving it out made every rule phrased "your opponent's Digimon" skip
    /// tokens. Measured case: `<Raid>` (general_rule.pdf 16-23-1 switches the
    /// attack to the opponent's unsuspended DIGIMON with the highest DP) never
    /// offered a Petrification Token, because `raid_switch_candidates` gates on
    /// this predicate -- every other filter passed (unsuspended, Standard option
    /// state, 3000 DP). DCGO applies no token filter there either
    /// (`CardEffectCommons/KeyWordEffects/Raid.cs`: `!permanent1.IsSuspended`
    /// plus `IsMaxDP`), so the rules manual and DCGO agreed against us.
    ///
    /// The engine already treated tokens as Digimon everywhere this predicate is
    /// not consulted -- `EffectContext::play_token` routes
    /// `CannotPlayDigimonByEffect` for them precisely because "a Token is a
    /// Digimon", and Medusamon (BT21-029)'s "when any of your opponent's Digimon
    /// are deleted" fires on a token deletion (oracle-confirmed). This closes
    /// that inconsistency rather than creating a new behaviour.
    ///
    /// `is_digimon` (the PRINTED-kind check, above) is deliberately NOT changed:
    /// it answers "what card type is on top", which for a token is genuinely
    /// `Token`.
    // A Digi-Egg card on the field is a Digimon permanent (EX2-007 Q3274,
    // Q3276, Q3285), while its printed card kind remains DigiEgg.
    pub fn is_digimon_for_rules(
        &self,
        data: &[CardData],
        modifiers: &ModifierRegistry,
        handle: PermanentHandle,
    ) -> bool {
        matches!(
            self.synth_identity(data, modifiers, handle).kind,
            CardKind::Digimon | CardKind::Dual | CardKind::Token | CardKind::DigiEgg
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
            .any(|card_name| crate::card_data::rules_name_contains(card_name, &needle))
        {
            return true;
        }
        self.card_sources
            .iter()
            .take(self.card_sources.len().saturating_sub(1))
            .any(|card| card.contains_card_name(name, data))
    }

    /// Whether this Digimon's printed traits include `trait_name`
    /// (case-insensitive): the TOP card's trait line, including its
    /// `(Rule) Trait: Has [X]` grants (§2-3-4-2, folded into `CardData::traits`).
    /// A digivolution card's traits are not the Digimon's: the bottom cards of
    /// a stack contribute their inherited effects (§4-7-4, §4-7-7 "treated as
    /// part of the information on the top card"), not their characteristics.
    /// Official Q&A Q3225 (EX1-038): trashing the top card turns the Digimon
    /// into one "without [Free] in its traits" while the [Free] EX1-038 is
    /// still one of its digivolution cards. DCGO reads `permanent.TopCard`
    /// (`BT25_020.cs` `CanTriggerWhenWinBattle`, `EX8_047.cs`).
    /// Trait-changing effects are layered on by [`Self::has_trait_for_rules`],
    /// the reader every rules check must use; this one is crate-private and
    /// named "printed" so no caller skips them by accident.
    /// G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS.
    pub(crate) fn has_printed_trait(&self, trait_name: &str, data: &[CardData]) -> bool {
        self.top_card()
            .traits(data)
            .iter()
            .any(|t| t.eq_ignore_ascii_case(trait_name))
    }

    /// Whether this Digimon has `trait_name` for the rules: its synthesized
    /// identity's traits (the top card plus trait-changing effects:
    /// `ChangeTraits`, `TreatAsDigimon`) or its top card's printed traits --
    /// the same reading as the DSL `trait_has` permanent predicate. Never a
    /// digivolution card's traits (see [`Self::has_printed_trait`]).
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
            || self.has_printed_trait(trait_name, data)
    }

    /// Digivolve: push a new card on top of the stack.
    pub fn digivolve(&mut self, card: CardSource, turn: u16) {
        self.push_top(card);
        self.turn_digivolved = turn;
    }

    /// Stack `card` on top as this permanent's new TOP card, face-up
    /// (§3-4-2), whatever zone it came from -- a hand or trash card, or a
    /// digivolution card digivolved into from under a permanent. Every path
    /// that installs a new top card on an existing stack (digivolution, DNA
    /// digivolution, Blast DNA, Assembly-style merges) goes through here.
    /// Digivolution cards placed UNDER the top keep the face they are placed
    /// with (§4-7-5, §4-7-9): use `push_under` / `push_as_top_source`.
    /// A new top card is a new face incarnation (`face_generation`), so a
    /// captured Security-check observer of the previous face cannot revive.
    pub fn push_top(&mut self, mut card: CardSource) {
        self.face_generation = self.face_generation.saturating_add(1);
        card.face_down = false;
        self.card_sources.push(card);
    }

    /// Insert `card` at the bottom of the digivolution stack (position 0).
    /// The current top card remains on top. Matches DCGO's "place X as the
    /// bottom digivolution source" semantics.
    pub fn push_under(&mut self, card: crate::card_source::CardSource) {
        self.card_sources.insert(0, card);
    }

    /// Insert `card` as the TOP digivolution source — directly beneath the
    /// active top card. The top card (and thus the permanent's identity)
    /// remains unchanged; a placed digivolution card never changes what the
    /// permanent IS. Matches DCGO `Permanent.AddDigivolutionCardsTop`
    /// (`cardSources.Insert(1, …)` in DCGO's top-first ordering; this
    /// engine's `card_sources` is bottom-first with the top card LAST, so
    /// the top-source position is `len - 1`). On a stack with only the top
    /// card, the placed card becomes the sole source (simultaneously top
    /// and bottom source). G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05).
    pub fn push_as_top_source(&mut self, card: crate::card_source::CardSource) {
        let idx = self.card_sources.len().saturating_sub(1);
        self.card_sources.insert(idx, card);
    }

    /// Reset per-turn state.
    pub fn new_turn(&mut self) {
        self.attacks_this_turn = 0;
        self.effect_activations.clear();
    }
}

impl std::fmt::Debug for SynthIdentity {
 fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
 let mut d=f.debug_struct("SynthIdentity");
 d.field("card_name", &self.card_name);
 d.field("card_names", &self.card_names);
 d.field("kind", &self.kind);
 d.field("level", &self.level);
 d.field("colors", &self.colors);
 d.field("traits", &self.traits);
 d.field("dp", &self.dp);
 if self.retained_kind.is_some() { d.field("retained_kind", &self.retained_kind); }
 d.finish()
 }
}
