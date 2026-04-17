use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::cards::{build_registry, CardEffectRegistry};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{EffectTiming, GamePhase, Keyword, ModifierType, PlayerId, SkipDraw};
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
use crate::player::Player;
use crate::rules::Rules;
use crate::selection::{EffectQueue, PendingAttack, PendingSecurity, PendingSelection, SelectionError};

/// Reasons `Game::activate_overclock` can fail. Exposed so callers
/// (Tauri commands, tests, Python bindings) can distinguish between
/// phase-violation and state-violation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverclockError {
    /// Current phase is not `EndOfTurnAction`.
    WrongPhase,
    /// Another selection or attack is in flight.
    Busy,
    /// The indicated permanent does not have `<Overclock>` (either the
    /// keyword isn't granted, or the slot doesn't hold a Digimon).
    NotOverclock,
    /// No sacrificeable Digimon is available to pay the Overclock cost.
    NoSacrifice,
    /// `overclock_index` is out of range for the turn player's battle area.
    InvalidIndex,
}

impl std::fmt::Display for OverclockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongPhase => write!(f, "activate_overclock called outside EndOfTurnAction"),
            Self::Busy => write!(f, "activate_overclock called while a selection or attack is in flight"),
            Self::NotOverclock => write!(f, "permanent does not have <Overclock>"),
            Self::NoSacrifice => write!(f, "no sacrificeable Digimon available"),
            Self::InvalidIndex => write!(f, "overclock_index out of range"),
        }
    }
}

