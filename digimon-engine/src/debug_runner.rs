//! DebugRunner — a test harness for behavioral validation of card effects.
//!
//! Builds a Game with deterministic hands/decks (no shuffling) so tests can
//! play specific cards in a specific order and assert on the resulting state.
//!
//! Example:
//! ```ignore
//! let mut r = DebugRunner::builder()
//!     .with_card_data(test_card_data())
//!     .hand(0, &["TEST-001"])  // P0's hand
//!     .deck(0, &["FILLER"; 50])
//!     .start();
//! r.play(0, 0);  // P0 plays hand[0]
//! assert_eq!(r.memory(), 1);
//! ```

use std::collections::HashMap;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::cards::{build_registry, CardEffectRegistry};
use crate::enums::{CardKind, GamePhase, ModifierType, PlayerId};
use crate::game::Game;
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
use crate::rules::Rules;

/// A scripted game runner for behavioral tests.
pub struct DebugRunner {
    pub game: Game,
}

impl DebugRunner {
    pub fn builder() -> DebugRunnerBuilder {
        DebugRunnerBuilder::default()
    }

    // ─── Action helpers ───────────────────────────────────────────────

    /// Play a card from a player's hand. Returns the new field index.
    pub fn play(&mut self, player: PlayerId, hand_index: usize) -> Option<usize> {
        self.game.play_from_hand(player, hand_index)
    }

    /// Move from breeding to battle for a player.
    pub fn move_from_breeding(&mut self, player: PlayerId) -> bool {
        self.game.move_from_breeding(player)
    }

    /// End the current turn.
    pub fn end_turn(&mut self) {
        self.game.end_turn();
    }

    /// Pass turn (memory to -3).
    pub fn pass_turn(&mut self) {
        self.game.pass_turn();
    }

    /// Manually fire OnPlay for a permanent that's already on the field.
    pub fn fire_on_play(&mut self, player: PlayerId, field_index: usize) {
        self.game.fire_on_play(player, field_index);
    }

    /// Place a card directly on a player's field (bypass hand/play_from_hand).
    /// Useful for setting up combat scenarios without dealing with summoning sickness.
    /// `turn_played_override`: sets the permanent's turn_played; pass 0 to make it eligible
    /// to attack immediately, or `None` to use the current turn.
    pub fn place_on_field(
        &mut self,
        player: PlayerId,
        card_id: &str,
        turn_played_override: Option<u16>,
    ) -> PermanentHandle {
        // Find the card in card_data store.
        let data_idx = self
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap_or_else(|| panic!("place_on_field: unknown card_id {}", card_id));
        let next_idx = self.game.next_card_index();
        let mut card = CardSource::new(data_idx, player, next_idx);
        card.card_index = next_idx;
        let turn = turn_played_override.unwrap_or(self.game.turn_count);
        let perm = crate::permanent::Permanent::new(card, turn);
        self.game.players[player as usize].battle_area.push(perm);
        let index = self.game.players[player as usize].battle_area.len() - 1;
        PermanentHandle {
            player,
            index: index as u8,
        }
    }

    /// Attack a Digimon.
    pub fn attack_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> crate::combat::AttackResult {
        self.game.attack_digimon(attacker, defender)
    }

    /// Attack the opposing player's security.
    pub fn attack_player(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
    ) -> crate::combat::AttackResult {
        self.game.attack_player(attacker, defender_player)
    }

    /// Effective DP of a permanent (base + modifiers).
    pub fn effective_dp(&self, handle: PermanentHandle) -> Option<i32> {
        self.game.effective_dp(handle)
    }

    /// Get a permanent's security stack size for a player.
    pub fn security_count(&self, player: PlayerId) -> usize {
        self.game.player(player).security.len()
    }

    /// Check if game is over.
    pub fn game_over(&self) -> bool {
        self.game.game_over
    }

    pub fn winner(&self) -> Option<PlayerId> {
        self.game.winner
    }

    // ─── State queries ────────────────────────────────────────────────

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn current_phase(&self) -> GamePhase {
        self.game.current_phase
    }

    pub fn turn_player(&self) -> PlayerId {
        self.game.turn_player()
    }

    pub fn hand_size(&self, player: PlayerId) -> usize {
        self.game.player(player).hand.len()
    }

    pub fn battle_area_size(&self, player: PlayerId) -> usize {
        self.game.player(player).battle_area.len()
    }

