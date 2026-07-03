//! Game setup / construction / deck + mulligan (Tier 1).

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

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

        // Build (or reuse) the shared, enriched, immutable card store. It is a
        // pure function of `all_card_data` — memoized and `Arc`-shared across
        // games, so each `Game::new` is an `Arc` clone + deck deal rather than a
        // clone+enrich+index of the ~4000-card store (was ~14 ms/game, 61% of the
        // per-step training cost). See `crate::card_store`.
        let shared = crate::card_store::shared_card_store(all_card_data);

        let mut next_card_index: u16 = 0;

        // Create players and populate decks
        let mut players = Vec::with_capacity(rules.player_count as usize);
        for (player_idx, deck_ids) in deck_card_ids.iter().enumerate() {
            let player_id = player_idx as PlayerId;
            let mut player = Player::new(player_id);
            let mut original_deck_counts: BTreeMap<String, (u16, bool)> = BTreeMap::new();

            for card_id in deck_ids {
                let data_idx = shared
                    .index
                    .get(card_id)
                    .ok_or_else(|| format!("Card {} not found in card database", card_id))?;

                let card_data = &shared.data[*data_idx];
                let card = CardSource::new(*data_idx, player_id, next_card_index);
                next_card_index += 1;

                // Route to correct deck based on card kind
                if card_data.card_kind == crate::enums::CardKind::DigiEgg {
                    player.digitama_deck.push(card);
                    let entry = original_deck_counts
                        .entry(card_id.clone())
                        .or_insert((0, true));
                    entry.0 += 1;
                    entry.1 = true;
                } else {
                    player.deck.push(card);
                    let entry = original_deck_counts
                        .entry(card_id.clone())
                        .or_insert((0, false));
                    entry.0 += 1;
                }
            }

            player.original_deck = original_deck_counts
                .into_iter()
                .map(|(card_id, (count, is_digitama))| OriginalDeckCardCount {
                    card_id,
                    count,
                    is_digitama,
                })
                .collect();

            // Shuffle decks
            player.shuffle_deck(&mut rng);
            player.shuffle_digitama_deck(&mut rng);

            players.push(player);
        }

        // Initial turn order / coin flip. Seeded games use a backend-neutral
        // starting-player selection so Rust/Python parity does not depend on
        // matching RNG algorithms. Preserve the historical RNG consumption for
        // seeded games so deck/mulligan reshuffles remain stable.
        let mut turn_order: Vec<PlayerId> = (0..rules.player_count).collect();
        if let Some(s) = seed {
            let mut consumed_turn_order = turn_order.clone();
            consumed_turn_order.shuffle(&mut rng);
            if !turn_order.is_empty() {
                let start = (s as usize) % turn_order.len();
                turn_order.rotate_left(start);
            }
        } else {
            turn_order.shuffle(&mut rng);
        }
        let memory_pair = if turn_order.len() >= 2 {
            (turn_order[0], turn_order[1])
        } else {
            (turn_order[0], turn_order[0])
        };

        let player_count = rules.player_count as usize;
        let mulligan_pending = turn_order.clone();
        let mulligan_used = vec![false; player_count];

        // Token registry handle for the `Game.token_registry` field. The
        // synthetic token `CardData` rows are now absorbed into the shared card
        // store by `card_store::build_shared_card_store` (deck-independent), so
        // the per-game absorption loop is gone.
        let token_registry = crate::token_registry::build_registry();

        let mut game = Self {
            rules,
            effects_cache: super::EffectsCache::default(),
            players,
            turn_count: 0,
            n_digivolutions: [0u32, 0u32],
            n_dna_digivolutions: [0u32, 0u32],
            n_digivolve_driven_attacks: [0u32, 0u32],
            digimon_attacks_this_turn: [0u32, 0u32],
            current_phase: GamePhase::Mulligan,
            memory: 0,
            memory_pair,
            turn_order,
            turn_player_idx: 0,
            game_over: false,
            winner: None,
            terminal_outcome_reason: None,
            card_data: crate::card_store::CardStore(shared.data.clone()),
            #[cfg(feature = "dsl-yaml-loader")]
            alt_path_registry: shared.alt_paths.clone(),
            modifiers: ModifierRegistry::new(),
            floating_mass_modifiers: Vec::new(),
            // Process-cached: the ~4000-card DSL pack is lowered once, not per
            // game. `Game::new` runs per episode in training, so the uncached
            // `build_registry()` (~190 ms) dominated wall-time. See
            // `cards::build_registry_cached`.
            effect_registry: crate::cards::build_registry_cached(),
            formula_extensions: FormulaExtensionRegistry::empty(),
            token_registry,
            rng,
            next_card_index,
            mulligan_pending,
            mulligan_used,
            revealed_cards: Vec::new(),
            pending_selection: None,
            pending_selection_resume: None,
            after_selection_resume_hooks: crate::resume::ResumeContinuationHooks::default(),
            effect_queue: EffectQueue::new(),
            pending_granted_fires: Vec::new(),
            granted_effect_bodies: crate::modifiers::GrantedEffectBodyRegistry::default(),
            next_granted_effect_id: 0,
            pending_pay_cost_effect: None,
            pending_pay_cost_stack: Vec::new(),
            pending_attack: None,
            pending_security: None,
            pending_effect_security_removal: Vec::new(),
            pending_option: None,
            pending_digixros_transaction: None,
            digixros_leaving_limbo: Vec::new(),
            active_digixros_wildcards: Vec::new(),
            pending_option_placed_turn_check: false,
            pending_option_placed_link_resume: None,
            security_resolution: None,
            effect_chain_depth: 0,
            effect_drain_depth: 0,
            logger: Box::new(SilentLogger),
            events: Vec::new(),
            event_seq: 0,
            replacement_depth: 0,
            replacement_pending_outcome: None,
            last_play_order_choice: None,
            pending_would_play_resume: None,
            pending_assembly_materials: None,
            pending_would_link_resume: None,
            pending_digimon_link: None,
            pending_link_host: None,
            pending_would_digivolve_resume: None,
            player_digivolve_cost_reducers: Vec::new(),
            pending_player_digivolve_reduction: 0,
            pending_interactive_digivolve_reduction: 0,
            pending_interactive_option_use_reduction: 0,
            interactive_option_use_reducer_prompted: false,
            pending_cost_reduction_amount_override: None,
            pending_digivolve_route_choice: None,
            replacement_fired: std::collections::HashSet::new(),
            in_replacement_commit: false,
            effect_source_player: None,
            effect_source_card: None,
            effect_source_permanent: None,
            current_trigger_context: None,
            current_deletion_cause: None,
            current_deletion_event_cause_override: None,
            pending_overclock_attack: None,
            declined_overclock_this_eot: HashSet::new(),
            current_dna_origin: None,
            parked_replacement: None,
            dsl_replacement_outcome: None,
            in_counter_window: false,
            effect_driven_option_use: false,
            active_deletion_batch: None,
            dsl_outer_tail: None,
            dsl_resolved_tail_bindings: None,
            dsl_clause_aborted: false,
            scheduled_effects: Vec::new(),
            scheduled_drain_tail: None,
            scheduled_provenance_deletions: Vec::new(),
            scheduled_provenance_deletions_opp: Vec::new(),
            pending_delayed_option_lifecycle: None,
            pending_delayed_option_lifecycle_stack: Vec::new(),
            pending_end_turn_resume: None,
            draining_deferred: 0,
            play_enters_suspended: false,
            on_play_suppressor: None,
            until_condition_dirty: false,
            until_condition_last_cycle_evaluations: 0,
            until_condition_total_evaluations: 0,
            until_condition_reevaluation_cycles: 0,
            reveal_source: None,
            opaque_data_index_map: None,
            card_id_index: shared.index.clone(),
        };

        // Deal starting hands. Security is deliberately NOT laid here — it
        // waits until mulligan finalizes, so a player who mulligans has the
        // full deck to re-shuffle into (matches Python setup order).
        for i in 0..player_count {
            game.players[i].draw_many(game.rules.starting_hand);
        }

        Ok(game)
    }

    /// Reset this game's **mutable** state in place to a clean, pre-deal
    /// state — reusing the already-built immutable shared state
    /// (`card_data`, `effect_registry`, `formula_extensions`,
    /// `token_registry`, `alt_path_registry`), `rules`, and the installed
    /// `logger`. This is the cheap foundation for replay reset-and-replay
    /// (see `runners::replay`): backward seek resets in place and replays
    /// forward instead of reconstructing via `Game::new` (which would clone
    /// every `CardData` and rebuild every registry).
    ///
    /// A full-state snapshot of `Game` is not possible — the mutable graph
    /// is closure-bearing (`ModifierEntry` is non-`Clone`,
    /// `pending_selection` carries a boxed callback, several parked
    /// continuations hold closures). Reset-and-replay sidesteps that.
    ///
    /// **Maintenance invariant:** every mutable field added to `Game` MUST
    /// be reset here to its `Game::new` default. The
    /// `reset_for_replay_restores_defaults` guard test asserts this; keep
    /// the two in lockstep. Fields intentionally preserved: `rules`,
    /// `card_data`, `alt_path_registry`, `effect_registry`,
    /// `formula_extensions`, `token_registry`, `logger`.
    pub fn reset_for_replay(&mut self) {
        let player_count = self.rules.player_count as usize;
        // Fresh players — zones are re-laid by the replay relay step.
        self.players = (0..player_count)
            .map(|i| Player::new(i as PlayerId))
            .collect();
        self.turn_count = 0;
        self.n_digivolutions = [0u32, 0u32];
        self.n_dna_digivolutions = [0u32, 0u32];
        self.n_digivolve_driven_attacks = [0u32, 0u32];
        self.digimon_attacks_this_turn = [0u32, 0u32];
        self.current_phase = GamePhase::Mulligan;
        self.memory = 0;
        let turn_order: Vec<PlayerId> = (0..self.rules.player_count).collect();
        self.memory_pair = if turn_order.len() >= 2 {
            (turn_order[0], turn_order[1])
        } else {
            (turn_order[0], turn_order[0])
        };
        self.turn_order = turn_order;
        self.turn_player_idx = 0;
        self.game_over = false;
        self.winner = None;
        self.terminal_outcome_reason = None;
        // card_data / alt_path_registry / effect_registry / formula_extensions
        // / token_registry intentionally preserved (immutable shared state).
        self.modifiers = ModifierRegistry::new();
        self.floating_mass_modifiers = Vec::new();
        // Reseed deterministically — mirrors the historical backward-seek
        // rebuild path (`Game::new(.., Some(0))`).
        self.rng = StdRng::seed_from_u64(0);
        self.next_card_index = 0;
        self.mulligan_pending = Vec::new();
        self.mulligan_used = vec![false; player_count];
        self.revealed_cards = Vec::new();
        self.pending_selection = None;
        self.effect_queue = EffectQueue::new();
        self.pending_granted_fires = Vec::new();
        self.granted_effect_bodies = crate::modifiers::GrantedEffectBodyRegistry::default();
        self.next_granted_effect_id = 0;
        self.pending_pay_cost_effect = None;
        self.pending_pay_cost_stack = Vec::new();
        self.pending_attack = None;
        self.pending_security = None;
        self.pending_effect_security_removal = Vec::new();
        self.pending_option = None;
        self.pending_option_placed_turn_check = false;
        self.pending_option_placed_link_resume = None;
        self.security_resolution = None;
        self.effect_chain_depth = 0;
        // logger intentionally preserved (session owns the logger choice).
        self.events = Vec::new();
        self.event_seq = 0;
        self.replacement_depth = 0;
        self.replacement_pending_outcome = None;
        self.last_play_order_choice = None;
        self.pending_would_play_resume = None;
        self.pending_would_link_resume = None;
        self.pending_digimon_link = None;
        self.pending_link_host = None;
        self.pending_would_digivolve_resume = None;
        self.player_digivolve_cost_reducers = Vec::new();
        self.pending_player_digivolve_reduction = 0;
        self.pending_interactive_digivolve_reduction = 0;
        self.pending_interactive_option_use_reduction = 0;
        self.interactive_option_use_reducer_prompted = false;
        self.pending_cost_reduction_amount_override = None;
        self.replacement_fired = std::collections::HashSet::new();
        self.in_replacement_commit = false;
        self.effect_source_player = None;
        self.effect_source_card = None;
        self.effect_source_permanent = None;
        self.current_trigger_context = None;
        self.current_deletion_cause = None;
        self.current_deletion_event_cause_override = None;
        self.pending_overclock_attack = None;
        self.declined_overclock_this_eot = HashSet::new();
        self.current_dna_origin = None;
        self.parked_replacement = None;
        self.dsl_replacement_outcome = None;
        self.in_counter_window = false;
        self.active_deletion_batch = None;
        self.dsl_outer_tail = None;
        self.dsl_resolved_tail_bindings = None;
        self.dsl_clause_aborted = false;
        self.scheduled_effects = Vec::new();
        self.scheduled_drain_tail = None;
        self.scheduled_provenance_deletions = Vec::new();
        self.scheduled_provenance_deletions_opp = Vec::new();
        self.pending_delayed_option_lifecycle = None;
        self.pending_delayed_option_lifecycle_stack = Vec::new();
        self.pending_end_turn_resume = None;
        self.draining_deferred = 0;
        self.until_condition_dirty = false;
        self.until_condition_last_cycle_evaluations = 0;
        self.until_condition_total_evaluations = 0;
        self.until_condition_reevaluation_cycles = 0;
        self.reveal_source = None;
        self.opaque_data_index_map = None;
    }

    /// Create a new game where one player's deck composition is known but
    /// its order is hidden — the engine consults `reveal_source` whenever
    /// it would draw from that player's pile. Used by the DCGO replay
    /// harness when replaying PvP recordings (where the local client only
    /// observes the opponent's reveals incrementally).
    ///
    /// ## Arguments
    /// - `my_player_id`: which side (0 or 1) gets the standard ordered
    ///   `my_deck`. The other player becomes opaque.
    /// - `my_deck`: ordered deck for the calling player (drawn from
    ///   index 0 first — standard `Game::new` semantics).
    /// - `opp_decklist`: unordered card-ID multiset for the opaque
    ///   opponent. Must equal the standard deck size for `rules` and
    ///   contain only card IDs known to `all_card_data`.
    /// - `reveal_source`: supplier the engine calls whenever it needs a
    ///   card from the opaque pile. The replay harness preloads a
    ///   `RevealQueue` with reveals observed from the recording.
    ///
    /// ## Integration scope (Phase 1)
    ///
    /// This constructor wires opaque draws into:
    /// - Initial-hand setup (the `draw_many` loop at end of `new`).
    /// - Mulligan redraws (via the same `draw_many` path post-redraw).
    /// - Security setup (`setup_security`).
    ///
    /// Mid-game draw paths (per-turn draw, effect-driven mill/draw/peek)
    /// are NOT yet integrated. Games that trigger those paths against the
    /// opaque player will panic with a clear "opaque mode + this draw
    /// path not yet supported" message rather than silently consume from
    /// the empty `deck` Vec. See task 6.6 follow-up.
    pub fn new_with_opaque_opponent(
        my_player_id: PlayerId,
        my_deck: Vec<String>,
        opp_decklist: Vec<String>,
        reveal_source: Box<dyn crate::opaque_deck::RevealSource>,
        all_card_data: &std::collections::HashMap<String, CardData>,
        rules: Rules,
        seed: Option<u64>,
    ) -> Result<Self, String> {
        // Standard player count is 2; opaque mode is undefined for >2 player
        // formats and explicitly rejected here.
        if rules.player_count != 2 {
            return Err(format!(
                "opaque-opponent-deck mode requires rules.player_count == 2, got {}",
                rules.player_count
            ));
        }
        if my_player_id >= 2 {
            return Err(format!("my_player_id must be 0 or 1, got {}", my_player_id));
        }

        // Validate the opponent decklist size matches what the rules expect.
        // (Game::new fails downstream if a deck is malformed; we surface
        // this earlier with a clearer message for the opaque path.)
        let expected_deck_size = my_deck.len();
        if opp_decklist.len() != expected_deck_size {
            return Err(format!(
                "opponent decklist size {} differs from calling-player deck size {}; \
                 both must equal the rules-mandated deck size",
                opp_decklist.len(),
                expected_deck_size
            ));
        }
        // Validate every card ID in the opponent decklist is known to the
        // card pool. (Game::new validates this for the in-order deck path
        // implicitly via the data_index_map lookup; we duplicate the check
        // here so opaque-mode failures surface at construction time rather
        // than at the first reveal.)
        for card_id in &opp_decklist {
            if !all_card_data.contains_key(card_id) {
                return Err(format!(
                    "opponent decklist contains unknown card ID `{}`",
                    card_id
                ));
            }
        }

        // For Game::new's standpoint, both decks must be provided. We
        // supply the opaque opponent a deterministic placeholder deck of
        // their declared composition — Game::new will populate
        // `player.deck` and shuffle it, then we immediately clear it
        // below and install the opaque state. This is the simplest
        // integration that reuses Game::new's effect-registry setup
        // and avoids forking the constructor.
        let placeholder_opp_deck = opp_decklist.clone();
        let decks_for_constructor = if my_player_id == 0 {
            vec![my_deck.clone(), placeholder_opp_deck]
        } else {
            vec![placeholder_opp_deck, my_deck.clone()]
        };

        let mut game = Self::new(&decks_for_constructor, all_card_data, rules, seed)?;

        // Replace the opponent's ordered deck with opaque state, and drop
        // the placeholder hand draws — opaque mode redraws via the
        // reveal source below.
        let opp_id = if my_player_id == 0 { 1u8 } else { 0u8 };
        let opp_idx = opp_id as usize;

        // Clear out the standard-path setup the opponent just received.
        // We're about to redraw their starting hand from the reveal source.
        // Also dump their digitama deck to be re-populated — wait, actually
        // digitama is dealt separately and is part of the decklist already
        // (4-5 DigiEggs interleaved). For Phase 1 we preserve the digitama
        // deck as-is (it's already shuffled and held in
        // `player.digitama_deck`) — opaque mode for digitama isn't in scope.
        let starting_hand = game.rules.starting_hand;
        game.players[opp_idx].hand.clear();
        game.players[opp_idx].deck.clear();

        // Cache the card-id → data_index lookup so opaque-mode reveals can
        // materialize CardSources.
        let mut data_index_map = std::collections::HashMap::new();
        for (i, card) in game.card_data.iter().enumerate() {
            data_index_map.insert(card.card_id.clone(), i);
        }
        game.opaque_data_index_map = Some(data_index_map);

        // Install the opaque deck state. Note: the digitama cards have
        // already been pulled out of the decklist by Game::new's per-card
        // routing (card_kind == DigiEgg goes to digitama_deck, others go
        // to deck). The opaque-deck multiset should reflect ONLY the
        // non-digitama portion to stay consistent.
        let non_digitama: Vec<String> = opp_decklist
            .iter()
            .filter(|id| {
                let data = match all_card_data.get(*id) {
                    Some(d) => d,
                    None => return false, // already validated above; unreachable
                };
                data.card_kind != crate::enums::CardKind::DigiEgg
            })
            .cloned()
            .collect();
        game.players[opp_idx].opaque_deck_state = Some(
            crate::opaque_deck::OpaqueDeckState::from_decklist(&non_digitama),
        );

        game.reveal_source = Some(reveal_source);

        // Redraw the opaque opponent's starting hand from the reveal
        // source. This is the first place real reveals get consumed.
        for _ in 0..starting_hand {
            game.draw_one_for_player(opp_id)?;
        }

        Ok(game)
    }

    /// Draw one card for `pid`. Branches on opaque mode: when
    /// `player.opaque_deck_state` is `Some`, consumes from the reveal
    /// source; otherwise pops from the ordered deck.
    ///
    /// Returns `Ok(true)` on successful draw, `Ok(false)` on deck-out
    /// (no cards remaining — either Vec is empty or opaque multiset is
    /// zero), or `Err(message)` for a reveal-source error.
    ///
    /// This is the single chokepoint setup-time draws use. Effect-driven
    /// draws still go through `Player::draw` directly — when one of those
    /// fires for an opaque player it will see an empty `deck` Vec and
    /// return false, which the engine treats as deck-out. That's the
    /// "panic with a clear message" deferred until task 6.6 follow-up;
    /// for now it manifests as the engine declaring deck-out, which is
    /// at least an obvious symptom.
    pub fn draw_one_for_player(&mut self, pid: PlayerId) -> Result<bool, String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            // Standard path.
            return Ok(self.players[pid as usize].draw());
        }
        self.reveal_into_hand(pid, crate::opaque_deck::RevealKind::Draw)
    }

    /// General-purpose "take one card from the top of `pid`'s deck" that
    /// branches on opaque mode. The caller decides which zone to push the
    /// returned CardSource into (trash for mill, revealed-list for peek,
    /// etc.) — this helper does NOT route to hand.
    ///
    /// Returns `Ok(Some(card))` on success, `Ok(None)` on deck-out (the
    /// standard-mode Vec is empty, or the opaque multiset is depleted —
    /// the latter currently surfaces as Err but may become Ok(None) once
    /// the engine has a typed deck-out vs reveal-error split).
    ///
    /// Used by effect-driven draws/mills/peeks that consume from the deck
    /// top but don't go through the hand. Each call site picks the right
    /// `kind` — `Mill` for trash-from-top, `Effect` for peek-and-reveal,
    /// `Draw` for "draw to hand" semantics (though `draw_one_for_player`
    /// is the convenience wrapper for that case).
    pub fn take_from_deck_top_for_player(
        &mut self,
        pid: PlayerId,
        kind: crate::opaque_deck::RevealKind,
    ) -> Result<Option<CardSource>, String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            // Standard path — just pop from the ordered deck Vec.
            return Ok(self.players[pid as usize].deck.pop());
        }
        // Opaque path — materialize from reveal source.
        let card = self.materialize_reveal(pid, kind)?;
        Ok(Some(card))
    }

    /// Set up `count` security cards for `pid`, branching on opaque mode.
    /// In standard mode, defers to `Player::setup_security`. In opaque
    /// mode, pushes `count` **placeholder** CardSources — their identities
    /// are materialized **lazily** when SecurityCheck flips them. This
    /// matches DCGO's PvP information model: the local client doesn't
    /// know the opponent's security cards' identities at setup time, only
    /// when they're flipped during gameplay.
    ///
    /// The opaque pile's multiset is also debited by `count` here, since
    /// these N cards are physically in the security stack (just hidden).
    /// When the placeholder is later materialized via
    /// [`Self::materialize_opaque_security_placeholder`], the multiset
    /// has already been debited and no further change happens — the
    /// materialization just resolves identity.
    pub fn setup_security_for_player(&mut self, pid: PlayerId, count: u8) -> Result<(), String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            // Standard path.
            self.players[pid as usize].setup_security(count);
            return Ok(());
        }
        // Opaque path: push placeholders, debit the multiset for the
        // count (the cards exist in security; we just don't know which).
        // The multiset is debited by `count` total via repeated
        // pop-from-multiset-without-supplier — but we don't actually
        // know WHICH cards moved to security. The honest accounting:
        // decrement `total_remaining` by `count`, leave per-card counts
        // alone. When a placeholder is later materialized with a
        // specific card_id, that card's multiset count gets decremented
        // at that point. This double-counts slightly: total_remaining
        // is decremented at setup, then again at flip-materialization.
        // We compensate by NOT decrementing total in
        // materialize_opaque_security_placeholder.
        let state = self.players[pid as usize]
            .opaque_deck_state
            .as_mut()
            .expect("checked above");
        // Decrement total_remaining via a public helper to avoid
        // bypassing accounting. (OpaqueDeckState exposes restore()
        // but not direct count manipulation — add a "debit_without_id"
        // method or thread accordingly.) For now we use the existing
        // multiset structure: we can't decrement per-card without
        // knowing which cards, so instead we track a separate
        // "placeholder count" on the state that subtracts from the
        // effective total. See OpaqueDeckState::reserve_placeholders.
        state.reserve_placeholders(count as usize);

        for _ in 0..count {
            let card_index = self.next_card_index;
            self.next_card_index += 1;
            self.players[pid as usize]
                .security
                .push(CardSource::new_opaque_security_placeholder(pid, card_index));
        }
        Ok(())
    }

    /// Convenience wrapper around [`materialize_opaque_security_placeholder`]
    /// for effect-driven security access sites — the common pattern of
    /// "I'm about to remove security[idx]; if it's an opaque placeholder,
    /// resolve its identity first so subsequent reads of its fields
    /// (card_id, color, type, replacement-effect lookups, observer
    /// firings) see real data instead of garbage `data_index = 0`."
    ///
    /// Silently no-ops on:
    ///   - invalid pid / idx (caller is about to surface that anyway)
    ///   - non-opaque player (standard-mode security has no placeholders)
    ///   - already-materialized placeholder (idempotent inner helper)
    ///
    /// Errors from the underlying materialization are logged-and-ignored
    /// (the calling effect proceeds with garbage data, replay surfaces
    /// the divergence). Lifting these to typed errors would require
    /// fallible signatures on every effect-driven security path — out
    /// of scope for the Phase-1-lazy mechanism.
    pub fn ensure_security_materialized(&mut self, pid: PlayerId, security_idx: usize) {
        if (pid as usize) >= self.players.len() {
            return;
        }
        let needs = self.players[pid as usize]
            .security
            .get(security_idx)
            .map(|c| c.is_opaque_placeholder)
            .unwrap_or(false);
        if !needs {
            return;
        }
        if let Err(e) = self.materialize_opaque_security_placeholder(pid, security_idx) {
            eprintln!(
                "[opaque-deck] effect-driven security materialize error for player {} \
                 idx {}: {}",
                pid, security_idx, e
            );
        }
    }

    /// Materialize a placeholder security card at position `security_idx`
    /// by consuming a `RevealKind::Security` reveal from the source. The
    /// placeholder's `card_index` is preserved (face_up_security tracking
    /// stays consistent); all other fields are overwritten.
    ///
    /// No-op if the position doesn't hold a placeholder (already
    /// materialized, or in standard-mode game).
    ///
    /// Called by the engine's security-pop path BEFORE reading the
    /// security card's data: when about to flip security[0], if it's a
    /// placeholder, materialize first, then proceed with normal flip
    /// semantics.
    pub fn materialize_opaque_security_placeholder(
        &mut self,
        pid: PlayerId,
        security_idx: usize,
    ) -> Result<bool, String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            return Ok(false);
        }
        let needs_materialization = self.players[pid as usize]
            .security
            .get(security_idx)
            .map(|c| c.is_opaque_placeholder)
            .unwrap_or(false);
        if !needs_materialization {
            return Ok(false);
        }

        // Pull the next Security reveal from the source.
        let card_id = {
            let source = self
                .reveal_source
                .as_mut()
                .ok_or_else(|| "opaque security flip but no reveal_source on Game".to_string())?;
            source
                .next_reveal(crate::opaque_deck::RevealKind::Security)
                .map_err(|e| e.to_string())?
        };
        // Consume from the per-card multiset. Note: total_remaining was
        // already debited at setup_security_for_player via
        // reserve_placeholders, so this consume call only decrements
        // the per-card slot, not total_remaining.
        {
            let state = self.players[pid as usize]
                .opaque_deck_state
                .as_mut()
                .expect("checked above");
            state
                .consume_per_card_only(&card_id)
                .map_err(|e| e.to_string())?;
        }
        // Look up data_index and overwrite the placeholder's fields,
        // preserving card_index for face_up_security continuity.
        let data_idx = self
            .opaque_data_index_map
            .as_ref()
            .and_then(|m| m.get(&card_id))
            .copied()
            .ok_or_else(|| {
                format!(
                    "security reveal returned card_id `{}` but it's not in the data index map",
                    card_id
                )
            })?;
        let slot = &mut self.players[pid as usize].security[security_idx];
        slot.data_index = data_idx;
        slot.is_opaque_placeholder = false;
        // face_down stays true (security is still face-down post-materialization
        // until it's actually flipped to face-up via face_up_security set
        // mutation; the engine's existing flip path handles that).
        Ok(true)
    }

    /// Swap out the game logger (defaults to `SilentLogger`). Callers
    /// that want to capture trace/reject messages should install a
    /// `VerboseLogger` here.
    pub fn set_logger(&mut self, logger: Box<dyn GameLogger>) {
        self.logger = logger;
    }

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
    pub fn accept_mulligan(&mut self, player: PlayerId, keep: bool) -> Result<(), &'static str> {
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
}
