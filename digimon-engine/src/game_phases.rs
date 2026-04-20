//! Turn lifecycle and phase transitions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! Callers still invoke `game.end_turn()`, `game.pass_turn()`, `game.activate_overclock(...)`.
//! The split exists because the forthcoming observer-firing gap work (Phase A
//! of the Medusamon plan) will land in this file — `StartOfYourTurn`,
//! `StartOfYourMainPhase`, and `OnLoseSecurity` fan-out all live at phase
//! boundaries.

use crate::enums::{EffectTiming, GamePhase, Keyword, ModifierType, PlayerId, SkipDraw};
use crate::game::{Game, OverclockError};
use crate::permanent::PermanentHandle;
use crate::selection::{AttackTarget, PendingSelection, SelectionKind};

impl Game {
    /// Begin a new turn for the current turn player.
    pub(crate) fn begin_turn(&mut self) {
        let tp = self.turn_player();

        // StartOfYourTurn fires BEFORE Unsuspend — matches Python's OnStartTurn.
        // Scripts that care about turn beginning (e.g. "at the start of your turn,
        // +1 memory") observe this timing.
        self.enqueue_triggered(
            EffectTiming::StartOfYourTurn,
            crate::selection::TriggerSource::PlayerBattleArea(tp),
        );
        self.drain_effect_queue();

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
        let tp = self.turn_player();
        // StartOfYourMainPhase fires after Draw/Breeding, before the turn player
        // takes their main-phase actions. Matches Python's OnStartMainPhase.
        self.enqueue_triggered(
            EffectTiming::StartOfYourMainPhase,
            crate::selection::TriggerSource::PlayerBattleArea(tp),
        );
        self.drain_effect_queue();

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

        // EndOfOpponentsTurn: every non-ending-player observes the turn ending.
        // Fires after EndOfYourTurn has drained but before memory flip and rotation.
        for opp in self.opponents(ending_player) {
            self.enqueue_triggered(
                EffectTiming::EndOfOpponentsTurn,
                crate::selection::TriggerSource::PlayerBattleArea(opp),
            );
        }
        self.drain_effect_queue();

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
            kind: SelectionKind::OwnField,
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
                    AttackTarget::Player(opponent),
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

    /// Fire `EndOfYourTurn` effects on every permanent in `player`'s battle area.
    /// Called by `end_turn`; exposed for tests that want to trigger swing-back.
    ///
    /// Thin wrapper over the effect-queue drainer — collects every matching
    /// effect across the battle area into `effect_queue`, then drains.
    pub fn fire_end_of_your_turn(&mut self, player: PlayerId) {
        self.enqueue_triggered(
            EffectTiming::EndOfYourTurn,
            crate::selection::TriggerSource::PlayerBattleArea(player),
        );
        self.drain_effect_queue();
    }
}