    pub fn deck_size(&self, player: PlayerId) -> usize {
        self.game.player(player).deck.len()
    }

    pub fn trash_size(&self, player: PlayerId) -> usize {
        self.game.player(player).trash.len()
    }

    /// Compute total DP of a permanent (base + modifiers).
    pub fn dp_of(&self, h: PermanentHandle) -> Option<i32> {
        let perm = self
            .game
            .player(h.player)
            .battle_area
            .get(h.index as usize)?;
        let base = perm.base_dp(&self.game.card_data)?;
        let bonus = self.game.modifiers.sum(h, ModifierType::ChangeDp);
        Some(base + bonus)
    }

    /// Get the modifier registry for direct inspection.
    pub fn modifiers(&self) -> &ModifierRegistry {
        &self.game.modifiers
    }

    pub fn perm_handle(&self, player: PlayerId, field_index: usize) -> PermanentHandle {
        PermanentHandle {
            player,
            index: field_index as u8,
        }
    }
}

/// Builder for DebugRunner.
pub struct DebugRunnerBuilder {
    card_data: HashMap<String, CardData>,
    /// Cards explicitly placed in each player's hand.
    hands: HashMap<PlayerId, Vec<String>>,
    /// Cards explicitly placed in each player's deck (top of deck = end of vec).
    decks: HashMap<PlayerId, Vec<String>>,
    /// Cards explicitly placed in each player's security stack (top = end of vec).
    securities: HashMap<PlayerId, Vec<String>>,
    /// Cards explicitly placed in each player's digitama deck.
    digitamas: HashMap<PlayerId, Vec<String>>,
    rules: Rules,
    /// Custom registry (defaults to build_registry()).
    registry: Option<CardEffectRegistry>,
    /// Number of players (defaults to rules.player_count).
    player_count: Option<u8>,
}

impl Default for DebugRunnerBuilder {
    fn default() -> Self {
        Self {
            card_data: HashMap::new(),
            hands: HashMap::new(),
            decks: HashMap::new(),
            securities: HashMap::new(),
            digitamas: HashMap::new(),
            rules: Rules::standard(),
            registry: None,
            player_count: None,
        }
    }
}

impl DebugRunnerBuilder {
    /// Provide the full card database.
    pub fn with_card_data(mut self, data: HashMap<String, CardData>) -> Self {
        self.card_data = data;
        self
    }

    /// Add a single card definition to the database.
    pub fn add_card(mut self, card: CardData) -> Self {
        self.card_data.insert(card.card_id.clone(), card);
        self
    }

    /// Set the rules. Default is `Rules::standard()`.
    pub fn with_rules(mut self, rules: Rules) -> Self {
        self.rules = rules;
        self
    }

