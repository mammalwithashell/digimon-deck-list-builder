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
#[cfg(feature = "dsl-yaml-loader")]
use std::sync::{Arc, OnceLock};

use crate::card_data::{
    CardData, DnaCost, DnaRequirement, DualCardData, DualDigimonFace, DualOptionFace,
};
use crate::card_source::CardSource;
use crate::cards::{build_registry, CardEffectRegistry};
use crate::dsl_cards::formula_registry::FormulaExtensionRegistry;
use crate::enums::{CardColor, CardKind, GamePhase, Keyword, ModifierType, PlayerId};
use crate::events::GameEvent;
use crate::game::Game;
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
use crate::rules::Rules;
use crate::selection::{PendingSelection, PendingSelectionView, SelectionError, SelectionKind};

#[cfg(feature = "dsl-yaml-loader")]
use crate::dsl_cards::DslCardEffect;
#[cfg(feature = "dsl-yaml-loader")]
use digimon_dsl::compiled::{CompiledCard, CompiledCardKind, CompiledClause, CompiledColor};

/// A scripted game runner for behavioral tests.
pub struct DebugRunner {
    pub game: Game,
    #[cfg(feature = "dsl-yaml-loader")]
    compiled_cards: HashMap<String, Arc<CompiledCard>>,
}

impl DebugRunner {
    pub fn builder() -> DebugRunnerBuilder {
        DebugRunnerBuilder::default()
    }

    /// Wrap an existing `Game` in a DebugRunner. Useful for tests that
    /// construct the game via `Game::new` directly (e.g. mulligan tests that
    /// want real deck draws) and still want DebugRunner's convenience API.
    pub fn wrap(game: Game) -> Self {
        Self {
            game,
            #[cfg(feature = "dsl-yaml-loader")]
            compiled_cards: HashMap::new(),
        }
    }

    // ─── Mulligan helpers ─────────────────────────────────────────────

    /// Who is expected to make the next mulligan decision, or `None` if done.
    pub fn mulligan_current(&self) -> Option<PlayerId> {
        self.game.mulligan_current_player()
    }

