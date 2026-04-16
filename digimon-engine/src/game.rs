use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::cards::{build_registry, CardEffectRegistry};
use crate::effect_context::EffectContext;
use crate::enums::{GamePhase, PlayerId, SkipDraw};
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
use crate::player::Player;
use crate::rules::Rules;

/// The core game state. Drives the turn state machine.
#[derive(Debug)]
pub struct Game {
    pub rules: Rules,
    pub players: Vec<Player>,
    pub turn_count: u16,
    pub current_phase: GamePhase,
    /// Memory seesaw value. Positive = favor of memory_pair.0, negative = favor of memory_pair.1.
    pub memory: i16,
    /// The active pair for the memory seesaw: (active_player, next_player).
    pub memory_pair: (PlayerId, PlayerId),
    /// Turn rotation order. Eliminated players are removed.
    pub turn_order: Vec<PlayerId>,
    /// Index into turn_order for the current turn player.
    pub turn_player_idx: usize,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    /// Shared card data store (all cards in the game reference into this).
    pub card_data: Vec<CardData>,
    /// Active modifiers (DP buffs, granted keywords, etc.) attached to permanents.
    pub modifiers: ModifierRegistry,
    /// Card effect registry — maps card_id to effect implementations.
    pub effect_registry: CardEffectRegistry,
    /// RNG for shuffling and random effects.
    pub rng: StdRng,
    /// Counter for assigning unique card instance indices.
    next_card_index: u16,
}

impl Game {
    /// Create a new game with the given decks and rules.
    /// `deck_card_ids` is one deck per player, each a list of card_id strings.
    /// `all_card_data` is the full card database.
    pub fn new(
        deck_card_ids: &[Vec<String>],
        all_card_data: &std::collections::HashMap<String, CardData>,
        rules: Rules,
        seed: Option<u64>,
    ) -> Result<Self, String> {
        if deck_card_ids.len() != rules.player_count as usize {
            return Err(format!(
                "Expected {} decks, got {}",
                rules.player_count,
                deck_card_ids.len()
            ));
        }

        let mut rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        // Build card data store (flat vec, indexed by position)
        let mut card_data_store: Vec<CardData> = Vec::new();
        let mut data_index_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for (card_id, data) in all_card_data {
            let idx = card_data_store.len();
            data_index_map.insert(card_id.clone(), idx);
            card_data_store.push(data.clone());
        }

        let mut next_card_index: u16 = 0;

        // Create players and populate decks
        let mut players = Vec::with_capacity(rules.player_count as usize);
        for (player_idx, deck_ids) in deck_card_ids.iter().enumerate() {
            let player_id = player_idx as PlayerId;
            let mut player = Player::new(player_id);

            for card_id in deck_ids {
                let data_idx = data_index_map.get(card_id).ok_or_else(|| {
                    format!("Card {} not found in card database", card_id)
                })?;

                let card_data = &card_data_store[*data_idx];
                let card = CardSource::new(*data_idx, player_id, next_card_index);
                next_card_index += 1;

                // Route to correct deck based on card kind
                if card_data.card_kind == crate::enums::CardKind::DigiEgg {
                    player.digitama_deck.push(card);
                } else {
                    player.deck.push(card);
                }
            }

            // Shuffle decks
            player.shuffle_deck(&mut rng);
            player.shuffle_digitama_deck(&mut rng);

            players.push(player);
        }

        // Build turn order
        let turn_order: Vec<PlayerId> = (0..rules.player_count).collect();
        let memory_pair = (0, 1); // P0 active, P1 next

        let mut game = Self {
            rules,
            players,
            turn_count: 0,
            current_phase: GamePhase::Mulligan,
            memory: 0,
            memory_pair,
            turn_order,
            turn_player_idx: 0,
            game_over: false,
            winner: None,
            card_data: card_data_store,
            modifiers: ModifierRegistry::new(),
            effect_registry: build_registry(),
            rng,
            next_card_index,
        };

        // Deal starting hands and security
        for i in 0..game.rules.player_count as usize {
            game.players[i].draw_many(game.rules.starting_hand);
            game.players[i].setup_security(game.rules.security_count);
        }

        Ok(game)
    }

    /// Get the current turn player's ID.
    pub fn turn_player(&self) -> PlayerId {
        self.turn_order[self.turn_player_idx]
    }

