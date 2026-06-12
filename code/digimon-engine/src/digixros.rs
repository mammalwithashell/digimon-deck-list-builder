use std::collections::{BTreeMap, BTreeSet};

use crate::card_data::CardData;
use crate::card_source::CardHandle;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::permanent::PermanentHandle;

/// Index offset that distinguishes a "leaving / limbo" DigiXros material handle
/// (`Game::digixros_leaving_limbo`) from a real `battle_area` index. Field slots
/// cap at 14 (`Rules::field_slots`), so any handle whose index is ≥ this base
/// addresses limbo entry `index - LIMBO_INDEX_BASE`. Chosen well above the field
/// cap and below `u8::MAX` to leave headroom. See G-DIGIXROS-REDIRECT-EXTRACTION.
pub(crate) const LIMBO_INDEX_BASE: u8 = 200;

/// True if `index` addresses the leaving/limbo zone rather than `battle_area`.
pub(crate) fn is_limbo_index(index: u8) -> bool {
    index >= LIMBO_INDEX_BASE
}

/// Origin zones that can contribute cards to a DigiXros transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigiXrosMaterialZone {
    Hand,
    BattleArea,
    Trash,
    UnderTamer,
}

/// Runtime origin for a selected DigiXros material card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigiXrosMaterialOrigin {
    Hand {
        player: PlayerId,
        index: usize,
        card: CardHandle,
    },
    BattleArea {
        permanent: PermanentHandle,
        card: CardHandle,
    },
    Trash {
        player: PlayerId,
        index: usize,
        card: CardHandle,
    },
    UnderTamer {
        tamer: PermanentHandle,
        source_index: usize,
        card: CardHandle,
    },
}

impl DigiXrosMaterialOrigin {
    pub fn zone(self) -> DigiXrosMaterialZone {
        match self {
            Self::Hand { .. } => DigiXrosMaterialZone::Hand,
            Self::BattleArea { .. } => DigiXrosMaterialZone::BattleArea,
            Self::Trash { .. } => DigiXrosMaterialZone::Trash,
            Self::UnderTamer { .. } => DigiXrosMaterialZone::UnderTamer,
        }
    }

    pub fn card(self) -> CardHandle {
        match self {
            Self::Hand { card, .. }
            | Self::BattleArea { card, .. }
            | Self::Trash { card, .. }
            | Self::UnderTamer { card, .. } => card,
        }
    }
}

/// Distinctness rule for a DigiXros recipe slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigiXrosDistinctBy {
    CardNumber,
    Level,
    Name,
}

/// A declarative recipe slot in a pending DigiXros transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigiXrosRecipeSlot {
    pub slot_index: usize,
    pub names: Vec<String>,
    pub traits: Vec<String>,
    pub min: u8,
    pub max: Option<u8>,
    pub distinct_by: Option<DigiXrosDistinctBy>,
    pub allowed_zones: BTreeSet<DigiXrosMaterialZone>,
    pub cost_delta_per_material: i16,
}

impl DigiXrosRecipeSlot {
    pub fn new(slot_index: usize) -> Self {
        Self {
            slot_index,
            names: Vec::new(),
            traits: Vec::new(),
            min: 0,
            max: Some(1),
            distinct_by: None,
            allowed_zones: BTreeSet::new(),
            cost_delta_per_material: -1,
        }
    }
}

/// Transaction-scoped permission for an extra material origin zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigiXrosZoneAllowance {
    pub zone: DigiXrosMaterialZone,
    pub max_count: Option<u8>,
}

impl DigiXrosZoneAllowance {
    pub fn unbounded(zone: DigiXrosMaterialZone) -> Self {
        Self {
            zone,
            max_count: None,
        }
    }
}

/// A material selected into, or pre-attached to, a DigiXros transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigiXrosSelectedMaterial {
    pub origin: DigiXrosMaterialOrigin,
    pub recipe_slot: Option<usize>,
    pub cost_delta: i16,
}

impl DigiXrosSelectedMaterial {
    pub fn new(
        origin: DigiXrosMaterialOrigin,
        recipe_slot: Option<usize>,
        cost_delta: i16,
    ) -> Self {
        Self {
            origin,
            recipe_slot,
            cost_delta,
        }
    }
}

/// Transaction-scoped permission for one specific material card to satisfy an
/// otherwise-unfilled DigiXros recipe requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigiXrosWildcardSubstitution {
    pub material_card: CardHandle,
    pub required_zone: Option<DigiXrosMaterialZone>,
    pub remaining_uses: u8,
}

impl DigiXrosWildcardSubstitution {
    pub fn once(material_card: CardHandle) -> Self {
        Self {
            material_card,
            required_zone: None,
            remaining_uses: 1,
        }
    }

    pub fn once_from_zone(material_card: CardHandle, required_zone: DigiXrosMaterialZone) -> Self {
        Self {
            material_card,
            required_zone: Some(required_zone),
            remaining_uses: 1,
        }
    }
}

/// A game-scoped DigiXros wildcard modifier waiting to be copied into later
/// transactions during its printed duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveDigiXrosWildcardSubstitution {
    pub controller: PlayerId,
    pub material_card: CardHandle,
    pub required_zone: Option<DigiXrosMaterialZone>,
    pub expires_at_end_of_turn_for: PlayerId,
}

impl ActiveDigiXrosWildcardSubstitution {
    pub fn for_current_turn(
        controller: PlayerId,
        material_card: CardHandle,
        required_zone: Option<DigiXrosMaterialZone>,
        turn_player: PlayerId,
    ) -> Self {
        Self {
            controller,
            material_card,
            required_zone,
            expires_at_end_of_turn_for: turn_player,
        }
    }

