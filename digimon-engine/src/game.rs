use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::cards::{build_registry, CardEffectRegistry};
use crate::enums::{GamePhase, PlayerId};
use crate::logger::{GameLogger, SilentLogger};
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
use crate::player::Player;
use crate::rules::Rules;
use crate::selection::{
    EffectQueue, PendingAttack, PendingOption, PendingSecurity, PendingSelection,
    SecurityResolutionState, SelectionError,
};
use crate::token_registry::TokenRegistry;

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
///
/// `impl Game` blocks for this struct are spread across three files for
/// readability; every method remains `Game::*` in the call surface:
/// - `game.rs` — struct, constructor, mulligan, state queries, memory
///   mgmt, tensor/DP/OPT helpers, elimination.
/// - `game_phases.rs` — turn lifecycle: `begin_turn`, `end_turn`,
///   `rotate_turn_player`, `pass_turn`, `activate_overclock`,
///   `fire_end_of_your_turn`.
/// - `game_actions.rs` — player mutators: `play_from_hand`,
///   `digivolve_from_hand`, `move_from_breeding`, `activate_*_main`,
///   `initiate_dna_digivolve`.
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
    /// Token metadata registry — maps canonical token names (e.g.
    /// "petrification") to `TokenDef` rows. `Game::new` pre-populates
    /// this via `token_registry::build_registry` and pushes a synthetic
    /// `CardData` row into `card_data` for each registered token so
    /// materialized tokens can reuse the standard `CardSource::data_index`
    /// machinery. Consumed by `EffectContext::play_token`.
    pub token_registry: TokenRegistry,
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
    /// Phase 8: in-flight Option card resolution. Set when an Option is
    /// played and cleared after dispose. Dispatch lands in Tasks 2-6.
    pub pending_option: Option<PendingOption>,
    /// Mid-security-check resolution state. Set by `resolve_security_card`
    /// at phase entry, mutated by `drive_security_resolution` as phases
    /// advance, and cleared at `Dispose`. Non-`None` when the engine is
    /// paused inside a security check — a `pending_selection` installed by
    /// a `SecuritySkill` process pauses resolution here; resumption is
    /// driven by `Game::advance_security_resolution`, called from
    /// `resolve_generic_selection`. See RUST_PYTHON_PARITY §2.5j.
    pub security_resolution: Option<SecurityResolutionState>,
    /// Safety rail matching Python's `_resolve_effect_stack` max-iterations=50
    /// cap. Incremented per drain step; reset to 0 when the queue empties.
    /// Prevents a self-triggering chain from hanging the engine.
    /// Consumed by the drainer in PR2.
    #[allow(dead_code)]
    pub(crate) effect_chain_depth: u16,

    /// Game logger. Defaults to `SilentLogger` (zero-overhead for RL
    /// training). Callers that want human-readable traces install a
    /// `VerboseLogger` via `set_logger`. Parity with Python's
    /// `Game.logger` field.
    pub logger: Box<dyn GameLogger>,

    /// Event buffer drained per `step` by the runner. See
    /// `src/events.rs` for the event taxonomy.
    pub events: Vec<crate::events::GameEvent>,
    /// Monotonic counter for `GameEvent::seq`. Never decreases across the
    /// lifetime of a `Game`.
    pub event_seq: u64,

    /// Current nesting depth of `Game::try_replace`. Incremented on entry,
    /// decremented on exit. At `>= MAX_REPLACEMENT_DEPTH`, the dispatcher
    /// short-circuits to `ReplacementOutcome::None` — safety rail against
    /// self-referential replacement chains (e.g. two permanents each
    /// replacing the other's deletion with "cancel").
    #[doc(hidden)]
    pub replacement_depth: u8,

    /// Outcome slot written by a replacement-selection callback (optional
    /// replacement accept path) and read by the `try_replace` caller after
    /// the selection resolves. `None` outside a replacement window; `None`
    /// on decline. See `replacement::try_replace_impl`.
    #[doc(hidden)]
    pub replacement_pending_outcome: Option<crate::replacement::ReplacementOutcome>,

    /// Spec §7.5 once-per-event guard. Records `(timing, subject)` pairs that
    /// have already fired within the current `try_replace` call chain so a
    /// redirected route does not re-fire the same timing for the same subject
    /// (e.g. `WhenWouldLeaveBattleArea` super-timing double-fire when a
    /// `Redirected(Deck)` outcome on `WhenWouldBeDeleted` routes through
    /// `return_to_deck`, which would otherwise re-invoke
    /// `WhenWouldLeaveBattleArea` for the same permanent).
    ///
    /// Cleared at the outermost entry (when `replacement_depth == 0`) of
    /// `try_replace_impl` — unless `in_replacement_commit` is set, in which
    /// case we are continuing the original call chain across a callback
    /// resolution boundary and the set must be preserved. See
    /// `replacement::try_replace_impl`.
    #[doc(hidden)]
    pub replacement_fired: std::collections::HashSet<(
        crate::enums::EffectTiming,
        crate::replacement::ReplacementSubject,
    )>,

    /// Spec §7.5 continuation marker. Set by the optional-replacement callback
    /// (accept/decline) just before invoking `commit_deferred_outcome`, cleared
    /// after the commit returns. While true, `try_replace_impl` treats a
    /// depth==0 entry as a continuation of the original call chain and does
    /// NOT clear `replacement_fired` — so the fired-set from the original
    /// event survives the callback boundary and blocks double-fires during
    /// the commit's zone-mover calls.
    #[doc(hidden)]
    pub(crate) in_replacement_commit: bool,

    /// Controller of the effect whose `process` is currently running, if
    /// any. Set by `run_queued_effect` at dispatch time and cleared at the
    /// end of the call. Consumed by `infer_deletion_cause` (and Task 4's
    /// sibling route inference helpers) to distinguish Own-effect vs
    /// Opponent-effect deletions at the fire-site. `None` when no effect is
    /// currently executing (e.g. direct-from-test call, combat,
    /// security-check driver between drains).
    #[doc(hidden)]
    pub(crate) effect_source_player: Option<PlayerId>,

    /// The cause of the deletion currently being observed by `OnDeletion`
    /// effects. Set by `commit_permanent_deletion` immediately before
    /// `enqueue_triggered(OnDeletion, ...)`; cleared after the drain via a
    /// panic-safe `catch_unwind` scope at the fire-site. Read by
    /// `EffectContext::deletion_cause()` / `was_deleted_by_effect()` /
    /// `was_deleted_by_opponent()`.
    ///
    /// `None` outside an OnDeletion observer body. Phase B §B5.
    #[doc(hidden)]
    pub(crate) current_deletion_cause: Option<crate::replacement::ReplacementCause>,

    /// Parked replacement state when a `WhenWouldBe*` replacement-process
    /// closure installs a nested player selection. Set by the dispatcher's
    /// post-process hook in `replacement::run_candidate_inner`; drained by
    /// `effect_queue::resolve_generic_selection` after the user's callback
    /// runs. `None` outside a parked-replacement scope.
    ///
    /// **Single-outstanding invariant:** at most one slot occupied at a time;
    /// the dispatcher `debug_assert!`s on duplicate install.
    ///
    /// **Coexistence with `dsl_outer_tail`** (Phase 2d): independent slots for
    /// independent concerns. Phase C §4.1.
    #[doc(hidden)]
    pub(crate) parked_replacement: Option<crate::replacement::ParkedReplacement>,

    /// Phase 9 Task 3 — set to `true` while a hand Counter Option is
    /// resolving through `play_option_from_hand`. Consumed by
    /// `play_option_core` to fire CounterEffect timing on the played
    /// card's effects BEFORE `OptionMain`. Cleared when the Counter
    /// resolver finishes the Option play. Spec §5.2.
    #[doc(hidden)]
    pub(crate) in_counter_window: bool,

    /// Phase D Task 6 — deferred-deletion finalizer. Set when an
    /// `OnDeletion`-timed effect (such as the printed `<Save>` keyword
    /// auto-install) parks a `pending_selection` mid-deletion. Cleared by
    /// `resume_pending_deletion` after the parked selection's callback
    /// resolves and the effect queue drains.
    ///
    /// While set, no further deletions can begin (the carrier slot is in an
    /// indeterminate state — its top card is mid-flight to trash, and the
    /// surrounding `commit_permanent_deletion` is paused waiting on the
    /// player's choice). Single-outstanding invariant. Enforced at the
    /// deferral site in `commit_permanent_deletion` (debug-asserted on
    /// duplicate park). A second top-level `delete_permanent_with_cause`
    /// call while this slot is set is theoretically possible but not
    /// produced by any existing fire-site under a parked selection.
    ///
    /// **Single-outstanding invariant.** Deletions don't nest in practice
    /// (the only way a second deletion could fire mid-Save would be a
    /// replacement-driven side-effect, which can't happen under a parked
    /// selection). If this assumption ever breaks, replace the field with
    /// a stack.
    ///
    /// **Coexistence with `parked_replacement`:** `parked_replacement` is
    /// drained earlier in `resolve_generic_selection`; the two slots
    /// address orthogonal concerns and do not interfere — a
    /// `WhenWouldBeDeleted` replacement parking via `parked_replacement`
    /// runs strictly before any `OnDeletion` handler can fire and park here.
    #[doc(hidden)]
    pub(crate) pending_deletion_resume: Option<crate::permanent::PermanentHandle>,
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

        // Build the token registry and absorb synthetic CardData rows
        // for each registered token. This extends `card_data_store` with
        // rows whose `card_id` matches `TokenDef::card_id`
        // (e.g. "TOKEN_PETRIFICATION") — `EffectContext::play_token`
        // uses those card_ids to look up the data_index when spawning a
        // token. Tokens never appear in a player's deck, so pushing here
        // does not affect the data_index_map used during deck seeding.
        let token_registry = crate::token_registry::build_registry();
        for def in token_registry.iter() {
            card_data_store.push(def.to_card_data());
        }

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
            token_registry,
            rng,
            next_card_index,
            mulligan_pending,
            mulligan_used,
            revealed_cards: Vec::new(),
            pending_selection: None,
            effect_queue: EffectQueue::new(),
            pending_attack: None,
            pending_security: None,
            pending_option: None,
            security_resolution: None,
            effect_chain_depth: 0,
            logger: Box::new(SilentLogger),
            events: Vec::new(),
            event_seq: 0,
            replacement_depth: 0,
            replacement_pending_outcome: None,
            replacement_fired: std::collections::HashSet::new(),
            in_replacement_commit: false,
            effect_source_player: None,
            current_deletion_cause: None,
            parked_replacement: None,
            in_counter_window: false,
            pending_deletion_resume: None,
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

    /// Swap out the game logger (defaults to `SilentLogger`). Callers
    /// that want to capture trace/reject messages should install a
    /// `VerboseLogger` here.
    pub fn set_logger(&mut self, logger: Box<dyn GameLogger>) {
        self.logger = logger;
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

    /// Fire all applicable replacement effects for the given would-event.
    /// Returns the final `ReplacementOutcome` the caller must honor.
    ///
    /// **Invariant:** if this returns `ReplacementOutcome::None`, no side
    /// effects have been applied to `Game` state. If it returns any other
    /// variant, side effects from the chosen replacements have already
    /// committed and the caller must NOT re-apply the original event.
    ///
    /// **Optional replacements:** if an optional replacement is in scope,
    /// this function installs a `PendingSelection::Replacement` and returns
    /// `ReplacementOutcome::None`. The caller is expected to re-enter
    /// `try_replace` — or inspect `game.replacement_pending_outcome` —
    /// once `resolve_selection` has fired.
    ///
    /// Visibility note: `#[doc(hidden)] pub` rather than `pub(crate)` so the
    /// Phase 7 integration tests under `digimon-engine/tests/replacements/`
    /// can drive the dispatcher directly. Fire-sites inside the crate (Task
    /// 3+) will call this via normal crate-local dispatch.
    #[doc(hidden)]
    pub fn try_replace(
        &mut self,
        timing: crate::enums::EffectTiming,
        subject: crate::replacement::ReplacementSubject,
        cause: crate::replacement::ReplacementCause,
        original_destination: Option<crate::enums::Zone>,
    ) -> crate::replacement::ReplacementOutcome {
        crate::replacement::try_replace_impl(
            self,
            timing,
            subject,
            cause,
            original_destination,
        )
    }

    /// Infer the `ReplacementCause` for a deletion of `target_handle` given
    /// the current game state. Priority:
    ///   1. `security_resolution.is_some()` → `SecurityCheck`
    ///   2. `pending_attack.is_some()` → `Battle`
    ///   3. `effect_source_player.is_some()` — an effect is currently
    ///      running; `Own` if its controller equals the target's
    ///      controller, otherwise `Opponent`.
    ///   4. Fallback → `OwnEffect`.
    ///
    /// Consumed by the deletion fire-site in `combat::delete_permanent_with_effects`.
    pub(crate) fn infer_deletion_cause(
        &self,
        target_handle: crate::permanent::PermanentHandle,
    ) -> crate::replacement::ReplacementCause {
        use crate::replacement::ReplacementCause;
        if self.security_resolution.is_some() {
            return ReplacementCause::SecurityCheck;
        }
        if self.pending_attack.is_some() {
            return ReplacementCause::Battle;
        }
        if let Some(acting) = self.effect_source_player {
            if acting == target_handle.player {
                return ReplacementCause::OwnEffect;
            }
            return ReplacementCause::OpponentEffect;
        }
        ReplacementCause::OwnEffect
    }

    /// Generalized cause inference for non-deletion Would-replacement fire-sites
    /// (return-to-hand/deck, trash-by-effect, draw, place-in-security,
    /// de-digivolve, etc.).
    ///
    /// Differs from `infer_deletion_cause` in that `Battle` is NOT a candidate:
    /// non-deletion routes are never reached via `resolve_battle`, so the only
    /// live signals are security-resolution and the effect-source player.
    ///
    /// Priority:
    ///   1. `security_resolution.is_some()` → `SecurityCheck`
    ///   2. `effect_source_player.is_some()` — compare against `target_player`;
    ///      equal → `OwnEffect`, else `OpponentEffect`.
    ///   3. Fallback → `OwnEffect`.
    ///
    /// Consumed by Task 4 fire-sites in `game_actions` / `effect_context`.
    pub(crate) fn infer_effect_cause(
        &self,
        target_player: PlayerId,
    ) -> crate::replacement::ReplacementCause {
        use crate::replacement::ReplacementCause;
        if self.security_resolution.is_some() {
            return ReplacementCause::SecurityCheck;
        }
        if let Some(acting) = self.effect_source_player {
            if acting == target_player {
                return ReplacementCause::OwnEffect;
            }
            return ReplacementCause::OpponentEffect;
        }
        ReplacementCause::OwnEffect
    }

    /// Test-only setter for `effect_source_player`. Production code MUST go
    /// through `run_queued_effect` (which sets/restores around the dispatch).
    /// Exposed `#[doc(hidden)] pub` so behavioral tests under
    /// `digimon-engine/tests/` can simulate "opponent effect currently
    /// resolving" without driving the queue.
    #[doc(hidden)]
    pub fn set_effect_source_player_for_test(
        &mut self,
        source: Option<crate::enums::PlayerId>,
    ) {
        self.effect_source_player = source;
    }

    /// Test-only setter for `parked_replacement`. Production code must go
    /// through the dispatcher's post-process hook in
    /// `replacement::run_candidate_inner`. Exposed so behavioral tests under
    /// `digimon-engine/tests/` can install a parked-replacement slot
    /// directly without driving an entire replacement-dispatch flow.
    #[doc(hidden)]
    pub fn install_parked_replacement_for_test(
        &mut self,
        parked: crate::replacement::ParkedReplacement,
    ) {
        self.parked_replacement = Some(parked);
    }

    /// Test-only getter for the parked-replacement outcome. The
    /// `parked_replacement` field is `pub(crate)`, so behavioral tests
    /// under `digimon-engine/tests/` cannot read it directly. Returns
    /// `None` when no replacement is parked.
    #[doc(hidden)]
    pub fn parked_replacement_outcome_for_test(
        &self,
    ) -> Option<crate::replacement::ReplacementOutcome> {
        self.parked_replacement.as_ref().map(|p| p.outcome)
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

    // ─── Event accumulator ─────────────────────────────────────────

    /// Allocate the next monotonic event sequence number.
    pub fn next_event_seq(&mut self) -> u64 {
        let s = self.event_seq;
        self.event_seq += 1;
        s
    }

    /// Drain accumulated events, returning them in emission order. The
    /// `HeadlessRunner::step` wrapper calls this after each action so the
    /// PyO3 layer can expose a per-step event list.
    pub fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.events)
    }

    // ─── Memory management ─────────────────────────────────────────

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
        let delta = new_memory - self.memory;
        self.memory = new_memory;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
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
        let before = self.memory;
        self.memory = (self.memory + amount).min(self.rules.memory_range.1);
        let delta = self.memory - before;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
    }

    /// Set memory to a specific value.
    pub fn set_memory(&mut self, value: i16) {
        let before = self.memory;
        self.memory = value.clamp(self.rules.memory_range.0, self.rules.memory_range.1);
        let delta = self.memory - before;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
    }

    // ─── Elimination / winner ──────────────────────────────────────

    /// Handle deck-out for a player.
    pub(crate) fn handle_deckout(&mut self, player_id: PlayerId) {
        if self.rules.player_count == 2 {
            // Standard: deck-out = loss
            self.game_over = true;
            let opponents = self.opponents(player_id);
            self.winner = opponents.first().copied();
            self.current_phase = GamePhase::GameOver;
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::GameOver {
                seq,
                winner: self.winner,
            });
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
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::GameOver {
                seq,
                winner: self.winner,
            });
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
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::GameOver {
            seq,
            winner: self.winner,
        });
    }

    /// Allocate a new unique card index (for tokens, etc.).
    pub fn next_card_index(&mut self) -> u16 {
        let idx = self.next_card_index;
        self.next_card_index += 1;
        idx
    }

    // --- Convenience methods that avoid borrow conflicts ---

    /// Suspend a single permanent. Fires `OnSuspend` observers in every
    /// player's battle area if the permanent was not already suspended.
    ///
    /// This is the canonical chokepoint for single-target suspension.
    /// `Player::unsuspend_all` (bulk turn-begin unsuspend) intentionally
    /// bypasses this path — `StartOfYourTurn` is the canonical timing for
    /// turn-start effects.
    pub fn suspend(&mut self, handle: PermanentHandle) {
        let already = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.is_suspended)
            .unwrap_or(true); // treat out-of-range as "already suspended" to no-op
        if already {
            return;
        }
        if let Some(perm) = self
            .players
            .get_mut(handle.player as usize)
            .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        {
            perm.is_suspended = true;
        }
        let n = self.players.len();
        for pid in 0..n {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnSuspend,
                crate::selection::TriggerSource::PlayerBattleArea(pid as crate::enums::PlayerId),
            );
        }
        self.drain_effect_queue();
    }

    /// Unsuspend a single permanent. Fires `OnUnsuspend` observers in every
    /// player's battle area if the permanent was suspended.
    ///
    /// See `suspend` for the bulk-unsuspend caveat.
    pub fn unsuspend(&mut self, handle: PermanentHandle) {
        let was_suspended = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.is_suspended)
            .unwrap_or(false); // treat out-of-range as "not suspended" to no-op
        if !was_suspended {
            return;
        }
        if let Some(perm) = self
            .players
            .get_mut(handle.player as usize)
            .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        {
            perm.is_suspended = false;
        }
        let n = self.players.len();
        for pid in 0..n {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnUnsuspend,
                crate::selection::TriggerSource::PlayerBattleArea(pid as crate::enums::PlayerId),
            );
        }
        self.drain_effect_queue();
    }

    /// Hatch for a player (copies turn_count to avoid borrow conflict).
    /// Fires `OnHatch` observers in every player's battle area after the egg
    /// moves into the breeding area.
    pub fn hatch(&mut self, player_id: PlayerId) -> bool {
        let turn = self.turn_count;
        let ok = self.player_mut(player_id).hatch(turn);
        if ok {
            let n = self.players.len();
            for pid in 0..n {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnHatch,
                    crate::selection::TriggerSource::PlayerBattleArea(
                        pid as crate::enums::PlayerId,
                    ),
                );
            }
            self.drain_effect_queue();
        }
        ok
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

    // ─── Unified keyword query (Phase 3 Task 2) ──────────────────────

    /// Unified keyword query — returns `true` if the permanent's top card
    /// has `keyword` either printed natively on its face (from
    /// `CardData.keywords`) OR granted by an active modifier.
    ///
    /// This is the canonical engine-wide keyword lookup. Engine code MUST
    /// NOT call `self.modifiers.has_keyword(...)` directly — that only
    /// sees granted keywords and would miss native printed keywords.
    ///
    /// Returns `false` for out-of-range handles (e.g. player index or
    /// battle-area index doesn't exist) so callers don't need a guard.
    pub fn has_keyword(
        &self,
        handle: PermanentHandle,
        keyword: crate::enums::Keyword,
    ) -> bool {
        // Modifier-granted (end-of-turn grants, Ally buffs, etc.)
        if self.modifiers.has_keyword(handle, keyword) {
            return true;
        }
        // Native printed on the top card's face.
        let Some(player) = self.players.get(handle.player as usize) else {
            return false;
        };
        let Some(perm) = player.battle_area.get(handle.index as usize) else {
            return false;
        };
        let top = perm.top_card();
        // `data_index` is a direct Vec index — O(1), no iteration needed.
        let card_data = &self.card_data[top.data_index];
        card_data.keywords.contains(&keyword)
    }

    /// Returns the `PermanentHandle` of the currently-attacking permanent,
    /// or `None` when no attack is in flight. Reads `pending_attack.attacker`
    /// — the same source the mask and combat-resolution code use.
    ///
    /// Used by `progress_excludes` to gate opponent-effect mutations on
    /// the Progress carrier specifically while it is the attacker.
    pub fn current_attacker(&self) -> Option<PermanentHandle> {
        self.pending_attack.as_ref().map(|p| p.attacker)
    }

    /// Gate predicate for the `<Progress>` keyword **and** the
    /// `ImmunityToOpponentEffects` modifier (both surface the same
    /// "opponent cannot target this with effects while it is the current
    /// attacker" rule; bundling them keeps every opponent-effect call-site
    /// to one branch).
    ///
    /// Returns `true` when:
    ///   - `target` is the current attacker (`current_attacker() == Some(target)`), AND
    ///   - `source` is `Some(pid)` where `pid != target.player`, AND
    ///   - `target` has either `Keyword::Progress` (printed or granted) **or**
    ///     `ModifierType::ImmunityToOpponentEffects`.
    ///
    /// Returns `false` if `source` is `None` (rule-driven mutations: battle,
    /// cost, rule checks). Opponent *effects* are gated; battle damage and
    /// cost-triggered cleanup are not.
    ///
    /// `ImmunityToOpponentEffects` is currently only applied with
    /// attack-scoped expiry (`EndOfAttack` / `EndOfBattle`), so the
    /// `current_attacker` gate is always satisfied when the modifier is
    /// live. If a future card grants the modifier with broader expiry,
    /// split this into `progress_excludes` (Progress only) +
    /// `effect_immunity_excludes` (modifier only) and update both
    /// call-sites; the helpers' shape is identical so the split is
    /// mechanical.
    ///
    /// Callers: `select_opponent_permanent` (selection-time gate, Phase A)
    /// and the script-API mutation entry points on `EffectContext` (Phase B):
    /// `delete_permanent`, `return_to_hand`, `return_to_deck`, `de_digivolve`,
    /// `suspend`, and the negative-DP branches of `add_dp_modifier` /
    /// `add_modifier`.
    pub fn progress_excludes(
        &self,
        target: PermanentHandle,
        source: Option<crate::enums::PlayerId>,
    ) -> bool {
        let Some(src) = source else { return false };
        if src == target.player {
            return false;
        }
        if self.current_attacker() != Some(target) {
            return false;
        }
        self.has_keyword(target, crate::enums::Keyword::Progress)
            || self
                .modifiers
                .has(target, crate::enums::ModifierType::ImmunityToOpponentEffects)
    }

    /// Returns `true` when an effect is currently resolving AND its
    /// controller is not `target`'s controller. The "opponent effect is
    /// targeting me" predicate that drives Mephistomon-style OnDeletion
    /// riders, Scapegoat eligibility (cause ≠ OwnEffect), and the
    /// `was_deleted_by_opponent` accessor.
    ///
    /// Returns `false` when:
    ///   - no effect is currently resolving (`effect_source_player == None`),
    ///   - the resolving effect's controller equals `target.player`.
    ///
    /// Phase B §B5.
    pub fn opponent_sourced_mutation(
        &self,
        target: crate::permanent::PermanentHandle,
    ) -> bool {
        match self.effect_source_player {
            Some(src) => src != target.player,
            None => false,
        }
    }

    /// Sum the net security-attack modifier contributed by native printed
    /// `<Security A. +N>` and `<Security A. -N>` keywords on `target`.
    /// Called by `resolve_player_security_loop` alongside the existing
    /// `ModifierType::SecurityAttackChange` sum so cards with only the
    /// printed keyword behave correctly without a hand-rolled script.
    pub fn security_attack_keyword_bonus(
        &self,
        target: crate::permanent::PermanentHandle,
    ) -> i32 {
        use crate::enums::Keyword;
        let Some(player) = self.players.get(target.player as usize) else {
            return 0;
        };
        let Some(perm) = player.battle_area.get(target.index as usize) else {
            return 0;
        };
        // Sum across the entire digivolution stack — inherited keywords count.
        let mut total = 0i32;
        for src in &perm.card_sources {
            let card_data = &self.card_data[src.data_index];
            for kw in &card_data.keywords {
                match kw {
                    Keyword::SecurityAttackPlus(n) => total += *n as i32,
                    Keyword::SecurityAttackMinus(n) => total -= *n as i32,
                    _ => {}
                }
            }
        }
        total
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
        // Registry effects come first — a hand-authored script owns its
        // slot order. Phase 7 Task 6 appends keyword-derived auto-install
        // replacements (Barrier / Evade / Fragment(N) / Decode) so cards
        // with those printed keywords get the matching WhenWouldBe* process
        // without hand-authoring. Partition / ArmorPurge are intentionally
        // deferred — see `crate::cards::keyword_effects` docstring.
        let registry_effects = self.effect_registry.get(card_id).map(|impl_| impl_.effects(handle));

        // Look up CardData for this card_id. The vec scan is O(card_data_len)
        // but is only hit once per effect query, and `effects_for_card` is
        // typically called at state-change fire-sites, not in the mask hot
        // loop — so the cost is acceptable for v1.
        let auto_effects: Vec<crate::effect::Effect> = self
            .card_data
            .iter()
            .find(|cd| cd.card_id == card_id)
            .map(|cd| {
                cd.keywords
                    .iter()
                    .flat_map(|kw| {
                        crate::cards::keyword_effects::keyword_to_auto_effect(*kw, handle)
                    })
                    .collect()
            })
            .unwrap_or_default();

        match (registry_effects, auto_effects.is_empty()) {
            (Some(mut es), false) => {
                es.extend(auto_effects);
                Some(es)
            }
            (Some(es), true) => Some(es),
            (None, false) => Some(auto_effects),
            (None, true) => None,
        }
    }

    /// Resolve a `CardHandle` (card_index) to its `CardKind` by scanning all
    /// player zones.
    ///
    /// Used by `source_is_tamer` flood-gate helpers on `EffectContext` /
    /// `EffectReadContext` to discriminate Tamer-sourced effects from
    /// Digimon/Option-sourced ones (matches DCGO `ICardEffect.IsTamerEffect`).
    ///
    /// Returns `None` if no `CardSource` with the given `card_index` is found
    /// in any zone (this should not occur in practice for a live effect).
    pub(crate) fn card_kind_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::enums::CardKind> {
        let target_index = handle.0;
        for player in &self.players {
            // Hand
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
            // Trash
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
            // Battle area (card_sources stacks)
            for perm in &player.battle_area {
                if let Some(cs) = perm.card_sources.iter().find(|c| c.card_index == target_index) {
                    return Some(self.card_data[cs.data_index].card_kind);
                }
                // Linked cards (Tamer equipment)
                if let Some(cs) = perm.linked_cards.iter().find(|c| c.card_index == target_index) {
                    return Some(self.card_data[cs.data_index].card_kind);
                }
            }
            // Breeding area
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding.card_sources.iter().find(|c| c.card_index == target_index) {
                    return Some(self.card_data[cs.data_index].card_kind);
                }
            }
            // Security (e.g. when effect fires from security card)
            if let Some(cs) = player.security.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
            // Deck (rare, but possible for mid-search effects)
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
        }
        // Also check revealed_cards pool
        if let Some(cs) = self.revealed_cards.iter().find(|c| c.card_index == target_index) {
            return Some(self.card_data[cs.data_index].card_kind);
        }
        None
    }

    /// Resolve a `CardHandle` to its `&CardData` by scanning all zones —
    /// mirrors `card_kind_for_handle` but returns the full data record so
    /// callers can read name, traits, colors, etc. Used by the DSL predicate
    /// evaluator (`dsl_cards::predicate`).
    ///
    /// Returns `None` if no `CardSource` with the given `card_index` is found.
    pub fn card_data_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<&crate::card_data::CardData> {
        let target_index = handle.0;
        for player in &self.players {
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            for perm in &player.battle_area {
                if let Some(cs) = perm.card_sources.iter().find(|c| c.card_index == target_index) {
                    return Some(&self.card_data[cs.data_index]);
                }
                if let Some(cs) = perm.linked_cards.iter().find(|c| c.card_index == target_index) {
                    return Some(&self.card_data[cs.data_index]);
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding.card_sources.iter().find(|c| c.card_index == target_index) {
                    return Some(&self.card_data[cs.data_index]);
                }
            }
            if let Some(cs) = player.security.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
        }
        if let Some(cs) = self.revealed_cards.iter().find(|c| c.card_index == target_index) {
            return Some(&self.card_data[cs.data_index]);
        }
        None
    }

    // ─── Tensor support: per-source DP + OPT helpers (§3.1 / §3.2) ───

    /// Sum of static `dp_modifier` values from a single source's effects
    /// that pass the inherited/top filter and their condition (if any).
    /// Returns a signed raw DP delta. Tensor writes this divided by DP_NORM.
    pub fn source_dp_contribution(
        &self,
        perm: crate::permanent::PermanentHandle,
        source_index: usize,
    ) -> i32 {
        use crate::effect_context::EffectReadContext;
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
    pub fn opt_total(&self, perm: crate::permanent::PermanentHandle) -> u32 {
        self.opt_counts(perm).0
    }

    /// Number of OPT effects whose activation count this turn has reached
    /// their `max_per_turn` cap.
    pub fn opt_used(&self, perm: crate::permanent::PermanentHandle) -> u32 {
        self.opt_counts(perm).1
    }

    /// Per-source OPT availability fraction in `[0.0, 1.0]`. `0.0` when the
    /// source has no OPT effects (matches Python's `source_opt_state`).
    pub fn source_opt_state(
        &self,
        perm: crate::permanent::PermanentHandle,
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
    fn opt_counts(&self, perm: crate::permanent::PermanentHandle) -> (u32, u32) {
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

#[cfg(test)]
mod current_attacker_tests {
    use crate::debug_runner::DebugRunner;
    use crate::card_data::CardData;
    use crate::enums::{CardColor, CardKind};

    fn card(id: &str) -> CardData {
        CardData {
            card_id: id.to_string(),
            card_name: id.to_string(),
            card_kind: CardKind::Digimon,
            level: Some(4),
            dp: Some(4000),
            play_cost: 4,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            effect_class_name: id.replace('-', "_"),
            index: 0,
            norm_id: 0.0,
        }
    }

    #[test]
    fn current_attacker_is_none_outside_combat() {
        let r = DebugRunner::builder().add_card(card("A")).start();
        assert!(r.game.current_attacker().is_none());
    }

    #[test]
    fn progress_excludes_only_when_attacking_and_opponent_sourced() {
        use crate::enums::{Expiry, Keyword};
        let mut r = DebugRunner::builder()
            .add_card(CardData {
                keywords: vec![Keyword::Progress],
                ..card("PROG")
            })
            .add_card(card("OPP"))
            .start();
        let progress = r.place_on_field(0, "PROG", None);
        let _opp_perm = r.place_on_field(1, "OPP", None);

        // Case 1: not attacking → never excluded.
        assert!(
            !r.game.progress_excludes(progress, Some(1)),
            "not-attacking carrier: no exclusion"
        );

        // Case 2: attacking, but effect is own-sourced → no exclusion.
        //
        // Simulate an in-flight attack by inserting a PendingAttack.
        use crate::selection::{AttackTarget, PendingAttack};
        r.game.pending_attack = Some(PendingAttack {
            attacker: progress,
            original_target: AttackTarget::Player(1),
            effective_target: AttackTarget::Player(1),
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            cancelled: false,
            battle_occurred: false,
            return_phase: crate::enums::GamePhase::Main,
            state: crate::selection::AttackState::Declared,
            counter_depth: 0,
        });
        assert!(
            !r.game.progress_excludes(progress, Some(0)),
            "own-sourced effect on own Progress: no exclusion"
        );
        assert!(
            !r.game.progress_excludes(progress, None),
            "no source player: no exclusion"
        );

        // Case 3: attacking + opponent-sourced → excluded.
        assert!(
            r.game.progress_excludes(progress, Some(1)),
            "opponent-sourced effect on attacking Progress carrier: excluded"
        );

        // Clean up the fake attack state to avoid leaking into later tests.
        r.game.pending_attack = None;

        // Case 4: Progress granted via modifier also triggers.
        let plain = r.place_on_field(0, "OPP", None);
        assert!(!r.game.progress_excludes(plain, Some(1)));
        r.game.modifiers.grant_keyword(
            plain,
            Keyword::Progress,
            Expiry::EndOfTurn,
            0,
        );
        r.game.pending_attack = Some(PendingAttack {
            attacker: plain,
            original_target: AttackTarget::Player(1),
            effective_target: AttackTarget::Player(1),
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            cancelled: false,
            battle_occurred: false,
            return_phase: crate::enums::GamePhase::Main,
            state: crate::selection::AttackState::Declared,
            counter_depth: 0,
        });
        assert!(
            r.game.progress_excludes(plain, Some(1)),
            "modifier-granted Progress should gate the same"
        );
    }

    #[test]
    fn opponent_sourced_mutation_only_when_effect_source_differs() {
        let mut r = DebugRunner::builder()
            .add_card(card("A"))
            .add_card(card("B"))
            .start();
        let a = r.place_on_field(0, "A", None);
        let _b = r.place_on_field(1, "B", None);

        // No effect resolving → false.
        assert!(!r.game.opponent_sourced_mutation(a));

        // Own effect resolving → false.
        r.game.set_effect_source_player_for_test(Some(0));
        assert!(!r.game.opponent_sourced_mutation(a));

        // Opponent effect resolving → true.
        r.game.set_effect_source_player_for_test(Some(1));
        assert!(r.game.opponent_sourced_mutation(a));

        r.game.set_effect_source_player_for_test(None);
    }
}