impl std::error::Error for OverclockError {}

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
    /// Players still owing a mulligan decision, in order. Empty once mulligan
    /// is finalized. Driven by `accept_mulligan`; see §1.6 in RUST_PYTHON_PARITY.
    pub mulligan_pending: Vec<PlayerId>,
    /// Whether each player has already re-drawn during mulligan. Indexed by
    /// `PlayerId`. Used by the action mask to suppress the re-draw bit once
    /// a player has used their single mulligan.
    pub mulligan_used: Vec<bool>,
    /// Cards currently revealed to all players (e.g. top-of-deck reveals,
    /// search pools). Rendered into the observation tensor at `OFF_REVEALED`
    /// and cleared on turn rotation. Populated by reveal-from-deck / search
    /// effects. Matches Python's `Game.revealed_cards`.
    pub revealed_cards: Vec<CardSource>,

    /// Parked player-choice prompt, if any. Set by `EffectContext::select_*`
    /// helpers and the effect-queue drainer; resolved by
    /// `Game::resolve_selection`. See `selection.rs` for the design.
    /// Always `None` until the selection subsystem lands (PR2/PR3).
    pub pending_selection: Option<PendingSelection>,
    /// Triggered effects waiting to resolve at the current timing window.
    /// Populated by `enqueue_triggered` and drained by `drain_effect_queue`.
    /// Empty until the drainer lands (PR2).
    pub effect_queue: EffectQueue,
    /// In-flight attack, if any. Installed by `begin_attack`, advanced by
    /// the combat state machine, cleared by `cleanup_attack`.
    /// Always `None` until the combat state machine lands (PR4).
    pub pending_attack: Option<PendingAttack>,
    /// Transient state for an in-progress security check. Set by
    /// `resolve_security_card` before firing `SecuritySkill` effects and
    /// cleared afterward. `EffectContext::play_from_security` inspects and
    /// mutates this slot to keep the revealed card from being trashed.
    pub pending_security: Option<PendingSecurity>,
    /// Safety rail matching Python's `_resolve_effect_stack` max-iterations=50
    /// cap. Incremented per drain step; reset to 0 when the queue empties.
    /// Prevents a self-triggering chain from hanging the engine.
    /// Consumed by the drainer in PR2.
    #[allow(dead_code)]
    pub(crate) effect_chain_depth: u16,
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

        // Initial turn order (before coin flip).
        let mut turn_order: Vec<PlayerId> = (0..rules.player_count).collect();
        // Coin flip: randomize which player goes first. For N-player we shuffle
        // the whole order so EDH-style modes pick a starting player uniformly.
        // Matches Python's `random.choice([True, False])` for 2-player, and
        // extends the concept cleanly to multiplayer.
        turn_order.shuffle(&mut rng);
        let memory_pair = if turn_order.len() >= 2 {
            (turn_order[0], turn_order[1])
        } else {
            (turn_order[0], turn_order[0])
        };

        let player_count = rules.player_count as usize;
        let mulligan_pending = turn_order.clone();
        let mulligan_used = vec![false; player_count];

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
            mulligan_pending,
            mulligan_used,
            revealed_cards: Vec::new(),
            pending_selection: None,
            effect_queue: EffectQueue::new(),
            pending_attack: None,
            pending_security: None,
            effect_chain_depth: 0,
        };

        // Deal starting hands. Security is deliberately NOT laid here — it
        // waits until mulligan finalizes, so a player who mulligans has the
        // full deck to re-shuffle into (matches Python setup order).
        for i in 0..player_count {
            game.players[i].draw_many(game.rules.starting_hand);
        }

        Ok(game)
    }

    /// Get the current turn player's ID.
    pub fn turn_player(&self) -> PlayerId {
        self.turn_order[self.turn_player_idx]
    }

    // ─── Mulligan ────────────────────────────────────────────────────

    /// The next player expected to make a mulligan decision, or `None` if
    /// mulligan is already complete.
    pub fn mulligan_current_player(&self) -> Option<PlayerId> {
        self.mulligan_pending.first().copied()
    }

    /// Record a mulligan decision for the current deciding player.
    ///
    /// - `keep = true` — keep the drawn hand as-is.
    /// - `keep = false` — shuffle the hand back into the deck, reshuffle,
    ///   draw a fresh `starting_hand`. `mulligan_used[player]` is set so the
    ///   action mask can suppress a second redraw.
    ///
    /// Returns `Err` if it's not this player's turn to decide or if mulligan
    /// is already complete.
    pub fn accept_mulligan(
        &mut self,
        player: PlayerId,
        keep: bool,
    ) -> Result<(), &'static str> {
        let Some(current) = self.mulligan_current_player() else {
            return Err("mulligan is already complete");
        };
        if current != player {
            return Err("it is a different player's turn to decide");
        }

        if !keep {
            self.redraw_hand(player);
            self.mulligan_used[player as usize] = true;
        }
        self.mulligan_pending.remove(0);

        if self.mulligan_pending.is_empty() {
            self.finalize_mulligan();
        }
        Ok(())
    }

    /// Shuffle the player's hand back into the deck and redraw `starting_hand`.
    fn redraw_hand(&mut self, player: PlayerId) {
        let starting_hand = self.rules.starting_hand;
        let p = self.player_mut(player);
        p.deck.extend(p.hand.drain(..));
        // Borrow the game's rng via a local reshuffle: move the cards into a
        // local vec, shuffle with game rng, put back.
        let mut deck = std::mem::take(&mut p.deck);
        deck.shuffle(&mut self.rng);
        self.player_mut(player).deck = deck;
        self.player_mut(player).draw_many(starting_hand);
    }

    /// Finalize mulligan: lay security for every player and begin turn 1.
    fn finalize_mulligan(&mut self) {
        let security_count = self.rules.security_count;
        for i in 0..self.rules.player_count as usize {
            self.players[i].setup_security(security_count);
        }
        self.turn_count = 1;
        self.memory = 0;
        self.begin_turn();
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

    // ─── Pending selection resolution ───────────────────────────────
    //
    // Wired up in PR3. For now this is a stub so callers can reference the
    // name without crashing — every invocation is a logic bug until the
    // selection subsystem lands. The error path is intentionally distinct
    // from `NoPendingSelection` so tests can assert "we haven't implemented
    // this yet" rather than "nothing to resolve".

    /// Resolve a pending selection with `action_id` submitted by `player`.
    ///
    /// Dispatches to `resolve_generic_selection` in `effect_queue.rs`,
    /// which validates, restores the pre-selection phase, invokes the
    /// callback (or `on_decline` for PASS on an optional prompt), and
    /// resumes the effect-queue drainer.
    ///
    /// Works uniformly for every `SelectionKind`: TriggerOrder, OppField,
    /// Hand, Trash, EffectChoice, etc. The callback stored on the
    /// selection does kind-specific decoding.
    pub fn resolve_selection(
        &mut self,
        player: PlayerId,
        action_id: u16,
    ) -> Result<(), SelectionError> {
        if self.pending_selection.is_none() {
            return Err(SelectionError::NoPendingSelection);
        }
        self.resolve_generic_selection(player, action_id)
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

    /// Start the game: auto-keep for every remaining mulligan-pending player
    /// and transition into turn 1. UIs / RL agents that want to make mulligan
    /// decisions explicitly should call `accept_mulligan` for each decider
    /// before invoking `start_game` (or instead of it — the last
    /// `accept_mulligan` call triggers `finalize_mulligan`, which begins turn 1).
    pub fn start_game(&mut self) {
        while let Some(p) = self.mulligan_current_player() {
            // Auto-keep; infallible because we just asked who's current.
            let _ = self.accept_mulligan(p, true);
        }
        // If the game was never in Mulligan phase (defensive), fall through
        // to an explicit turn-1 transition.
        if self.turn_count == 0 {
            self.turn_count = 1;
            self.memory = 0;
            self.begin_turn();
        }
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
            // "The first player of the game skips their first draw." Turn 1
            // uniquely identifies that moment — whoever the coin flip sat at
            // `turn_order[0]` is the turn player here. Don't hardcode tp == 0.
            SkipDraw::FirstPlayerOnly => self.turn_count == 1,
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
    /// Fires OnEndTurn effects, checks memory swing-back (§1.5): if an OnEndTurn
    /// effect restored memory from negative to non-negative, the turn continues
    /// and returns to Main phase instead of switching.
    ///
    /// If the ending player has a permanent with a pending end-of-turn action
    /// (§4.6b: Vortex / Overclock / MayAttack), the phase parks in
    /// `GamePhase::EndOfTurnAction` and the caller resumes by picking an attack
    /// bit or calling `pass_end_of_turn_action`. Matches Python
    /// `_complete_end_phase`.
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

        // §4.6b: park in EndOfTurnAction if the player has a pending
        // end-of-turn-keyword action. Turn rotation is deferred until the
        // player resumes via `pass_end_of_turn_action`. ForceAttack is not
        // checked here (Python doesn't either): it's enforced at the
        // Main-phase mask (§4.7d) before the turn reaches `end_turn`.
        if self.has_end_of_turn_keywords(ending_player) {
            self.current_phase = GamePhase::EndOfTurnAction;
            return;
        }

        self.rotate_turn_player(ending_player);
    }

    /// Advance the turn rotation — expires end-of-turn modifiers, flips the
    /// memory seesaw, and calls `begin_turn` for the new active player.
    /// Extracted from `end_turn` so `pass_end_of_turn_action` can resume
    /// rotation without re-running the EOT-keyword check.
    fn rotate_turn_player(&mut self, ending_player: PlayerId) {
        // Reveal pool is transient — clear on turn rotation so tensor reveals
        // don't leak across turns. Matches Python's clear in `switch_turn`.
        self.revealed_cards.clear();

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

    /// Resume turn rotation from the `EndOfTurnAction` phase. Called when the
    /// player declines further end-of-turn actions (PASS bit 62 while phase ==
    /// `EndOfTurnAction`) or the runner has exhausted all reachable EOT
    /// attacks. No-op if called outside the EOT-action phase.
    ///
    /// Mirrors Python's `next_phase` branch at [game/__init__.py:242-245].
    pub fn pass_end_of_turn_action(&mut self) {
        if self.current_phase != GamePhase::EndOfTurnAction {
            return;
        }
        let ending_player = self.turn_player();
        self.rotate_turn_player(ending_player);
    }

    /// True iff the given player has any permanent with a pending end-of-turn
    /// keyword action (Vortex attack, Overclock sacrifice-and-attack, or
    /// MayAttack). Mirrors Python `_has_end_of_turn_keywords`.
    pub fn has_end_of_turn_keywords(&self, player: PlayerId) -> bool {
        let Some(me) = self.players.get(player as usize) else {
            return false;
        };
        for (i, perm) in me.battle_area.iter().enumerate() {
            if !perm.top_card().is_digimon(&self.card_data) {
                continue;
            }
            let handle = PermanentHandle {
                player,
                index: i as u8,
            };
            // Vortex — matches Python `perm.can_attack(is_vortex=True)`.
            if self.modifiers.has_keyword(handle, Keyword::Vortex)
                && self.can_attack(handle, /* vortex = */ true)
            {
                return true;
            }
            // Overclock — needs at least one other sacrificeable permanent.
            if self.modifiers.has_keyword(handle, Keyword::Overclock)
                && self.has_overclock_sacrifice(player, i)
            {
                return true;
            }
            // MayAttack — normal can_attack (not vortex-exempt).
            if self.modifiers.has(handle, ModifierType::MayAttack)
                && self.can_attack(handle, /* vortex = */ false)
            {
                return true;
            }
        }
        false
    }

    /// True iff `player`'s battle area contains at least one Digimon other
    /// than the Overclock Digimon at `overclock_index` — i.e. a valid
    /// Overclock sacrifice is available.
    ///
    /// Python checks `p.is_token or p.is_digimon`; Rust's `CardKind` has no
    /// `Token` variant (tokens are registered as Digimon via
    /// `token_registry`), so the check collapses to `is_digimon`. See plan
    /// note on Token detection.
    pub fn has_overclock_sacrifice(&self, player: PlayerId, overclock_index: usize) -> bool {
        let Some(me) = self.players.get(player as usize) else {
            return false;
        };
        me.battle_area.iter().enumerate().any(|(i, p)| {
            i != overclock_index && p.top_card().is_digimon(&self.card_data)
        })
    }

    /// Activate `<Overclock>` on the turn player's battle-area permanent at
    /// `overclock_index`. Installs a `PendingSelection` over the other
    /// sacrificeable battle-area Digimon; resolving it deletes the sacrifice
    /// and fires an end-of-turn attack on the opponent player that does NOT
    /// suspend the attacker. Declining (PASS) is legal and returns to
    /// `EndOfTurnAction` with no side effects — the Overclock bit remains
    /// available on the next mask build.
    ///
    /// Target is always the opposing player (security check). Mirrors Python
    /// `action_decoder._initiate_overclock` /
    /// [action_decoder.py:501-522](../digimon_gym/engine/game/action_decoder.py#L501).
    ///
    /// § Parity: §4.6c-residual.
    pub fn activate_overclock(
        &mut self,
        overclock_index: usize,
    ) -> Result<(), OverclockError> {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};

        if self.current_phase != GamePhase::EndOfTurnAction {
            return Err(OverclockError::WrongPhase);
        }
        if self.pending_selection.is_some() || self.pending_attack.is_some() {
            return Err(OverclockError::Busy);
        }

        let player = self.turn_player();
        let overclock_handle = PermanentHandle {
            player,
            index: overclock_index as u8,
        };

        let me = self
            .players
            .get(player as usize)
            .ok_or(OverclockError::InvalidIndex)?;
        let overclock_perm = me
            .battle_area
            .get(overclock_index)
            .ok_or(OverclockError::InvalidIndex)?;

        if !self.modifiers.has_keyword(overclock_handle, Keyword::Overclock) {
            return Err(OverclockError::NotOverclock);
        }
        if !overclock_perm.top_card().is_digimon(&self.card_data) {
            return Err(OverclockError::NotOverclock);
        }
        if !self.has_overclock_sacrifice(player, overclock_index) {
            return Err(OverclockError::NoSacrifice);
        }

        // Build the OwnField selection over sacrificeable Digimon. Encoding
        // uses the ATTACK target-half range — same convention the existing
        // `install_field_selection` helper uses for OwnField/OppField selects.
        let mut valid_action_ids: Vec<u16> = Vec::new();
        for (i, perm) in me.battle_area.iter().enumerate() {
            if i == overclock_index {
                continue;
            }
            if perm.top_card().is_digimon(&self.card_data) {
                valid_action_ids.push(encode_attack(0, i as u16));
            }
        }
        debug_assert!(!valid_action_ids.is_empty(), "has_overclock_sacrifice promised ≥1");

        let source_card = overclock_perm.top_card().handle();
        let opponent = self.next_clockwise(player);
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;

        self.pending_selection = Some(PendingSelection {
            kind: crate::selection::SelectionKind::OwnField,
            selecting_player: player,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: "Choose a Digimon to sacrifice for <Overclock>".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(overclock_handle),
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let sacrifice_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let sacrifice_handle = PermanentHandle {
                    player,
                    index: sacrifice_index,
                };

                // Delete the sacrifice (firing OnDeletion triggers).
                game.delete_permanent_with_effects(sacrifice_handle);

                // OnDeletion may have removed the Overclock Digimon, shifted
                // indices, or killed the game. Bail if the Overclock Digimon
                // is no longer present at its original index.
                let attacker_alive = game
                    .players
                    .get(player as usize)
                    .and_then(|p| p.battle_area.get(overclock_index as usize))
                    .map(|p| p.top_card().card_index == source_card.0)
                    .unwrap_or(false);
                if !attacker_alive || game.game_over {
                    return;
                }

                // Fire the attack on the opponent player without suspending.
                game.begin_attack_overclock(
                    overclock_handle,
                    crate::selection::AttackTarget::Player(opponent),
                );
            }),
            on_decline: None,
        });

        Ok(())
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
    ///
    /// Thin wrapper over the effect-queue drainer — collects every matching
    /// effect across the battle area into `effect_queue`, then drains.
    pub fn fire_end_of_your_turn(&mut self, player: PlayerId) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::EndOfYourTurn,
            crate::selection::TriggerSource::PlayerBattleArea(player),
        );
        self.drain_effect_queue();
    }

    /// Fire OnPlay effects for the permanent at `(player, field_index)`.
    /// Called by play_from_hand; can also be called directly by tests.
    ///
    /// Thin wrapper over the effect-queue drainer. Single-trigger cases fire
    /// in one step exactly like the old atomic loop; multi-trigger cases
    /// park on a `TriggerOrder` selection for the controller to order.
    pub fn fire_on_play(&mut self, player_id: PlayerId, field_index: usize) {
        if field_index >= self.players[player_id as usize].battle_area.len() {
            return;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnPlay,
            crate::selection::TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s hand slot
    /// `hand_index`. Returns `true` if a matching effect fired, `false` if no
    /// `EffectTiming::MainFromHand` effect on the card was legal.
    ///
    /// Consumes `HAND_EFFECT` action bits (30-59) that the mask emits. Memory
    /// cost, card movement, and any side effects are handled inside the
    /// effect's `process` closure — mirroring Python's
    /// `_execute_hand_main_effect`. First-match-wins: once an effect fires we
    /// stop iterating, matching the mask's own first-match-wins emission.
    ///
    /// Hand/Trash per-turn activation counters (§4.5c-residual 🟡) are not
    /// tracked here; see docs/RUST_PYTHON_PARITY.md §4.5c.
    pub fn activate_hand_main(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        let (card_id, handle) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.hand.get(hand_index) {
                Some(c) => c,
                None => return false,
            };
            (card.card_id(&self.card_data).to_string(), card.handle())
        };

        let effect_impl = match self.effect_registry.get(&card_id) {
            Some(arc) => arc,
            None => return false,
        };
        let effects = effect_impl.effects(handle);

        for effect in &effects {
            if effect.timing != EffectTiming::MainFromHand {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(self, handle, None, player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, None, player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
    }

    /// Activate a `[Main]` effect on the permanent at `player_id`'s battle-area
    /// slot `field_index`. Returns `true` if a matching effect fired.
    ///
    /// Consumes `FIELD_EFFECT` bits at sub-slot `FIELD_EFFECT_SLOT_FOR_MAIN`
    /// (per-permanent base + 2). Walks the digivolution stack bottom-up,
    /// applying the inherited-vs-top filter used by
    /// [`Game::source_dp_contribution`] so a given Field [Main] effect only
    /// fires on the same source/position the mask emitted from. Honors OPT via
    /// [`Permanent::activation_count`] and records activation on success so a
    /// subsequent mask rebuild sees the bit suppressed.
    ///
    /// Mirrors Python's `_execute_field_main_effect`.
    pub fn activate_field_main(&mut self, player_id: PlayerId, field_index: usize) -> bool {
        // Snapshot per-source identity without holding the battle_area borrow
        // across the effect closure invocations (which need `&mut self`).
        let (perm_handle, sources) = {
            let Some(player) = self.players.get(player_id as usize) else {
                return false;
            };
            let Some(perm) = player.battle_area.get(field_index) else {
                return false;
            };
            let stack_size = perm.card_sources.len();
            let handle = PermanentHandle {
                player: player_id,
                index: field_index as u8,
            };
            let mut infos: Vec<(bool, String, crate::card_source::CardHandle)> =
                Vec::with_capacity(stack_size);
            for (i, source) in perm.card_sources.iter().enumerate() {
                let is_under = i + 1 < stack_size;
                infos.push((
                    is_under,
                    source.card_id(&self.card_data).to_string(),
                    source.handle(),
                ));
            }
            (handle, infos)
        };

        for (is_under, card_id, source_handle) in sources {
            let Some(effect_impl) = self.effect_registry.get(&card_id) else {
                continue;
            };
            let effects = effect_impl.effects(source_handle);
            for (slot, effect) in effects.iter().enumerate() {
                if effect.timing != EffectTiming::MainOnField {
                    continue;
                }
                if is_under != effect.inherited {
                    continue;
                }
                if effect.max_per_turn > 0 {
                    let perm = &self.players[player_id as usize].battle_area[field_index];
                    if perm.activation_count(source_handle, slot as u8) >= effect.max_per_turn {
                        continue;
                    }
                }
                if let Some(cond) = &effect.condition {
                    let ctx =
                        EffectReadContext::new(self, source_handle, Some(perm_handle), player_id);
                    if !cond(&ctx) {
                        continue;
                    }
                }
                // Python records activation before invoking the callback so a
                // panic inside the process still counts toward OPT. Mirror that.
                if let Some(perm) = self.players[player_id as usize]
                    .battle_area
                    .get_mut(field_index)
                {
                    perm.record_activation(source_handle, slot as u8);
                }
                if let Some(process) = &effect.process {
                    let mut ctx = EffectContext::new(
                        self,
                        source_handle,
                        Some(perm_handle),
                        player_id,
                    );
                    process(&mut ctx);
                }
                return true;
            }
        }
        false
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s trash slot
    /// `trash_index`. Returns `true` if a matching effect fired.
    ///
    /// Consumes `TRASH_EFFECT` action bits (1150-1194). Mirrors Python's
    /// `_execute_trash_main_effect`: memory cost and any card movement happen
    /// inside the effect's process closure, and there is no per-turn
    /// activation counter (§4.5c-residual 🟡).
    pub fn activate_trash_main(&mut self, player_id: PlayerId, trash_index: usize) -> bool {
        let (card_id, handle) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.trash.get(trash_index) {
                Some(c) => c,
                None => return false,
            };
            (card.card_id(&self.card_data).to_string(), card.handle())
        };

        let effect_impl = match self.effect_registry.get(&card_id) {
            Some(arc) => arc,
            None => return false,
        };
        let effects = effect_impl.effects(handle);

        for effect in &effects {
            if effect.timing != EffectTiming::MainFromTrash {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(self, handle, None, player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, None, player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
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

    /// Returns `true` when `card` may digivolve onto `perm` per standard
    /// evo-cost rules: `card` has an `EvoCost` entry whose `level` matches
    /// `perm.top_card()`'s level and whose color is present on
    /// `perm.top_card()`'s color list.
    ///
    /// Memory cost is **not** checked — blast digivolve bypasses memory,
    /// and regular digivolve pays memory at the call site. Mirrors
    /// Python's `can_digivolve(card, base_perm)` validator. Used by
    /// `combat::try_enter_counter` for §2.3 parity.
    pub fn can_digivolve(
        &self,
        card: &CardSource,
        perm: &crate::permanent::Permanent,
    ) -> bool {
        let Some(base_level) = perm.top_card().level(&self.card_data) else {
            return false;
        };
        let base_colors = perm.top_card().colors(&self.card_data);
        let evo_costs = &self.card_data[card.data_index].evo_costs;
        evo_costs.iter().any(|ec| {
            ec.level == base_level
                && crate::action::mask::evo_color(ec.card_color)
                    .map(|c| base_colors.contains(&c))
                    .unwrap_or(false)
        })
    }

    // ─── Effect-listing API (§4.5c) ──────────────────────────────────

    /// Enumerate a card's effects by asking the registry for its impl.
    /// Returns `None` when no impl is registered so hot-path callers (the
    /// mask builder) can skip the match-iterate loop entirely instead of
    /// walking an empty `Vec`.
    ///
    /// Analogous to Python's `CardSource.effect_list(timing)` but expressed
    /// Rust-idiomatically: the registry is owned by `Game`, so this is the
    /// single entry point callers use regardless of whether the card lives
    /// in hand, trash, or a `card_sources` slot. Callers filter the returned
    /// vec by `effect.timing` (e.g. `MainFromHand`).
    ///
    /// The inner `Vec` allocation is driven by `CardEffect::effects(handle)`
    /// re-boxing per-instance closures and is unavoidable with the current
    /// trait shape. The helper does not add an extra empty-case allocation.
    pub fn effects_for_card(
        &self,
        card_id: &str,
        handle: crate::card_source::CardHandle,
    ) -> Option<Vec<crate::effect::Effect>> {
        self.effect_registry
            .get(card_id)
            .map(|impl_| impl_.effects(handle))
    }

    // ─── Tensor support: per-source DP + OPT helpers (§3.1 / §3.2) ───

    /// Sum of static `dp_modifier` values from a single source's effects
    /// that pass the inherited/top filter and their condition (if any).
    /// Returns a signed raw DP delta. Tensor writes this divided by DP_NORM.
    pub fn source_dp_contribution(
        &self,
        perm: PermanentHandle,
        source_index: usize,
    ) -> i32 {
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0;
        };
        let stack_size = permanent.card_sources.len();
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0;
        };
        let is_under = source_index + 1 < stack_size;
        let card_id = source.card_id(&self.card_data).to_string();
        let Some(impl_) = self.effect_registry.get(&card_id) else {
            return 0;
        };
        let effects = impl_.effects(source.handle());

        let mut total = 0i32;
        for effect in &effects {
            if effect.dp_modifier == 0 {
                continue;
            }
            if is_under != effect.inherited {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(
                    self,
                    source.handle(),
                    Some(perm),
                    perm.player,
                );
                if !cond(&ctx) {
                    continue;
                }
            }
            total += effect.dp_modifier;
        }
        total
    }

    /// OPT effects on a permanent, counted across its entire digivolution
    /// stack with the same inherited/top filter as `source_dp_contribution`.
    /// Linked card effects are not iterated (residual gap §3.1b).
    pub fn opt_total(&self, perm: PermanentHandle) -> u32 {
        self.opt_counts(perm).0
    }

    /// Number of OPT effects whose activation count this turn has reached
    /// their `max_per_turn` cap.
    pub fn opt_used(&self, perm: PermanentHandle) -> u32 {
        self.opt_counts(perm).1
    }

    /// Per-source OPT availability fraction in `[0.0, 1.0]`. `0.0` when the
    /// source has no OPT effects (matches Python's `source_opt_state`).
    pub fn source_opt_state(
        &self,
        perm: PermanentHandle,
        source_index: usize,
    ) -> f32 {
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0.0;
        };
        let stack_size = permanent.card_sources.len();
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0.0;
        };
        let is_under = source_index + 1 < stack_size;
        let card_id = source.card_id(&self.card_data).to_string();
        let Some(impl_) = self.effect_registry.get(&card_id) else {
            return 0.0;
        };
        let effects = impl_.effects(source.handle());

        let mut total = 0u32;
        let mut available = 0u32;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.max_per_turn == 0 {
                continue;
            }
            if is_under != effect.inherited {
                continue;
            }
            total += 1;
            let used = permanent.activation_count(source.handle(), slot as u8);
            if used < effect.max_per_turn {
                available += 1;
            }
        }

        if total == 0 {
            0.0
        } else {
            available as f32 / total as f32
        }
    }

    /// Shared implementation: `(total_opt_effects, used_opt_effects)` across
    /// every source in the permanent's stack with the inherited/top filter.
    fn opt_counts(&self, perm: PermanentHandle) -> (u32, u32) {
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return (0, 0);
        };
        let stack_size = permanent.card_sources.len();

        let mut total = 0u32;
        let mut used = 0u32;
        for (source_index, source) in permanent.card_sources.iter().enumerate() {
            let is_under = source_index + 1 < stack_size;
            let card_id = source.card_id(&self.card_data).to_string();
            let Some(impl_) = self.effect_registry.get(&card_id) else {
                continue;
            };
            let effects = impl_.effects(source.handle());
            for (slot, effect) in effects.iter().enumerate() {
                if effect.max_per_turn == 0 {
                    continue;
                }
                if is_under != effect.inherited {
                    continue;
                }
                total += 1;
                let count = permanent.activation_count(source.handle(), slot as u8);
                if count >= effect.max_per_turn {
                    used += 1;
                }
            }
        }
        (total, used)
    }
}