    /// Override the card effect registry (e.g. to inject custom test effects).
    pub fn with_registry(mut self, registry: CardEffectRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Set the explicit hand for a player.
    pub fn hand(mut self, player: PlayerId, card_ids: &[&str]) -> Self {
        self.hands
            .insert(player, card_ids.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set the explicit deck for a player (last element = top of deck).
    pub fn deck(mut self, player: PlayerId, card_ids: &[&str]) -> Self {
        self.decks
            .insert(player, card_ids.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set the explicit security stack for a player.
    pub fn security(mut self, player: PlayerId, card_ids: &[&str]) -> Self {
        self.securities
            .insert(player, card_ids.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set the explicit digitama deck for a player.
    pub fn digitama(mut self, player: PlayerId, card_ids: &[&str]) -> Self {
        self.digitamas
            .insert(player, card_ids.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Override the player count (defaults to rules.player_count).
    pub fn player_count(mut self, n: u8) -> Self {
        self.player_count = Some(n);
        self
    }

    /// Build the runner and start the game (advances past Mulligan into turn 1).
    pub fn start(mut self) -> DebugRunner {
        let mut r = self.build_inner();
        r.game.start_game();
        r
    }

    /// Build the runner without starting the game (stays in Mulligan).
    pub fn build(mut self) -> DebugRunner {
        self.build_inner()
    }

    fn build_inner(&mut self) -> DebugRunner {
        let player_count = self
            .player_count
            .unwrap_or(self.rules.player_count);

        // We bypass Game::new because it shuffles/deals from decks, which loses
        // determinism. Instead, build an empty Game with the shared card_data and
        // manually populate each player's zones.

        // Build the card_data store and lookup map.
        let mut card_data_store: Vec<CardData> = Vec::new();
        let mut data_index_map: HashMap<String, usize> = HashMap::new();
        for (id, data) in &self.card_data {
            let idx = card_data_store.len();
            data_index_map.insert(id.clone(), idx);
            card_data_store.push(data.clone());
        }

        // Build empty decks (one per player) and pass through Game::new with
        // rules adjusted to the actual player_count we want.
        let mut rules = self.rules.clone();
        rules.player_count = player_count;

        // Game::new requires deck_card_ids.len() == rules.player_count, but doesn't
        // require non-empty decks. We pass empty decks and populate zones manually.
        let empty_decks: Vec<Vec<String>> = (0..player_count).map(|_| Vec::new()).collect();
        let mut game = Game::new(&empty_decks, &self.card_data, rules, Some(0xC0FFEE))
            .expect("DebugRunner: Game::new failed");

        // Wipe any cards Game::new placed (it populates from empty decks, so this
        // should be a no-op, but we want to be defensive).
        for p in &mut game.players {
            p.hand.clear();
            p.deck.clear();
            p.security.clear();
            p.digitama_deck.clear();
            p.battle_area.clear();
            p.breeding_area = None;
        }

        // Populate each player's zones from the builder spec.
        let mut next_card_index: u16 = 0;
        for player_idx in 0..player_count {
            let pid = player_idx as PlayerId;

            if let Some(ids) = self.hands.get(&pid) {
                for card_id in ids {
                    let card =
                        Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    game.players[pid as usize].hand.push(card);
                }
            }
            if let Some(ids) = self.decks.get(&pid) {
                for card_id in ids {
                    let card =
                        Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    game.players[pid as usize].deck.push(card);
                }
            }
            if let Some(ids) = self.securities.get(&pid) {
                for card_id in ids {
                    let card =
                        Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    game.players[pid as usize].security.push(card);
                }
            }
            if let Some(ids) = self.digitamas.get(&pid) {
                for card_id in ids {
                    let card =
                        Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    if card_data_store[card.data_index].card_kind != CardKind::DigiEgg {
                        // Allow non-eggs in digitama for tests, but warn via debug.
                    }
                    game.players[pid as usize].digitama_deck.push(card);
                }
            }
        }

        // Replace the registry if the user supplied one.
        if let Some(reg) = self.registry.take() {
            game.effect_registry = reg;
        } else {
            // Default registry already populated by Game::new -> build_registry().
            // Make sure test cards are registered (build_registry() does this).
            let _ = build_registry; // silence unused-import if path changes
        }

        DebugRunner { game }
    }

    fn make_card(
        index_map: &HashMap<String, usize>,
        card_id: &str,
        owner: PlayerId,
        next_idx: &mut u16,
    ) -> CardSource {
        let data_idx = index_map
            .get(card_id)
            .unwrap_or_else(|| panic!("DebugRunner: card_id {} not in card_data", card_id));
        let card = CardSource::new(*data_idx, owner, *next_idx);
        *next_idx += 1;
        card
    }
}

// ─── Convenience constructors for synthetic test cards ────────────────

/// Build a minimal CardData entry suitable for tests.
/// Leaves `index` / `norm_id` at 0 so the registry falls into alphabetical mode.
pub fn make_test_card(card_id: &str, card_name: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(2000),
        play_cost: 3,
        colors: vec![crate::enums::CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Build a minimal DigiEgg CardData entry suitable for tests.
pub fn make_test_egg(card_id: &str, card_name: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_name.to_string(),
        card_kind: CardKind::DigiEgg,
        level: Some(2),
        dp: None,
        play_cost: 0,
        colors: vec![crate::enums::CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_starts_in_turn_1() {
        let r = DebugRunner::builder()
            .add_card(make_test_card("TEST-001", "TestOne"))
            .hand(0, &["TEST-001"])
            .start();
        assert_eq!(r.turn_count(), 1);
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(r.hand_size(1), 0);
    }

    #[test]
    fn play_test_001_gains_memory() {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-001", "TestOne"))
            .hand(0, &["TEST-001"])
            .start();
        // Memory starts at 3 for the active player on turn 1.
        let m_before = r.memory();
        r.play(0, 0);
        assert_eq!(r.memory(), m_before + 1);
        assert_eq!(r.battle_area_size(0), 1);
    }
}