    pub fn to_transaction_substitution(self) -> DigiXrosWildcardSubstitution {
        DigiXrosWildcardSubstitution {
            material_card: self.material_card,
            required_zone: self.required_zone,
            remaining_uses: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigiXrosMaterialValidationError {
    ZoneNotAllowed,
    ZoneLimitReached,
    AlreadySelected,
    NoMatchingRecipeSlot,
    RecipeSlotFull,
}

/// First-class state for one pending DigiXros play before cost is paid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigiXrosTransaction {
    pub played_card: CardHandle,
    pub controller: PlayerId,
    pub base_cost: u16,
    pub recipe_slots: Vec<DigiXrosRecipeSlot>,
    pub selected_materials: Vec<DigiXrosSelectedMaterial>,
    pub pre_attached_materials: Vec<DigiXrosSelectedMaterial>,
    pub zone_allowances: BTreeMap<DigiXrosMaterialZone, DigiXrosZoneAllowance>,
    pub wildcard_substitutions: Vec<DigiXrosWildcardSubstitution>,
    pub one_shot_cost_delta: i16,
    pub digixros_count: u8,
}

impl DigiXrosTransaction {
    pub fn new(
        played_card: CardHandle,
        controller: PlayerId,
        base_cost: u16,
        recipe_slots: Vec<DigiXrosRecipeSlot>,
    ) -> Self {
        let mut zone_allowances = BTreeMap::new();
        for slot in &recipe_slots {
            for zone in &slot.allowed_zones {
                zone_allowances
                    .entry(*zone)
                    .or_insert_with(|| DigiXrosZoneAllowance::unbounded(*zone));
            }
        }

        Self {
            played_card,
            controller,
            base_cost,
            recipe_slots,
            selected_materials: Vec::new(),
            pre_attached_materials: Vec::new(),
            zone_allowances,
            wildcard_substitutions: Vec::new(),
            one_shot_cost_delta: 0,
            digixros_count: 0,
        }
    }

    pub fn allow_zone(&mut self, allowance: DigiXrosZoneAllowance) {
        self.zone_allowances.insert(allowance.zone, allowance);
    }

    pub fn is_zone_allowed(&self, zone: DigiXrosMaterialZone) -> bool {
        self.zone_allowances.contains_key(&zone)
    }

    pub fn add_selected_material(&mut self, material: DigiXrosSelectedMaterial) {
        self.selected_materials.push(material);
        self.refresh_digixros_count();
    }

    pub fn add_wildcard_substitution(&mut self, substitution: DigiXrosWildcardSubstitution) {
        self.wildcard_substitutions.push(substitution);
    }

    pub fn try_select_material(
        &mut self,
        origin: DigiXrosMaterialOrigin,
        card: &CardData,
    ) -> Result<usize, DigiXrosMaterialValidationError> {
        let resolution = self.resolve_material_origin(origin, card)?;
        let slot = resolution.slot_index;
        let cost_delta = self.recipe_slots[slot].cost_delta_per_material;
        self.add_selected_material(DigiXrosSelectedMaterial::new(
            origin,
            Some(slot),
            cost_delta,
        ));
        self.consume_wildcard_resolution(resolution);
        Ok(slot)
    }

    pub fn validate_material_origin(
        &self,
        origin: DigiXrosMaterialOrigin,
        card: &CardData,
    ) -> Result<usize, DigiXrosMaterialValidationError> {
        self.resolve_material_origin(origin, card)
            .map(|resolution| resolution.slot_index)
    }

    fn resolve_material_origin(
        &self,
        origin: DigiXrosMaterialOrigin,
        card: &CardData,
    ) -> Result<DigiXrosMaterialResolution, DigiXrosMaterialValidationError> {
        if !self.is_zone_allowed(origin.zone()) {
            return Err(DigiXrosMaterialValidationError::ZoneNotAllowed);
        }
        if self.zone_limit_reached(origin.zone()) {
            return Err(DigiXrosMaterialValidationError::ZoneLimitReached);
        }
        if self.material_card_already_selected(origin.card()) {
            return Err(DigiXrosMaterialValidationError::AlreadySelected);
        }

        let mut found_matching_full_slot = false;
        for slot in &self.recipe_slots {
            if !slot_accepts_card(slot, card) {
                continue;
            }
            if self.recipe_slot_is_full(slot.slot_index) {
                found_matching_full_slot = true;
                continue;
            }
            return Ok(DigiXrosMaterialResolution {
                slot_index: slot.slot_index,
                wildcard_index: None,
            });
        }

        if let Some(wildcard_index) = self.available_wildcard_index(origin) {
            if let Some(slot_index) = self.first_unfilled_recipe_slot() {
                return Ok(DigiXrosMaterialResolution {
                    slot_index,
                    wildcard_index: Some(wildcard_index),
                });
            }
            found_matching_full_slot = true;
        }

        if found_matching_full_slot {
            Err(DigiXrosMaterialValidationError::RecipeSlotFull)
        } else {
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        }
    }

    pub fn add_pre_attached_material(&mut self, material: DigiXrosSelectedMaterial) {
        self.pre_attached_materials.push(material);
        self.refresh_digixros_count();
    }

    pub fn try_pre_attach_material(
        &mut self,
        origin: DigiXrosMaterialOrigin,
        card: &CardData,
        cost_delta: i16,
    ) -> Result<usize, DigiXrosMaterialValidationError> {
        let resolution = self.resolve_material_origin(origin, card)?;
        let slot = resolution.slot_index;
        self.add_pre_attached_material(DigiXrosSelectedMaterial::new(
            origin,
            Some(slot),
            cost_delta,
        ));
        self.consume_wildcard_resolution(resolution);
        Ok(slot)
    }

    /// Pre-attach a material that satisfies NO recipe slot (slot-independent).
    /// DCGO parity (`SelectDigiXrosClass.AddDigivolutionCardInfos`): would-play
    /// hooks like Yuu Amano (BT10-093) place cards in the played card's
    /// digivolution cards that are not DigiXros requirement materials at all —
    /// they bypass recipe/zone validation entirely (the granting effect's own
    /// selection filter is the only constraint) but still carry a per-card
    /// cost delta and ride the pre-attached placement order (judge-quiz Q29).
    pub fn pre_attach_extra_material(&mut self, origin: DigiXrosMaterialOrigin, cost_delta: i16) {
        self.add_pre_attached_material(DigiXrosSelectedMaterial::new(origin, None, cost_delta));
    }

    pub fn add_one_shot_cost_delta(&mut self, delta: i16) {
        self.one_shot_cost_delta += delta;
    }

    pub fn apply_optional_modifier<F>(&mut self, accepted: bool, apply: F) -> bool
    where
        F: FnOnce(&mut Self) -> bool,
    {
        if !accepted {
            return false;
        }
        let checkpoint = self.clone();
        if apply(self) {
            true
        } else {
            *self = checkpoint;
            false
        }
    }

    pub fn material_count(&self) -> usize {
        self.selected_materials.len() + self.pre_attached_materials.len()
    }

    /// True if every recipe slot still has at least its `min` materials assigned.
    /// A DigiXros play is only legal while its recipe is satisfied; when a
    /// required material is redirected/consumed mid-resolution (e.g. BT17-095
    /// DNA-extracts a material — judge-quiz Q26) the recipe can drop below `min`,
    /// making the declared play unpayable so the host returns to hand.
    pub fn recipe_is_satisfied(&self) -> bool {
        self.recipe_slots.iter().all(|slot| {
            if slot.min == 0 {
                return true;
            }
            let filled = self
                .selected_materials
                .iter()
                .chain(self.pre_attached_materials.iter())
                .filter(|m| m.recipe_slot == Some(slot.slot_index))
                .count();
            filled >= slot.min as usize
        })
    }

    /// Drop every selected / pre-attached material whose origin fails `keep`,
    /// then refresh `digixros_count`. Used by the DigiXros declare-then-pay
    /// recompute when a battle-area material is redirected/consumed away by a
    /// `WhenWouldLeaveBattleArea` replacement mid-resolution: dropping it both
    /// removes its negative `cost_delta` from `final_cost()` and stops
    /// `commit_digixros_material_sources` from trying to consume a vanished
    /// permanent.
    pub fn retain_materials<F>(&mut self, keep: F)
    where
        F: Fn(DigiXrosMaterialOrigin) -> bool,
    {
        self.selected_materials.retain(|m| keep(m.origin));
        self.pre_attached_materials.retain(|m| keep(m.origin));
        self.refresh_digixros_count();
    }

    pub fn selected_cost_delta(&self) -> i16 {
        self.selected_materials
            .iter()
            .chain(self.pre_attached_materials.iter())
            .map(|material| material.cost_delta)
            .sum::<i16>()
            + self.one_shot_cost_delta
    }

    pub fn final_cost(&self) -> u16 {
        (self.base_cost as i16 + self.selected_cost_delta()).max(0) as u16
    }

    fn refresh_digixros_count(&mut self) {
        self.digixros_count = self.material_count().min(u8::MAX as usize) as u8;
    }

    fn material_card_already_selected(&self, card: CardHandle) -> bool {
        self.selected_materials
            .iter()
            .chain(self.pre_attached_materials.iter())
            .any(|material| material.origin.card() == card)
    }

    fn zone_limit_reached(&self, zone: DigiXrosMaterialZone) -> bool {
        let Some(max) = self
            .zone_allowances
            .get(&zone)
            .and_then(|allowance| allowance.max_count)
        else {
            return false;
        };
        let selected_from_zone = self
            .selected_materials
            .iter()
            .chain(self.pre_attached_materials.iter())
            .filter(|material| material.origin.zone() == zone)
            .count();
        selected_from_zone >= max as usize
    }

    fn recipe_slot_is_full(&self, slot_index: usize) -> bool {
        let Some(slot) = self
            .recipe_slots
            .iter()
            .find(|slot| slot.slot_index == slot_index)
        else {
            return true;
        };
        let Some(max) = slot.max else {
            return false;
        };
        let filled = self
            .selected_materials
            .iter()
            .chain(self.pre_attached_materials.iter())
            .filter(|material| material.recipe_slot == Some(slot_index))
            .count();
        filled >= max as usize
    }

    fn first_unfilled_recipe_slot(&self) -> Option<usize> {
        self.recipe_slots
            .iter()
            .find(|slot| !self.recipe_slot_is_full(slot.slot_index))
            .map(|slot| slot.slot_index)
    }

    fn available_wildcard_index(&self, origin: DigiXrosMaterialOrigin) -> Option<usize> {
        self.wildcard_substitutions.iter().position(|wildcard| {
            wildcard.material_card == origin.card()
                && wildcard.remaining_uses > 0
                && wildcard
                    .required_zone
                    .is_none_or(|required| required == origin.zone())
        })
    }

    fn consume_wildcard_resolution(&mut self, resolution: DigiXrosMaterialResolution) {
        let Some(wildcard_index) = resolution.wildcard_index else {
            return;
        };
        if let Some(wildcard) = self.wildcard_substitutions.get_mut(wildcard_index) {
            wildcard.remaining_uses = wildcard.remaining_uses.saturating_sub(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DigiXrosMaterialResolution {
    slot_index: usize,
    wildcard_index: Option<usize>,
}

pub(crate) fn slot_accepts_card(slot: &DigiXrosRecipeSlot, card: &CardData) -> bool {
    let name_matches = slot.names.is_empty()
        || slot
            .names
            .iter()
            .any(|name| matches_digixros_name_requirement(card, name));
    let trait_matches = slot.traits.is_empty()
        || slot.traits.iter().any(|required| {
            card.traits
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(required))
        });
    name_matches && trait_matches
}

pub fn matches_digixros_name_requirement(card: &CardData, required_name: &str) -> bool {
    name_matches(&card.card_name, required_name)
        || card
            .digixros_aliases
            .iter()
            .any(|alias| name_matches(alias, required_name))
}

pub fn matches_generic_name_requirement(card: &CardData, required_name: &str) -> bool {
    name_matches(&card.card_name, required_name)
}

fn name_matches(candidate: &str, required_name: &str) -> bool {
    candidate
        .to_lowercase()
        .contains(&required_name.to_lowercase())
}

pub fn matches_digixros_name_requirement_for_test(card: &CardData, required_name: &str) -> bool {
    matches_digixros_name_requirement(card, required_name)
}

pub fn matches_generic_name_requirement_for_test(card: &CardData, required_name: &str) -> bool {
    matches_generic_name_requirement(card, required_name)
}

impl Game {
    /// Build the inert transaction context for a hand card's first authored
    /// DigiXros path. Later slices consume this context for material prompts,
    /// transaction modifiers, cost math, and post-payment source attachment.
    pub(crate) fn build_digixros_transaction_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<DigiXrosTransaction> {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            let card = self.player(player).hand.get(hand_index)?;
            let card_id = card.card_id(&self.card_data);
            let path = self.alt_path_registry.get(card_id)?.iter().find(|path| {
                matches!(
                    path.kind,
                    digimon_dsl::compiled::CompiledAltPathKind::DigiXros
                )
            })?;
            let mut transaction = compiled_path_transaction(
                card.handle(),
                player,
                card.play_cost(&self.card_data),
                path,
            );
            self.apply_active_digixros_wildcards(&mut transaction);
            Some(transaction)
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = (player, hand_index);
            None
        }
    }

    #[allow(dead_code)]
    pub(crate) fn pending_digixros_transaction(&self) -> Option<&DigiXrosTransaction> {
        self.pending_digixros_transaction.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn pending_digixros_transaction_mut(&mut self) -> Option<&mut DigiXrosTransaction> {
        self.pending_digixros_transaction.as_mut()
    }

    // ─── Leaving / limbo holding slot (G-DIGIXROS-REDIRECT-EXTRACTION) ─────────

    /// Pop the battle-area permanent whose TOP card is `card` (owned by
    /// `player`) out of `battle_area` and park it in `digixros_leaving_limbo`.
    /// Used when a DigiXros material's `WhenWouldLeaveBattleArea` window parks an
    /// optional reward (e.g. BT17-095's `<Delay>` accept): the material is no
    /// longer a standalone top card, but the parked observer can still extract it
    /// (`rematerialize_digixros_limbo`). Returns the limbo-encoded handle, or
    /// `None` if no such standalone permanent exists.
    pub(crate) fn move_battle_permanent_to_limbo(
        &mut self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        let idx = self
            .player(player)
            .battle_area
            .iter()
            .position(|p| p.top_card().handle() == card)?;
        let original_handle = PermanentHandle {
            player,
            index: idx as u8,
        };
        let perm = self.player_mut(player).battle_area.remove(idx);
        // Removing from `battle_area` shifts every later permanent (same player)
        // down by one, invalidating any captured handle with a higher index — in
        // particular the parked replacement's / pending selection's
        // `source_permanent` (e.g. BT17-095's Delay-Option carrier), which the
        // leave observer re-reads at accept time (`source_is_delayed_option`).
        // Decrement those so they keep resolving after the shift.
        self.fixup_source_handles_after_battle_removal(player, idx as u8);
        let limbo_pos = self.digixros_leaving_limbo.len();
        self.digixros_leaving_limbo
            .push((player, original_handle, perm));
        Some(PermanentHandle {
            player,
            index: LIMBO_INDEX_BASE + limbo_pos as u8,
        })
    }

    /// If `subject` is a permanent that was moved to the leaving/limbo slot
    /// (matched by the original battle handle captured at park time), return the
    /// limbo-encoded handle. A `WhenWouldLeaveBattleArea` replacement's subject is
    /// addressed by index alone; after the leaving material moves to limbo that
    /// index is stale (it now aliases a different, shifted-down permanent), so
    /// re-point it to limbo before the parked process reads it.
    pub(crate) fn remap_digixros_limbo_subject(
        &self,
        subject: crate::replacement::ReplacementSubject,
    ) -> crate::replacement::ReplacementSubject {
        let crate::replacement::ReplacementSubject::Permanent(h) = subject else {
            return subject;
        };
        for (pos, (_, original, _)) in self.digixros_leaving_limbo.iter().enumerate() {
            if *original == h {
                return crate::replacement::ReplacementSubject::Permanent(PermanentHandle {
                    player: h.player,
                    index: LIMBO_INDEX_BASE + pos as u8,
                });
            }
        }
        subject
    }

    /// Decrement captured `source_permanent` handles (parked replacement +
    /// pending selection) that referenced a `battle_area` slot AFTER
    /// `removed_index` for `player`, so they survive a `battle_area.remove`.
    fn fixup_source_handles_after_battle_removal(&mut self, player: PlayerId, removed_index: u8) {
        let shift = |h: &mut PermanentHandle| {
            if h.player == player && h.index > removed_index {
                h.index -= 1;
            }
        };
        if let Some(parked) = self.parked_replacement.as_mut() {
            if let Some(sp) = parked.source_permanent.as_mut() {
                shift(sp);
            }
            if let crate::replacement::ReplacementSubject::Permanent(h) = &mut parked.subject {
                shift(h);
            }
        }
        if let Some(sel) = self.pending_selection.as_mut() {
            if let Some(sp) = sel.source_permanent.as_mut() {
                shift(sp);
            }
        }
    }

    /// If `card` (owned by `player`) is the top card of a limbo permanent, return
    /// its limbo-encoded handle. Lets `find_battle_permanent_containing_card`
    /// resolve a leaving subject that is parked in limbo.
    pub(crate) fn find_limbo_permanent_containing_card(
        &self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.digixros_leaving_limbo
            .iter()
            .position(|(owner, _, perm)| {
                *owner == player
                    && perm
                        .card_sources
                        .iter()
                        .chain(perm.linked_cards.iter())
                        .any(|s| s.handle() == card)
            })
            .map(|pos| PermanentHandle {
                player,
                index: LIMBO_INDEX_BASE + pos as u8,
            })
    }

    /// Move the limbo permanent addressed by `handle` (a `LIMBO_INDEX_BASE`-offset
    /// handle) back into `battle_area`, returning its fresh battle-area handle.
    /// Used when a parked reward (DNA-evo) extracts the leaving material: it must
    /// be a real battle-area permanent for the merge to operate on. Other limbo
    /// entries shift down by one. Returns `None` if `handle` is not a live limbo
    /// slot.
    pub(crate) fn rematerialize_digixros_limbo(
        &mut self,
        handle: PermanentHandle,
    ) -> Option<PermanentHandle> {
        if !is_limbo_index(handle.index) {
            return None;
        }
        let pos = (handle.index - LIMBO_INDEX_BASE) as usize;
        if pos >= self.digixros_leaving_limbo.len() {
            return None;
        }
        let (owner, _original, perm) = self.digixros_leaving_limbo.remove(pos);
        let battle_index = self.player(owner).battle_area.len();
        self.player_mut(owner).battle_area.push(perm);
        Some(PermanentHandle {
            player: owner,
            index: battle_index as u8,
        })
    }

    /// True if `card` is RESERVED by the in-flight DigiXros play: it is either
    /// the host being played (`played_card`) or a hand card already selected as a
    /// material. Reserved cards are committed to the declared play and must not be
    /// offered to intervening effects (e.g. BT17-095's DNA-evo partner pick) as a
    /// free hand card. See G-DIGIXROS-REDIRECT-EXTRACTION (judge-quiz Q26).
    pub(crate) fn card_reserved_by_pending_digixros(&self, card: CardHandle) -> bool {
        let Some(tx) = self.pending_digixros_transaction.as_ref() else {
            return false;
        };
        if tx.played_card == card {
            return true;
        }
        tx.selected_materials
            .iter()
            .chain(tx.pre_attached_materials.iter())
            .any(|m| matches!(m.origin, DigiXrosMaterialOrigin::Hand { card: c, .. } if c == card))
    }

    /// Restore ALL limbo permanents owned by `player` back into `battle_area`
    /// (appended in limbo order). Called at DigiXros finalize so any material the
    /// observer DECLINED to extract is consumed under the host as normal.
    pub(crate) fn restore_digixros_limbo_to_battle_area(&mut self, player: PlayerId) {
        let mut i = 0;
        while i < self.digixros_leaving_limbo.len() {
            if self.digixros_leaving_limbo[i].0 == player {
                let (owner, _original, perm) = self.digixros_leaving_limbo.remove(i);
                self.player_mut(owner).battle_area.push(perm);
            } else {
                i += 1;
            }
        }
    }

    pub(crate) fn register_digixros_wildcard_for_current_turn(
        &mut self,
        controller: PlayerId,
        material_card: CardHandle,
        required_zone: Option<DigiXrosMaterialZone>,
    ) {
        let turn_player = self.turn_player();
        self.active_digixros_wildcards
            .push(ActiveDigiXrosWildcardSubstitution::for_current_turn(
                controller,
                material_card,
                required_zone,
                turn_player,
            ));
    }

    pub(crate) fn expire_digixros_wildcards_at_end_of_turn(&mut self, ending_player: PlayerId) {
        self.active_digixros_wildcards
            .retain(|wildcard| wildcard.expires_at_end_of_turn_for != ending_player);
    }

    fn apply_active_digixros_wildcards(&self, transaction: &mut DigiXrosTransaction) {
        let controller = transaction.controller;
        for wildcard in self
            .active_digixros_wildcards
            .iter()
            .copied()
            .filter(|wildcard| wildcard.controller == controller)
        {
            transaction.add_wildcard_substitution(wildcard.to_transaction_substitution());
        }
    }

    pub(crate) fn digixros_recipe_slots_for_card(
        &self,
        card: CardHandle,
    ) -> Vec<DigiXrosRecipeSlot> {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            let Some(data) = self.card_data_for_handle(card) else {
                return Vec::new();
            };
            let Some(path) = self.alt_path_registry.get(&data.card_id).and_then(|paths| {
                paths.iter().find(|path| {
                    matches!(
                        path.kind,
                        digimon_dsl::compiled::CompiledAltPathKind::DigiXros
                    )
                })
            }) else {
                return Vec::new();
            };
            compiled_path_transaction(card, 0, data.play_cost, path).recipe_slots
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = card;
            Vec::new()
        }
    }

    pub(crate) fn card_matches_digixros_recipe(
        &self,
        carrier: CardHandle,
        candidate: CardHandle,
    ) -> bool {
        let recipe = self.digixros_recipe_slots_for_card(carrier);
        if recipe.is_empty() {
            return true;
        }
        let Some(candidate_data) = self.card_data_for_handle(candidate) else {
            return false;
        };
        recipe
            .iter()
            .any(|slot| slot_accepts_card(slot, candidate_data))
    }
}

#[cfg(feature = "dsl-yaml-loader")]
fn compiled_path_transaction(
    played_card: CardHandle,
    controller: PlayerId,
    printed_cost: u16,
    path: &digimon_dsl::compiled::CompiledAltPath,
) -> DigiXrosTransaction {
    use digimon_dsl::compiled::{
        CompiledCost, CompiledDistinctBy, CompiledFormula, CompiledPerSelector, CompiledRepeat,
        CompiledZone,
    };

    let mut base_cost = printed_cost;
    let mut default_delta = -1;
    if let Some(cost) = &path.cost {
        match cost {
            CompiledCost::Literal(n) => {
                if let Ok(n) = u16::try_from(*n) {
                    base_cost = n;
                }
            }
            CompiledCost::Formula(CompiledFormula::BasePerDelta {
                base,
                per: CompiledPerSelector::MaterialCount,
                delta,
            }) => {
                if let Ok(n) = u16::try_from(*base) {
                    base_cost = n;
                }
                if let Ok(n) = i16::try_from(*delta) {
                    default_delta = n;
                }
            }
            CompiledCost::Formula(_) => {}
        }
    }

    let recipe_slots = path
        .materials
        .iter()
        .enumerate()
        .map(|(slot_index, material)| {
            let (min, max) = match material.repeat {
                Some(CompiledRepeat::Unbounded) => (0, None),
                Some(CompiledRepeat::Range { min, max }) => (min, Some(max)),
                None => (1, Some(1)),
            };
            let mut slot = DigiXrosRecipeSlot {
                slot_index,
                names: Vec::new(),
                traits: Vec::new(),
                min,
                max,
                distinct_by: material.distinct_by.map(|distinct| match distinct {
                    CompiledDistinctBy::CardNumber => DigiXrosDistinctBy::CardNumber,
                    CompiledDistinctBy::Level => DigiXrosDistinctBy::Level,
                    CompiledDistinctBy::Name => DigiXrosDistinctBy::Name,
                }),
                allowed_zones: BTreeSet::new(),
                cost_delta_per_material: material.cost_delta.unwrap_or(default_delta),
            };
            if let Some(name) = material.filter.name_is.as_ref() {
                slot.names.push(name.clone());
            }
            if let Some(name) = material.filter.name_contains.as_ref() {
                slot.names.push(name.clone());
            }
            if let Some(names) = material.filter.name_in.as_ref() {
                slot.names.extend(names.iter().cloned());
            }
            if let Some(trait_name) = material.filter.trait_has.as_ref() {
                slot.traits.push(trait_name.clone());
            }
            collect_nested_recipe_identity(&material.filter, &mut slot);
            for zone in &material.zones {
                match zone {
                    CompiledZone::Hand => {
                        slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
                    }
                    CompiledZone::BattleArea => {
                        slot.allowed_zones.insert(DigiXrosMaterialZone::BattleArea);
                    }
                    CompiledZone::Trash => {
                        slot.allowed_zones.insert(DigiXrosMaterialZone::Trash);
                    }
                    CompiledZone::Material => {
                        slot.allowed_zones.insert(DigiXrosMaterialZone::UnderTamer);
                    }
                    _ => {}
                }
            }
            if slot.allowed_zones.is_empty() {
                slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
                slot.allowed_zones.insert(DigiXrosMaterialZone::BattleArea);
            }
            slot
        })
        .collect();

    DigiXrosTransaction::new(played_card, controller, base_cost, recipe_slots)
}

#[cfg(feature = "dsl-yaml-loader")]
fn collect_nested_recipe_identity(
    pred: &digimon_dsl::compiled::CompiledPredicate,
    slot: &mut DigiXrosRecipeSlot,
) {
    for child in pred.all_of.iter().chain(pred.any_of.iter()) {
        if let Some(name) = child.name_is.as_ref() {
            slot.names.push(name.clone());
        }
        if let Some(name) = child.name_contains.as_ref() {
            slot.names.push(name.clone());
        }
        if let Some(names) = child.name_in.as_ref() {
            slot.names.extend(names.iter().cloned());
        }
        if let Some(trait_name) = child.trait_has.as_ref() {
            slot.traits.push(trait_name.clone());
        }
        collect_nested_recipe_identity(child, slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(card_index: u16) -> CardHandle {
        CardHandle(card_index)
    }

    #[test]
    fn transaction_counts_selected_and_preattached_material_costs() {
        let mut slot = DigiXrosRecipeSlot::new(0);
        slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
        slot.cost_delta_per_material = -2;

        let mut tx = DigiXrosTransaction::new(handle(1), 0, 9, vec![slot]);
        tx.add_selected_material(DigiXrosSelectedMaterial::new(
            DigiXrosMaterialOrigin::Hand {
                player: 0,
                index: 0,
                card: handle(2),
            },
            Some(0),
            -2,
        ));
        tx.add_pre_attached_material(DigiXrosSelectedMaterial::new(
            DigiXrosMaterialOrigin::BattleArea {
                permanent: PermanentHandle {
                    player: 0,
                    index: 0,
                },
                card: handle(3),
            },
            Some(0),
            -1,
        ));

        assert_eq!(tx.digixros_count, 2);
        assert_eq!(tx.final_cost(), 6);
    }

    #[test]
    fn transaction_tracks_transaction_scoped_zone_allowances() {
        let tx = DigiXrosTransaction::new(handle(1), 0, 5, Vec::new());
        assert!(!tx.is_zone_allowed(DigiXrosMaterialZone::UnderTamer));

        let mut tx = tx;
        tx.allow_zone(DigiXrosZoneAllowance {
            zone: DigiXrosMaterialZone::UnderTamer,
            max_count: Some(1),
        });

        assert!(tx.is_zone_allowed(DigiXrosMaterialZone::UnderTamer));
        assert_eq!(
            tx.zone_allowances
                .get(&DigiXrosMaterialZone::UnderTamer)
                .and_then(|allowance| allowance.max_count),
            Some(1)
        );
    }

    fn test_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
        CardData {
            card_id: card_id.to_string(),
            card_name: name.to_string(),
            card_kind: crate::enums::CardKind::Digimon,
            level: Some(3),
            dp: Some(1000),
            play_cost: 3,
            colors: vec![crate::enums::CardColor::Red],
            traits: traits
                .iter()
                .map(|trait_name| trait_name.to_string())
                .collect(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            effect_class_name: String::new(),
            index: 0,
            norm_id: 0.0,
            keywords: Vec::new(),
            ace_overflow: None,
            dual: None,
            digixros_aliases: Vec::new(),
            also_treated_as: Vec::new(),
        }
    }

    fn shoutmon_slot() -> DigiXrosRecipeSlot {
        let mut slot = DigiXrosRecipeSlot::new(0);
        slot.names.push("Shoutmon".to_string());
        slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
        slot.allowed_zones.insert(DigiXrosMaterialZone::BattleArea);
        slot
    }

    fn xros_heart_slot(slot_index: usize) -> DigiXrosRecipeSlot {
        let mut slot = DigiXrosRecipeSlot::new(slot_index);
        slot.traits.push("Xros Heart".to_string());
        slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
        slot.allowed_zones.insert(DigiXrosMaterialZone::BattleArea);
        slot
    }

    fn named_slot(slot_index: usize, name: &str) -> DigiXrosRecipeSlot {
        let mut slot = DigiXrosRecipeSlot::new(slot_index);
        slot.names.push(name.to_string());
        slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
        slot.allowed_zones.insert(DigiXrosMaterialZone::BattleArea);
        slot
    }

    #[test]
    fn transaction_validates_material_zone_recipe_and_duplicates() {
        let mut tx = DigiXrosTransaction::new(handle(1), 0, 9, vec![shoutmon_slot()]);
        let shoutmon = test_card("BT10-008", "Shoutmon", &["Xros Heart"]);
        let unrelated = test_card("BT10-999", "Agumon", &["Dinosaur"]);

        let under_tamer = DigiXrosMaterialOrigin::UnderTamer {
            tamer: PermanentHandle {
                player: 0,
                index: 0,
            },
            source_index: 0,
            card: handle(2),
        };
        assert_eq!(
            tx.validate_material_origin(under_tamer, &shoutmon),
            Err(DigiXrosMaterialValidationError::ZoneNotAllowed)
        );

        let hand_origin = DigiXrosMaterialOrigin::Hand {
            player: 0,
            index: 0,
            card: handle(2),
        };
        assert_eq!(tx.try_select_material(hand_origin, &shoutmon), Ok(0));
        assert_eq!(
            tx.validate_material_origin(hand_origin, &shoutmon),
            Err(DigiXrosMaterialValidationError::AlreadySelected)
        );

        let other_hand_origin = DigiXrosMaterialOrigin::Hand {
            player: 0,
            index: 1,
            card: handle(3),
        };
        assert_eq!(
            tx.validate_material_origin(other_hand_origin, &unrelated),
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        );
        assert_eq!(
            tx.validate_material_origin(other_hand_origin, &shoutmon),
            Err(DigiXrosMaterialValidationError::RecipeSlotFull)
        );
    }

    #[test]
    fn transaction_extends_under_tamer_access_for_this_transaction_with_count_cap() {
        let mut tx = DigiXrosTransaction::new(
            handle(1),
            0,
            9,
            vec![xros_heart_slot(0), xros_heart_slot(1)],
        );
        let shoutmon = test_card("BT10-008", "Shoutmon", &["Xros Heart"]);
        let ballistamon = test_card("BT10-049", "Ballistamon", &["Xros Heart"]);
        let first_under_tamer = DigiXrosMaterialOrigin::UnderTamer {
            tamer: PermanentHandle {
                player: 0,
                index: 0,
            },
            source_index: 0,
            card: handle(2),
        };
        let second_under_tamer = DigiXrosMaterialOrigin::UnderTamer {
            tamer: PermanentHandle {
                player: 0,
                index: 0,
            },
            source_index: 1,
            card: handle(3),
        };

        assert_eq!(
            tx.validate_material_origin(first_under_tamer, &shoutmon),
            Err(DigiXrosMaterialValidationError::ZoneNotAllowed)
        );

        tx.allow_zone(DigiXrosZoneAllowance {
            zone: DigiXrosMaterialZone::UnderTamer,
            max_count: Some(1),
        });
        assert_eq!(tx.try_select_material(first_under_tamer, &shoutmon), Ok(0));
        assert_eq!(
            tx.validate_material_origin(second_under_tamer, &ballistamon),
            Err(DigiXrosMaterialValidationError::ZoneLimitReached)
        );
    }

    #[test]
    fn transaction_extends_trash_access_for_this_transaction_with_count_cap() {
        let mut tx = DigiXrosTransaction::new(
            handle(1),
            0,
            9,
            vec![xros_heart_slot(0), xros_heart_slot(1)],
        );
        let shoutmon = test_card("BT10-008", "Shoutmon", &["Xros Heart"]);
        let ballistamon = test_card("BT10-049", "Ballistamon", &["Xros Heart"]);
        let first_trash = DigiXrosMaterialOrigin::Trash {
            player: 0,
            index: 0,
            card: handle(2),
        };
        let second_trash = DigiXrosMaterialOrigin::Trash {
            player: 0,
            index: 1,
            card: handle(3),
        };

        assert_eq!(
            tx.validate_material_origin(first_trash, &shoutmon),
            Err(DigiXrosMaterialValidationError::ZoneNotAllowed)
        );

        tx.allow_zone(DigiXrosZoneAllowance {
            zone: DigiXrosMaterialZone::Trash,
            max_count: Some(1),
        });
        assert_eq!(tx.try_select_material(first_trash, &shoutmon), Ok(0));
        assert_eq!(
            tx.validate_material_origin(second_trash, &ballistamon),
            Err(DigiXrosMaterialValidationError::ZoneLimitReached)
        );
    }

    #[test]
    fn transaction_wildcard_replaces_one_unfilled_requirement_without_identity_alias() {
        let ballistamon_slot = named_slot(0, "Ballistamon");
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let origin = DigiXrosMaterialOrigin::BattleArea {
            permanent: PermanentHandle {
                player: 0,
                index: 0,
            },
            card: handle(2),
        };
        let mut tx = DigiXrosTransaction::new(handle(1), 0, 5, vec![ballistamon_slot.clone()]);

        assert!(!slot_accepts_card(&ballistamon_slot, &king));
        assert!(!matches_generic_name_requirement(&king, "Ballistamon"));
        assert_eq!(
            tx.validate_material_origin(origin, &king),
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        );

        tx.add_wildcard_substitution(DigiXrosWildcardSubstitution::once(handle(2)));

        assert_eq!(tx.validate_material_origin(origin, &king), Ok(0));
        assert_eq!(tx.try_select_material(origin, &king), Ok(0));
        assert_eq!(tx.final_cost(), 4);
        assert_eq!(tx.wildcard_substitutions[0].remaining_uses, 0);
    }

    #[test]
    fn transaction_wildcard_is_consumed_and_does_not_make_other_cards_wild() {
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let starmons = test_card("BT10-011", "Starmons", &["Xros Heart"]);
        let mut tx = DigiXrosTransaction::new(
            handle(1),
            0,
            5,
            vec![named_slot(0, "Ballistamon"), named_slot(1, "Dorulumon")],
        );
        let king_origin = DigiXrosMaterialOrigin::BattleArea {
            permanent: PermanentHandle {
                player: 0,
                index: 0,
            },
            card: handle(2),
        };
        let starmons_origin = DigiXrosMaterialOrigin::BattleArea {
            permanent: PermanentHandle {
                player: 0,
                index: 1,
            },
            card: handle(3),
        };

        tx.add_wildcard_substitution(DigiXrosWildcardSubstitution::once(handle(2)));

        assert_eq!(tx.try_select_material(king_origin, &king), Ok(0));
        assert_eq!(
            tx.validate_material_origin(starmons_origin, &starmons),
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        );
        assert_eq!(tx.selected_materials[0].recipe_slot, Some(0));
    }

    #[test]
    fn transaction_preattaches_material_and_applies_one_shot_cost_delta() {
        let mut tx = DigiXrosTransaction::new(handle(1), 0, 15, vec![shoutmon_slot()]);
        let shoutmon = test_card("BT10-008", "Shoutmon", &["Xros Heart"]);
        let origin = DigiXrosMaterialOrigin::BattleArea {
            permanent: PermanentHandle {
                player: 0,
                index: 0,
            },
            card: handle(2),
        };

        assert_eq!(tx.try_pre_attach_material(origin, &shoutmon, -1), Ok(0));
        tx.add_one_shot_cost_delta(-1);

        assert_eq!(tx.digixros_count, 1);
        assert_eq!(tx.pre_attached_materials.len(), 1);
        assert_eq!(tx.selected_materials.len(), 0);
        assert_eq!(tx.final_cost(), 13);
        assert_eq!(
            tx.validate_material_origin(origin, &shoutmon),
            Err(DigiXrosMaterialValidationError::AlreadySelected)
        );
    }

    #[test]
    fn optional_transaction_modifier_decline_or_abort_leaves_transaction_unchanged() {
        let mut tx = DigiXrosTransaction::new(handle(1), 0, 15, vec![shoutmon_slot()]);
        let unchanged = tx.clone();

        assert!(!tx.apply_optional_modifier(false, |transaction| {
            transaction.allow_zone(DigiXrosZoneAllowance {
                zone: DigiXrosMaterialZone::Trash,
                max_count: Some(1),
            });
            transaction.add_one_shot_cost_delta(-3);
            true
        }));
        assert_eq!(tx, unchanged);

        assert!(!tx.apply_optional_modifier(true, |transaction| {
            transaction.allow_zone(DigiXrosZoneAllowance {
                zone: DigiXrosMaterialZone::UnderTamer,
                max_count: Some(1),
            });
            transaction.add_one_shot_cost_delta(-2);
            false
        }));
        assert_eq!(tx, unchanged);

        assert!(tx.apply_optional_modifier(true, |transaction| {
            transaction.add_one_shot_cost_delta(-1);
            true
        }));
        assert_eq!(tx.final_cost(), 14);
    }

    #[cfg(feature = "dsl-yaml-loader")]
    fn test_digixros_yaml() -> &'static str {
        r#"
card: DX-TEST
name: Test DigiXros
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { trait_has: Xros Heart }
        repeat: { min: 1, max: 1 }
    cost:
      formula:
        base: 5
        per: material_count
        delta: -2
"#
    }

    #[cfg(feature = "dsl-yaml-loader")]
    fn ballistamon_target_yaml() -> &'static str {
        r#"
card: DX-BALLISTAMON
name: Ballistamon Requirement Target
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_is: Ballistamon }
        zones: [battle_area]
        cost_delta: -2
    cost: 5
"#
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn pending_transaction_expires_after_digixros_play_resolves() {
        let mut runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(test_digixros_yaml())
            .expect("compile inline DigiXros DSL")
            .hand(0, &["DX-TEST"])
            .memory(10)
            .start();

        assert!(runner.game.pending_digixros_transaction.is_none());
        assert_eq!(runner.play(0, 0), Some(0));
        assert!(runner.game.pending_digixros_transaction.is_none());
        assert_eq!(runner.memory(), 5);
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn pending_transaction_expires_after_digixros_play_aborts() {
        let mut runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(test_digixros_yaml())
            .expect("compile inline DigiXros DSL")
            .hand(0, &["DX-TEST"])
            .memory(-6)
            .start();

        assert_eq!(runner.play(0, 0), None);
        assert!(runner.game.pending_digixros_transaction.is_none());
        assert_eq!(runner.memory(), -6);
        assert_eq!(runner.game.player(0).hand.len(), 1);
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn dsl_digixros_path_lowers_slot_zones_and_cost_deltas_into_transaction() {
        let yaml = r#"
card: DX-SLOT-COST
name: DigiXros Slot Cost
kind: digimon
level: 4
color: [red]
cost: 6
dp: 5000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_contains: Shoutmon }
        zones: [hand, battle_area]
        cost_delta: -2
      - filter: { name_contains: Ballistamon }
        zones: [trash]
        cost_delta: -1
    cost: 6
"#;
        let runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(yaml)
            .expect("compile inline DigiXros DSL")
            .hand(0, &["DX-SLOT-COST"])
            .start();

        let tx = runner
            .game
            .build_digixros_transaction_for_hand_card(0, 0)
            .expect("DigiXros transaction");
        assert_eq!(tx.recipe_slots.len(), 2);
        assert_eq!(tx.recipe_slots[0].cost_delta_per_material, -2);
        assert_eq!(tx.recipe_slots[1].cost_delta_per_material, -1);
        assert!(tx.recipe_slots[0]
            .allowed_zones
            .contains(&DigiXrosMaterialZone::Hand));
        assert!(tx.recipe_slots[0]
            .allowed_zones
            .contains(&DigiXrosMaterialZone::BattleArea));
        assert!(tx.recipe_slots[1]
            .allowed_zones
            .contains(&DigiXrosMaterialZone::Trash));
        assert!(!tx.recipe_slots[1]
            .allowed_zones
            .contains(&DigiXrosMaterialZone::Hand));
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn dsl_transaction_steps_preattach_bound_material_and_apply_cost_delta() {
        use crate::dsl_cards::bindings::Bindings;
        use crate::dsl_cards::step::run_steps;
        use crate::effect_context::EffectContext;
        use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

        let shoutmon = test_card("DX-SHOUT", "Shoutmon", &["Xros Heart"]);
        let mut runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(test_digixros_yaml())
            .expect("compile inline DigiXros DSL")
            .add_card(shoutmon)
            .hand(0, &["DX-TEST", "DX-SHOUT"])
            .start();
        runner.game.pending_digixros_transaction =
            runner.game.build_digixros_transaction_for_hand_card(0, 0);

        let picked = runner.game.player(0).hand[1].handle();
        let mut bindings = Bindings::new();
        bindings.insert_card("pick", picked);
        let steps = vec![
            CompiledStep::PreattachDigixrosMaterial {
                card: CompiledBindingRef::Named("pick".to_string()),
                cost_delta: -1,
            },
            CompiledStep::AddDigixrosCostDelta { delta: -1 },
        ];
        let mut ctx = EffectContext::new(&mut runner.game, picked, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);

        let tx = runner
            .game
            .pending_digixros_transaction
            .as_ref()
            .expect("transaction remains pending");
        assert_eq!(tx.pre_attached_materials.len(), 1);
        assert_eq!(tx.final_cost(), 3);
        assert_eq!(tx.digixros_count, 1);
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn turn_scoped_wildcard_copies_into_later_transaction_and_expires() {
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let mut runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(ballistamon_target_yaml())
            .expect("compile inline DigiXros DSL")
            .add_card(king)
            .hand(0, &["DX-BALLISTAMON"])
            .start();
        let permanent = runner.place_on_field(0, "BT10-111", Some(0));
        let material_card = runner.game.player(0).battle_area[permanent.index as usize]
            .top_card()
            .handle();
        let origin = DigiXrosMaterialOrigin::BattleArea {
            permanent,
            card: material_card,
        };
        let card_data = runner
            .game
            .card_data_for_handle(material_card)
            .unwrap()
            .clone();

        runner.game.register_digixros_wildcard_for_current_turn(
            0,
            material_card,
            Some(DigiXrosMaterialZone::BattleArea),
        );
        let tx = runner
            .game
            .build_digixros_transaction_for_hand_card(0, 0)
            .expect("DigiXros transaction");
        assert_eq!(tx.validate_material_origin(origin, &card_data), Ok(0));

        runner.end_turn();
        let tx = runner
            .game
            .build_digixros_transaction_for_hand_card(0, 0)
            .expect("DigiXros transaction after expiry");
        assert_eq!(
            tx.validate_material_origin(origin, &card_data),
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        );
    }

    #[test]
    fn wildcard_required_zone_does_not_follow_card_to_other_zones() {
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let mut slot = named_slot(0, "Ballistamon");
        slot.allowed_zones.insert(DigiXrosMaterialZone::Hand);
        let mut tx = DigiXrosTransaction::new(handle(1), 0, 5, vec![slot]);
        tx.add_wildcard_substitution(DigiXrosWildcardSubstitution::once_from_zone(
            handle(2),
            DigiXrosMaterialZone::BattleArea,
        ));

        let hand_origin = DigiXrosMaterialOrigin::Hand {
            player: 0,
            index: 0,
            card: handle(2),
        };
        assert_eq!(
            tx.validate_material_origin(hand_origin, &king),
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        );
    }

    #[test]
    fn wildcard_substitution_does_not_satisfy_non_digixros_identity_predicates() {
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let mut tx = DigiXrosTransaction::new(handle(1), 0, 5, vec![named_slot(0, "Ballistamon")]);
        let origin = DigiXrosMaterialOrigin::BattleArea {
            permanent: PermanentHandle {
                player: 0,
                index: 0,
            },
            card: handle(2),
        };
        tx.add_wildcard_substitution(DigiXrosWildcardSubstitution::once_from_zone(
            handle(2),
            DigiXrosMaterialZone::BattleArea,
        ));

        assert_eq!(tx.validate_material_origin(origin, &king), Ok(0));
        assert!(!matches_generic_name_requirement(&king, "Ballistamon"));
        assert!(!king
            .traits
            .iter()
            .any(|trait_name| trait_name.eq_ignore_ascii_case("Machine")));
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn current_transaction_wildcard_does_not_leak_to_later_transactions() {
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let mut runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(ballistamon_target_yaml())
            .expect("compile inline DigiXros DSL")
            .add_card(king)
            .hand(0, &["DX-BALLISTAMON"])
            .start();
        let permanent = runner.place_on_field(0, "BT10-111", Some(0));
        let material_card = runner.game.player(0).battle_area[permanent.index as usize]
            .top_card()
            .handle();
        let origin = DigiXrosMaterialOrigin::BattleArea {
            permanent,
            card: material_card,
        };
        let card_data = runner
            .game
            .card_data_for_handle(material_card)
            .unwrap()
            .clone();
        runner.game.pending_digixros_transaction =
            runner.game.build_digixros_transaction_for_hand_card(0, 0);

        {
            let mut ctx = crate::effect_context::EffectContext::new(
                &mut runner.game,
                material_card,
                Some(permanent),
                0,
            );
            assert!(ctx.add_digixros_wildcard_to_pending_transaction(
                material_card,
                Some(DigiXrosMaterialZone::BattleArea),
            ));
        }
        assert_eq!(
            runner
                .game
                .pending_digixros_transaction
                .as_ref()
                .unwrap()
                .validate_material_origin(origin, &card_data),
            Ok(0)
        );

        runner.game.pending_digixros_transaction = None;
        let fresh = runner
            .game
            .build_digixros_transaction_for_hand_card(0, 0)
            .expect("fresh DigiXros transaction");
        assert_eq!(
            fresh.validate_material_origin(origin, &card_data),
            Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot)
        );
    }

    #[test]
    #[cfg(feature = "dsl-yaml-loader")]
    fn wildcard_material_is_prompted_once_and_masked_after_selection() {
        let king = test_card("BT10-111", "Shoutmon (King Version)", &["Xros Heart"]);
        let mut runner = crate::debug_runner::DebugRunner::builder()
            .from_dsl_yaml(ballistamon_target_yaml())
            .expect("compile inline DigiXros DSL")
            .add_card(king)
            .hand(0, &["DX-BALLISTAMON"])
            .memory(10)
            .start();
        let permanent = runner.place_on_field(0, "BT10-111", Some(0));
        let material_card = runner.game.player(0).battle_area[permanent.index as usize]
            .top_card()
            .handle();
        runner.game.register_digixros_wildcard_for_current_turn(
            0,
            material_card,
            Some(DigiXrosMaterialZone::BattleArea),
        );

        assert_eq!(runner.play(0, 0), None);
        let action_id = permanent.index as u16;
        assert!(
            runner
                .pending_selection()
                .expect("DigiXros material prompt")
                .valid_action_ids
                .contains(&action_id),
            "wildcard material must be exposed as a legal material action"
        );

        runner
            .execute_action(0, action_id)
            .expect("select wildcard material");
        assert!(
            !runner
                .pending_selection()
                .expect("follow-up DigiXros material prompt")
                .valid_action_ids
                .contains(&action_id),
            "selected wildcard material must be masked after its substitution is consumed"
        );
        runner
            .execute_action(0, crate::action::space::PASS)
            .expect("finish material selection");

        let target = runner.game.players[0]
            .battle_area
            .iter()
            .find(|permanent| {
                permanent.top_card().card_id(&runner.game.card_data) == "DX-BALLISTAMON"
            })
            .expect("DigiXros target entered");
        assert!(target
            .card_sources
            .iter()
            .any(|card| card.handle() == material_card));
    }
}