    /// Apply a mulligan decision for the currently-deciding player.
    /// `keep = true` keeps the hand; `keep = false` redraws.
    pub fn mulligan_decide(&mut self, keep: bool) -> Result<(), &'static str> {
        let p = self
            .game
            .mulligan_current_player()
            .ok_or("mulligan is already complete")?;
        self.game.accept_mulligan(p, keep)
    }

    /// Auto-keep for every remaining mulligan-pending player. Equivalent to
    /// calling `start_game` but without the `turn_count == 0` defensive branch.
    pub fn skip_mulligan(&mut self) {
        while let Some(p) = self.game.mulligan_current_player() {
            let _ = self.game.accept_mulligan(p, true);
        }
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

    /// Attack a Digimon. `vortex=false` for the vast majority of tests;
    /// pass `true` to simulate an end-of-turn <Vortex> attack.
    pub fn attack_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
        vortex: bool,
    ) -> crate::combat::AttackResult {
        self.game.attack_digimon(attacker, defender, vortex)
    }

    /// Attack the opposing player's security. See [`Self::attack_digimon`]
    /// for the `vortex` flag.
    pub fn attack_player(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
        vortex: bool,
    ) -> crate::combat::AttackResult {
        self.game.attack_player(attacker, defender_player, vortex)
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

    /// Mutable access to the underlying `Game` — for tests that drive new
    /// APIs before the higher-level `DebugRunner` helpers exist.
    pub fn game_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    /// Install a `CardEffect` into the registry under a card id. Tests can
    /// declare one-off effects inline without a frozen `cards/` entry.
    pub fn register_effect(
        &mut self,
        card_id: &str,
        effect: std::sync::Arc<dyn crate::effect::CardEffect>,
    ) {
        self.game.effect_registry.insert(card_id, effect);
    }

    pub fn set_formula_extensions(&mut self, registry: FormulaExtensionRegistry) {
        self.game.formula_extensions = registry;
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

    // ─── DSL inspection helpers ─────────────────────────────────────

    #[cfg(feature = "dsl-yaml-loader")]
    pub fn compiled_card(&self, card_id: &str) -> Option<&CompiledCard> {
        self.compiled_cards.get(card_id).map(Arc::as_ref)
    }

    #[cfg(feature = "dsl-yaml-loader")]
    pub fn dsl_clause(&self, card_id: &str, idx: usize) -> Option<&CompiledClause> {
        self.compiled_card(card_id)?.effects.get(idx)
    }

    // ─── Selection/action helpers ───────────────────────────────────

    pub fn pending_selection(&self) -> Option<&PendingSelection> {
        self.game.pending_selection.as_ref()
    }

    pub fn pending_selection_view(&self) -> Option<PendingSelectionView> {
        self.pending_selection().map(PendingSelection::view)
    }

    pub fn pending_kind(&self) -> Option<SelectionKind> {
        self.pending_selection().map(|s| s.kind)
    }

    pub fn pending_is_optional(&self) -> bool {
        self.pending_selection()
            .map(|s| s.is_optional)
            .unwrap_or(false)
    }

    pub fn pending_action_count(&self) -> usize {
        self.pending_selection()
            .map(|s| s.valid_action_ids.len())
            .unwrap_or(0)
    }

    pub fn execute_action(
        &mut self,
        player: PlayerId,
        action_id: u16,
    ) -> Result<(), SelectionError> {
        self.game.resolve_selection(player, action_id)
    }

    pub fn execute_branch(&mut self, branch_index: usize) -> Result<(), SelectionError> {
        let (player, action_id) = {
            let sel = self
                .pending_selection()
                .ok_or(SelectionError::NoPendingSelection)?;
            let choices = sel
                .effect_choices
                .as_ref()
                .ok_or(SelectionError::InvalidAction)?;
            let choice = choices
                .get(branch_index)
                .ok_or(SelectionError::InvalidAction)?;
            (sel.selecting_player, choice.action_id)
        };
        self.execute_action(player, action_id)
    }

    pub fn auto_resolve(&mut self) -> Result<(), SelectionError> {
        while let Some(sel) = self.pending_selection() {
            let player = sel.selecting_player;
            let action_id = match sel.valid_action_ids.first().copied() {
                Some(id) => id,
                None if sel.is_optional => crate::action::space::PASS,
                None => return Err(SelectionError::InvalidAction),
            };
            self.execute_action(player, action_id)?;
        }
        Ok(())
    }

    // ─── Event helpers ──────────────────────────────────────────────

    pub fn event_checkpoint(&self) -> usize {
        self.game.events().len()
    }

    pub fn events_since(&self, checkpoint: usize) -> &[GameEvent] {
        let start = checkpoint.min(self.game.events().len());
        &self.game.events()[start..]
    }

    pub fn events_of_kind<F>(&self, checkpoint: usize, predicate: F) -> Vec<&GameEvent>
    where
        F: Fn(&GameEvent) -> bool,
    {
        self.events_since(checkpoint)
            .iter()
            .filter(|event| predicate(event))
            .collect()
    }
}

/// Builder for DebugRunner.
pub struct DebugRunnerBuilder {
    card_data: HashMap<String, CardData>,
    #[cfg(feature = "dsl-yaml-loader")]
    compiled_cards: HashMap<String, Arc<CompiledCard>>,
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
    /// Initial memory value applied after `start()`. Useful for tests that
    /// want to play non-zero-cost cards without thinking about the seesaw.
    initial_memory: Option<i16>,
}

impl Default for DebugRunnerBuilder {
    fn default() -> Self {
        Self {
            card_data: HashMap::new(),
            #[cfg(feature = "dsl-yaml-loader")]
            compiled_cards: HashMap::new(),
            hands: HashMap::new(),
            decks: HashMap::new(),
            securities: HashMap::new(),
            digitamas: HashMap::new(),
            rules: Rules::standard(),
            registry: None,
            player_count: None,
            initial_memory: None,
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

    /// Load one DSL card from the embedded DSL pack and register its lowered
    /// effects into this runner.
    #[cfg(feature = "dsl-yaml-loader")]
    pub fn dsl_card(mut self, card_id: &str) -> Result<Self, String> {
        let compiled = embedded_dsl_registry()?
            .lookup(card_id)
            .ok_or_else(|| format!("DSL card {card_id} not found in embedded pack"))?
            .clone();
        self.register_compiled_card(compiled);
        Ok(self)
    }

    /// Compile one inline YAML DSL card fixture and register it into this
    /// runner. Useful for tests that need compact behavior-specific cards.
    #[cfg(feature = "dsl-yaml-loader")]
    pub fn from_dsl_yaml(mut self, yaml: &str) -> Result<Self, String> {
        let spec: digimon_dsl::CardSpec =
            serde_yml::from_str(yaml).map_err(|e| format!("parse DSL YAML: {e}"))?;
        let compiled = digimon_dsl::compile::compile(&spec).map_err(|errors| {
            errors
                .into_iter()
                .map(|e| format!("{} {}: {}", e.card_id, e.path, e.message))
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        self.register_compiled_card(compiled);
        Ok(self)
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

    /// Pre-fund the memory seesaw after `start()`. Without this, games begin
    /// at memory 0 and most non-zero-cost plays would be unaffordable.
    pub fn memory(mut self, value: i16) -> Self {
        self.initial_memory = Some(value);
        self
    }

    /// Build the runner and start the game (advances past Mulligan into turn 1).
    pub fn start(mut self) -> DebugRunner {
        let initial_memory = self.initial_memory;
        let mut r = self.build_inner();
        r.game.start_game();
        if let Some(m) = initial_memory {
            r.game.set_memory(m);
        }
        r
    }

    /// Build the runner without starting the game (stays in Mulligan).
    pub fn build(mut self) -> DebugRunner {
        self.build_inner()
    }

    fn build_inner(&mut self) -> DebugRunner {
        let player_count = self.player_count.unwrap_or(self.rules.player_count);

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

        // DebugRunner populates zones manually after this point. Clear the
        // mulligan-pending list so that a subsequent `start_game()` doesn't
        // walk through `finalize_mulligan` and steal cards from the explicit
        // deck/security the test just set up. DebugRunner represents a
        // post-setup snapshot; mulligan is "already done" from its perspective.
        game.mulligan_pending.clear();

        // Populate each player's zones from the builder spec.
        let mut next_card_index: u16 = 0;
        for player_idx in 0..player_count {
            let pid = player_idx as PlayerId;

            if let Some(ids) = self.hands.get(&pid) {
                for card_id in ids {
                    let card = Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    game.players[pid as usize].hand.push(card);
                }
            }
            if let Some(ids) = self.decks.get(&pid) {
                for card_id in ids {
                    let card = Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    game.players[pid as usize].deck.push(card);
                }
            }
            if let Some(ids) = self.securities.get(&pid) {
                for card_id in ids {
                    let card = Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    game.players[pid as usize].security.push(card);
                }
            }
            if let Some(ids) = self.digitamas.get(&pid) {
                for card_id in ids {
                    let card = Self::make_card(&data_index_map, card_id, pid, &mut next_card_index);
                    if card_data_store[card.data_index].card_kind != CardKind::DigiEgg {
                        // Allow non-eggs in digitama for tests, but warn via debug.
                    }
                    game.players[pid as usize].digitama_deck.push(card);
                }
            }
        }

        // Persist the advanced counter so that any post-build allocation
        // (tokens, orphan handles, etc.) via `game.next_card_index()` returns
        // fresh indices that don't alias the builder-seeded cards.
        game.advance_card_index_to(next_card_index);

        // Replace the registry if the user supplied one.
        if let Some(reg) = self.registry.take() {
            game.effect_registry = reg;
        } else {
            // Default registry already populated by Game::new -> build_registry().
            // Make sure test cards are registered (build_registry() does this).
            let _ = build_registry; // silence unused-import if path changes
        }

        DebugRunner {
            game,
            #[cfg(feature = "dsl-yaml-loader")]
            compiled_cards: std::mem::take(&mut self.compiled_cards),
        }
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

    #[cfg(feature = "dsl-yaml-loader")]
    fn register_compiled_card(&mut self, compiled: CompiledCard) {
        use crate::cards::raw_rust;
        let card_id = compiled.card.clone();
        let compiled = Arc::new(compiled);
        self.card_data
            .entry(card_id.clone())
            .or_insert_with(|| card_data_from_compiled(compiled.as_ref()));
        self.compiled_cards
            .insert(card_id.clone(), compiled.clone());
        let registry = self.registry.get_or_insert_with(build_registry);
        let raw = Arc::new(raw_rust::build_registry());
        registry.insert(
            &card_id,
            Arc::new(DslCardEffect::new_with_raw(compiled, raw)),
        );
    }
}

#[cfg(feature = "dsl-yaml-loader")]
fn embedded_dsl_registry() -> Result<&'static digimon_dsl::CardRegistry, String> {
    static REGISTRY: OnceLock<Result<digimon_dsl::CardRegistry, String>> = OnceLock::new();
    REGISTRY
        .get_or_init(crate::dsl_registry::from_embedded)
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(feature = "dsl-yaml-loader")]
fn card_data_from_compiled(card: &CompiledCard) -> CardData {
    use crate::dsl_cards::modifier_map::lookup_keyword;
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};

    // Populate native keywords from face-up `grant_keyword` declarative clauses.
    // A `GrantKeyword` clause is semantically equivalent to a printed keyword
    // on the card face, so `has_keyword` can detect it via the native path
    // without needing the declarative process to run at placement. Inherited
    // grants are stack text and must stay out of native top-card keywords.
    let mut keywords: Vec<Keyword> = Vec::new();
    for clause in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            value,
            scope,
            ..
        }) = clause
        {
            if matches!(scope, CompiledScope::Inherited) {
                continue;
            }
            if let Some(kw) = lookup_keyword(keyword, *value) {
                if !keywords.contains(&kw) {
                    keywords.push(kw);
                }
            }
        }
    }

    CardData {
        card_id: card.card.clone(),
        card_name: card.name.clone(),
        card_kind: match card.kind {
            CompiledCardKind::Digimon => CardKind::Digimon,
            CompiledCardKind::Tamer => CardKind::Tamer,
            CompiledCardKind::Option => CardKind::Option,
            CompiledCardKind::DigiEgg => CardKind::DigiEgg,
            CompiledCardKind::Token => CardKind::Token,
            CompiledCardKind::Dual => CardKind::Dual,
        },
        level: card.level,
        dp: card.dp,
        play_cost: card.cost.unwrap_or(0).max(0) as u16,
        colors: card
            .color
            .iter()
            .copied()
            .map(compiled_color_to_engine)
            .collect(),
        traits: card.traits.clone(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card.card.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        keywords,
        dual: card.dual.as_ref().map(compiled_dual_to_engine),
    }
}

#[cfg(feature = "dsl-yaml-loader")]
fn compiled_color_to_engine(color: CompiledColor) -> CardColor {
    match color {
        CompiledColor::Red => CardColor::Red,
        CompiledColor::Blue => CardColor::Blue,
        CompiledColor::Yellow => CardColor::Yellow,
        CompiledColor::Green => CardColor::Green,
        CompiledColor::Black => CardColor::Black,
        CompiledColor::Purple => CardColor::Purple,
        CompiledColor::White => CardColor::White,
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
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        dual: None,
    }
}

#[cfg(feature = "dsl-yaml-loader")]
fn compiled_dual_to_engine(dual: &digimon_dsl::compiled::CompiledDual) -> DualCardData {
    DualCardData {
        digimon: DualDigimonFace {
            level: dual.digimon.level,
            dp: dual.digimon.dp,
            colors: dual
                .digimon
                .colors
                .iter()
                .copied()
                .map(compiled_color_to_engine)
                .collect(),
            traits: dual.digimon.traits.clone(),
            evo_costs: Vec::new(),
            effect_text: dual.digimon.effect_text.clone(),
            inherited_text: dual.digimon.inherited_text.clone(),
            keywords: Vec::new(),
        },
        option: DualOptionFace {
            use_cost: dual.option.use_cost,
            colors: dual
                .option
                .colors
                .iter()
                .copied()
                .map(compiled_color_to_engine)
                .collect(),
            effect_text: dual.option.effect_text.clone(),
            security_text: dual.option.security_text.clone(),
            keywords: dual
                .option
                .keywords
                .iter()
                .filter_map(|kw| match kw.as_str() {
                    "ArtsDigivolve" | "Arts Digivolve" | "arts_digivolve" => {
                        Some(Keyword::ArtsDigivolve)
                    }
                    _ => None,
                })
                .collect(),
        },
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
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        dual: None,
    }
}

// ─── DNA-digivolve test helpers ───────────────────────────────────────

/// Build a `DnaRequirement` matching exactly one level. All other
/// constraint fields (colors, name_contains, text_contains) are empty.
pub fn dna_req_lv(level: u8) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: Vec::new(),
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

/// Build a minimal `CardData` with a non-default `level` set on top of
/// `make_test_card`'s defaults.
pub fn make_test_card_with_level(card_id: &str, card_name: &str, level: u8) -> CardData {
    let mut d = make_test_card(card_id, card_name);
    d.level = Some(level);
    d
}

/// Build a minimal Digimon `CardData` with a single `DnaCost` whose
/// `requirement1` and `requirement2` are level-only (`dna_req_lv`).
pub fn make_test_dna_card(
    card_id: &str,
    card_name: &str,
    req1_level: u8,
    req2_level: u8,
    memory_cost: i16,
) -> CardData {
    let mut d = make_test_card(card_id, card_name);
    d.dna_costs = vec![DnaCost {
        memory_cost,
        requirement1: dna_req_lv(req1_level),
        requirement2: dna_req_lv(req2_level),
    }];
    d
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
        // TEST-001 costs 3 (default `make_test_card`). Pre-fund memory so the
        // play is affordable, then verify the OnPlay adds 1.
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-001", "TestOne"))
            .hand(0, &["TEST-001"])
            .memory(5)
            .start();
        let m_before = r.memory(); // 5
        r.play(0, 0); // -3 for cost, +1 for OnPlay = net -2
        assert_eq!(r.memory(), m_before - 3 + 1);
        assert_eq!(r.battle_area_size(0), 1);
    }
}