    /// Get a reference to a player by ID.
    pub fn player(&self, id: PlayerId) -> &Player {
        &self.players[id as usize]
    }

    /// Get a mutable reference to a player by ID.
    pub fn player_mut(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id as usize]
    }

    /// Get all non-eliminated opponents of a player.
    pub fn opponents(&self, id: PlayerId) -> Vec<PlayerId> {
        self.turn_order
            .iter()
            .copied()
            .filter(|&pid| pid != id)
            .collect()
    }

    /// Get the next player clockwise from the given player.
    pub fn next_clockwise(&self, id: PlayerId) -> PlayerId {
        let pos = self
            .turn_order
            .iter()
            .position(|&p| p == id)
            .unwrap_or(0);
        let next_pos = (pos + 1) % self.turn_order.len();
        self.turn_order[next_pos]
    }

    /// Start the game: transition from Mulligan to first turn.
    /// In a full implementation, mulligan decisions would be handled via step().
    /// For now, skip mulligan and go directly to turn 1.
    pub fn start_game(&mut self) {
        self.turn_count = 1;
        self.memory = 0;
        self.begin_turn();
    }

    /// Begin a new turn for the current turn player.
    fn begin_turn(&mut self) {
        let tp = self.turn_player();

        // Reset per-turn state
        self.player_mut(tp).new_turn();

        // Unsuspend phase
        self.current_phase = GamePhase::Unsuspend;
        self.player_mut(tp).unsuspend_all();

        // Draw phase
        self.current_phase = GamePhase::Draw;
        let should_skip_draw = match self.rules.skip_first_draw {
            SkipDraw::P1Only => self.turn_count == 1 && tp == 0,
            SkipDraw::AllRound1 => {
                self.turn_count <= self.rules.player_count as u16
            }
            SkipDraw::None => false,
        };
        if !should_skip_draw {
            let drew = self.player_mut(tp).draw();
            if !drew {
                // Deck-out: player is eliminated (multiplayer) or loses (standard)
                self.handle_deckout(tp);
                return;
            }
        }

        // Breeding phase
        self.current_phase = GamePhase::Breeding;
        // Breeding actions handled via step() — move to main if no breeding action
    }

    /// Advance from breeding to main phase.
    pub fn enter_main_phase(&mut self) {
        self.current_phase = GamePhase::Main;
    }

    /// End the current turn and advance to the next player.
    ///
    /// Fires OnEndTurn effects (when the effect system is wired to this),
    /// checks memory swing-back (§1.5): if an OnEndTurn effect restored memory
    /// from negative to non-negative, the turn continues and returns to Main
    /// phase instead of switching. Matches Python `_complete_end_phase`.
    pub fn end_turn(&mut self) {
        if self.game_over {
            return;
        }

        self.current_phase = GamePhase::EndTurn;

        // Memory swing-back: capture memory before firing OnEndTurn effects,
        // fire them, then see if an effect restored memory from negative.
        let memory_before = self.memory;
        let ending_player = self.turn_player();
        self.fire_end_of_your_turn(ending_player);

        if memory_before < 0 && self.memory >= 0 && !self.game_over {
            self.current_phase = GamePhase::Main;
            return;
        }

        // Expire end-of-turn modifiers/keywords for the ending player's turn.
        self.modifiers.expire_end_of_turn(ending_player);

        // Advance turn
        self.turn_player_idx = (self.turn_player_idx + 1) % self.turn_order.len();
        self.turn_count += 1;

        // Update memory pair for the new active player
        let new_active = self.turn_player();
        let new_next = self.next_clockwise(new_active);
        self.memory_pair = (new_active, new_next);

        // Flip the seesaw. Memory is always expressed from the active player's
        // perspective: positive = their side, negative = opponent's side. When
        // the turn switches, the new active player sees the opposite sign.
        // Matches Python's `switch_turn`: `self.memory = -self.memory`.
        //
        // No clamping. Over-cost plays that pushed memory deep negative carry
        // their magnitude across the switch as positive memory for the next
        // player — that's the intended tempo consequence.
        self.memory = -self.memory;

        // Check max turns
        if self.turn_count > self.rules.max_turns {
            self.game_over = true;
            // Draw - no winner
            self.current_phase = GamePhase::GameOver;
            return;
        }

        self.begin_turn();
    }

    /// Pass action: give the next player 3 memory, then end turn.
    ///
    /// Only forces memory to -3 if the passing player still had memory to give
    /// (i.e., memory >= 0). If memory is already negative — because an
    /// over-cost play pushed it there — that overflow is preserved and carried
    /// through the turn switch. Matches Python `game.pass_turn`.
    pub fn pass_turn(&mut self) {
        if self.memory >= 0 {
            self.memory = -3;
        }
        self.end_turn();
    }

    /// Pay memory cost. Returns `true` if affordable (memory stays above
    /// `rules.memory_range.0`).
    ///
    /// Does **not** end the turn even if memory crosses zero. Python's rule:
    /// the turn only ends when `check_turn_end()` is called (typically after
    /// all OnPlay/WhenDigivolving/etc. effects have resolved). Callers should
    /// invoke `check_turn_end()` at the natural resolution boundary.
    pub fn pay_memory(&mut self, cost: u16) -> bool {
        let new_memory = self.memory - cost as i16;
        if new_memory < self.rules.memory_range.0 {
            return false;
        }
        self.memory = new_memory;
        true
    }

    /// End the turn if memory has crossed to the opponent's side.
    /// Call this after a batch of effects resolves (not synchronously inside
    /// `pay_memory`, which would starve effects of their turn).
    pub fn check_turn_end(&mut self) {
        if self.memory < 0 && !self.game_over {
            self.end_turn();
        }
    }

    /// Gain memory for the active player.
    pub fn gain_memory(&mut self, amount: i16) {
        self.memory = (self.memory + amount).min(self.rules.memory_range.1);
    }

    /// Set memory to a specific value.
    pub fn set_memory(&mut self, value: i16) {
        self.memory = value.clamp(self.rules.memory_range.0, self.rules.memory_range.1);
    }

    /// Handle deck-out for a player.
    fn handle_deckout(&mut self, player_id: PlayerId) {
        if self.rules.player_count == 2 {
            // Standard: deck-out = loss
            self.game_over = true;
            let opponents = self.opponents(player_id);
            self.winner = opponents.first().copied();
            self.current_phase = GamePhase::GameOver;
        } else {
            // Multiplayer: elimination
            self.eliminate_player(player_id);
        }
    }

    /// Eliminate a player (multiplayer modes).
    pub fn eliminate_player(&mut self, player_id: PlayerId) {
        self.players[player_id as usize].is_eliminated = true;

        // Remove from turn order
        self.turn_order.retain(|&p| p != player_id);

        // Check if only one player remains
        if self.turn_order.len() == 1 {
            self.game_over = true;
            self.winner = Some(self.turn_order[0]);
            self.current_phase = GamePhase::GameOver;
        }

        // Adjust turn_player_idx if needed
        if self.turn_player_idx >= self.turn_order.len() {
            self.turn_player_idx = 0;
        }
    }

    /// Declare a winner (e.g., after a direct attack on a player with 0 security).
    pub fn declare_winner(&mut self, winner_id: PlayerId) {
        self.game_over = true;
        self.winner = Some(winner_id);
        self.current_phase = GamePhase::GameOver;
    }

    /// Allocate a new unique card index (for tokens, etc.).
    pub fn next_card_index(&mut self) -> u16 {
        let idx = self.next_card_index;
        self.next_card_index += 1;
        idx
    }

    // --- Convenience methods that avoid borrow conflicts ---

    /// Hatch for a player (copies turn_count to avoid borrow conflict).
    pub fn hatch(&mut self, player_id: PlayerId) -> bool {
        let turn = self.turn_count;
        self.player_mut(player_id).hatch(turn)
    }

    /// Move from breeding to battle area for a player.
    pub fn move_from_breeding(&mut self, player_id: PlayerId) -> bool {
        let field_slots = self.rules.field_slots;
        let player = self.player_mut(player_id);
        if player.battle_area.len() >= field_slots as usize {
            return false;
        }
        if let Some(perm) = player.breeding_area.take() {
            player.battle_area.push(perm);
            true
        } else {
            false
        }
    }

    /// Play a card from hand to field for a player.
    ///
    /// Flow (matches Python):
    /// 1. Validate hand index and field capacity.
    /// 2. Read the card's play cost from `card_data`.
    /// 3. Call `pay_memory(cost)`; if unaffordable, abort with `None` and
    ///    leave state unchanged.
    /// 4. Remove the card from hand, create a Permanent on the field.
    /// 5. Fire `OnPlay` effects via the registry.
    ///
    /// Cost reduction (BeforePayCost scanning, DigiXros reductions, etc.) is
    /// not yet implemented — the base `play_cost` is used verbatim. See §4.7
    /// and the deferred list in docs/RUST_PYTHON_PARITY.md.
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> Option<usize> {
        let turn = self.turn_count;
        let field_slots = self.rules.field_slots;

        // Borrow-check-friendly pre-checks: gather everything we need from
        // immutable borrows before taking a mutable borrow.
        let cost = {
            let player = self.player(player_id);
            if hand_index >= player.hand.len() {
                return None;
            }
            if player.battle_area.len() >= field_slots as usize {
                return None;
            }
            player.hand[hand_index].play_cost(&self.card_data)
        };

        // Pay the cost up-front. If unaffordable, do not remove the card.
        if !self.pay_memory(cost) {
            return None;
        }

        // Now the cost is paid — commit the play.
        let player = self.player_mut(player_id);
        let card = player.hand.remove(hand_index);
        let perm = crate::permanent::Permanent::new(card, turn);
        player.battle_area.push(perm);
        let field_index = player.battle_area.len() - 1;

        self.fire_on_play(player_id, field_index);

        Some(field_index)
    }

    /// Fire `EndOfYourTurn` effects on every permanent in `player`'s battle area.
    /// Called by `end_turn`; exposed for tests that want to trigger swing-back.
    pub fn fire_end_of_your_turn(&mut self, player: PlayerId) {
        // Snapshot (card_id, handle) for each permanent up-front, because
        // firing an effect could mutate the battle_area (e.g. self-deletion).
        let snapshot: Vec<(String, crate::card_source::CardHandle, PermanentHandle)> = {
            let area = &self.player(player).battle_area;
            area.iter()
                .enumerate()
                .map(|(i, p)| {
                    let top = p.top_card();
                    (
                        top.card_id(&self.card_data).to_string(),
                        top.handle(),
                        PermanentHandle {
                            player,
                            index: i as u8,
                        },
                    )
                })
                .collect()
        };

        for (card_id, card_handle, perm_handle) in snapshot {
            let Some(effect_impl) = self.effect_registry.get(&card_id) else {
                continue;
            };
            let effects = effect_impl.effects(card_handle);
            for effect in &effects {
                if effect.timing != crate::enums::EffectTiming::EndOfYourTurn {
                    continue;
                }
                if let Some(cond) = &effect.condition {
                    let ctx =
                        EffectContext::new(self, card_handle, Some(perm_handle), player);
                    if !cond(&ctx) {
                        continue;
                    }
                }
                if let Some(process) = &effect.process {
                    let mut ctx =
                        EffectContext::new(self, card_handle, Some(perm_handle), player);
                    process(&mut ctx);
                }
            }
        }
    }

    /// Fire OnPlay effects for the permanent at `(player, field_index)`.
    /// Called by play_from_hand; can also be called directly by tests.
    pub fn fire_on_play(&mut self, player_id: PlayerId, field_index: usize) {
        // Read card identity (immutable borrow) before getting an effect impl.
        let (card_id, handle) = {
            let perm = match self.players[player_id as usize].battle_area.get(field_index) {
                Some(p) => p,
                None => return,
            };
            let top = perm.top_card();
            let card_id = top.card_id(&self.card_data).to_string();
            (card_id, top.handle())
        };

        // Pull the Arc out of the registry (clones the Arc, releases borrow).
        let effect_impl = match self.effect_registry.get(&card_id) {
            Some(arc) => arc,
            None => return,
        };

        let effects = effect_impl.effects(handle);
        let perm_handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };

        for effect in &effects {
            if !effect.on_play {
                continue;
            }
            // Check condition (if any).
            if let Some(cond) = &effect.condition {
                let ctx = EffectContext::new(self, handle, Some(perm_handle), player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            // Run process (if any).
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, Some(perm_handle), player_id);
                process(&mut ctx);
            }
        }
    }

    /// Digivolve: push a card onto a permanent's stack.
    pub fn digivolve_onto(
        &mut self,
        player_id: PlayerId,
        field_index: usize,
        card: CardSource,
    ) -> bool {
        let turn = self.turn_count;
        let player = self.player_mut(player_id);
        if field_index >= player.battle_area.len() {
            return false;
        }
        player.battle_area[field_index].digivolve(card, turn);
        true
    }
}
