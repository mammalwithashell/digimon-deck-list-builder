//! Turn lifecycle and phase transitions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! Callers still invoke `game.end_turn()`, `game.pass_turn()`.
//! The split exists because the forthcoming observer-firing gap work (Phase A
//! of the Medusamon plan) will land in this file — `StartOfYourTurn`,
//! `StartOfYourMainPhase`, and `OnLoseSecurity` fan-out all live at phase
//! boundaries.

use crate::action::space::PASS;
use crate::card_source::CardHandle;
use crate::effect_context::EffectReadContext;
use crate::enums::{
    CardKind, DelayTrigger, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId, SkipDraw,
};
use crate::game::{
    DelayedOptionLifecycleResume, DelayedOptionLifecycleResumeKind, EndTurnResume, Game,
    TurnBoundaryResume,
};
use crate::permanent::PermanentHandle;
use crate::selection::{AttackTarget, PendingSelection, SelectionKind};

#[derive(Debug, Clone)]
pub struct OverclockSelectionState {
    pub player: PlayerId,
    pub source_card: CardHandle,
    pub opponent: PlayerId,
}

impl Game {
    /// Set a MAIN turn phase and emit a [`crate::events::GameEvent::PhaseChange`].
    /// Used ONLY by the turn machine for the Unsuspend/Draw/Breeding/Main/EndTurn
    /// progression — NOT by the transient selection sub-phases (SelectTarget,
    /// EffectChoice, …), which set `current_phase` directly and are not
    /// turn-phase changes. `player` is the current turn player.
    fn set_turn_phase(&mut self, phase: GamePhase) {
        self.current_phase = phase;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events
            .push(crate::events::GameEvent::PhaseChange { seq, player, phase });
    }

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
        if self.pending_selection.is_some() {
            self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                turn: self.turn_count,
                kind: DelayedOptionLifecycleResumeKind::StartTurn,
                pending_delete_key: None,
                skip_key: None,
            });
            return;
        }

        self.resolve_start_delayed_options(self.turn_count);
        if self.pending_selection.is_some() {
            return;
        }

        self.continue_begin_turn_after_start_delays();
    }

    fn continue_begin_turn_after_start_delays(&mut self) {
        let tp = self.turn_player();

        // A new turn has started — fires once per turn (including turn 1, which
        // reaches here without going through the end-of-turn `turn_count` bump).
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::TurnStart {
            seq,
            player: tp,
            turn_count: self.turn_count,
        });

        // Pending processing due at the turn player's next unsuspend phase
        // (§18-1-1). Several due at once are one simultaneous pool (§18-1-2).
        // A decision in it (a `TriggerOrder`, a body's own pick) holds the
        // turn here until it resolves.
        if self.enqueue_scheduled_for_timing(EffectTiming::UntilNextUnsuspend) > 0 {
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                self.pending_turn_boundary_resume =
                    Some(TurnBoundaryResume::BeginTurnAfterUntilNextUnsuspend);
                return;
            }
        }

        self.continue_begin_turn_after_until_next_unsuspend();
    }

    fn continue_begin_turn_after_until_next_unsuspend(&mut self) {
        let tp = self.turn_player();

        // Reset per-turn state. [Once Per Turn] counters reset for BOTH
        // players — "per turn" is per GAME turn, so an [All Turns][Once Per
        // Turn] effect (e.g. BT21-073's linked leave-replacement) re-arms on
        // the opponent's turn too. Attack counters stay owner-turn-scoped.
        self.player_mut(tp).new_turn();
        for pid in 0..self.players.len() {
            if pid as crate::PlayerId != tp {
                self.player_mut(pid as crate::PlayerId)
                    .reset_effect_activations();
            }
        }
        if let Some(count) = self.digimon_attacks_this_turn.get_mut(tp as usize) {
            *count = 0;
        }

        // Unsuspend phase. The bulk turn-start unsuspend honors the
        // `CannotUnsuspend` modifier ("[All Turns] … don't unsuspend" —
        // BT12-057 Quartzmon; surfaced by judge-quiz Q3 authoring): a locked
        // permanent stays suspended. `cannot_unsuspend` also honors the
        // PLAYER-scoped mass lock ("none of their Digimon can unsuspend until
        // their turn ends" — ST24-11 Rosemon, G-PLAYER-MASS-CANNOT-UNSUSPEND).
        //
        // G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND (2026-10-01): the turn
        // player's battle-area unsuspension and the opponents' <Reboot>
        // unsuspension below form ONE simultaneous batch (DCGO
        // `TurnStateMachine.cs` collects both into a single
        // `IUnsuspendPermanents(list, null).Unsuspend()`, which then stacks
        // `OnUnTappedAnyone` for the whole batch). Every battle-area
        // permanent that actually went suspended → unsuspended fires
        // `OnUnsuspend` (non-effect-initiated). The breeding area unsuspends
        // silently, as in DCGO (it is not in the batch list). Rule 6-1-3: the
        // draw phase doesn't start until the triggered effects resolve, so a
        // prompting observer parks the turn machine (resume kind
        // `UnsuspendPhase`).
        self.set_turn_phase(GamePhase::Unsuspend);
        // G-ENGINE-AURA-REBOOT-NOT-APPLIED (2026-10-04): the turn player just
        // changed, so turn-gated declarative auras (`[Opponent's Turn]` /
        // `[Your Turn]` grants such as BT24-041's "[Opponent's Turn] … gain
        // <Reboot>") must be re-materialized BEFORE the <Reboot> scan below
        // reads `has_keyword` — otherwise the granted half still reflects the
        // previous turn and the aura-granted Reboot is missed.
        self.tick_declarative_effects();
        let mut unsuspended_batch: Vec<PermanentHandle> = Vec::new();
        {
            let n = self.player(tp).battle_area.len();
            let mut unlocked: Vec<u8> = Vec::with_capacity(n);
            for i in 0..n {
                let h = PermanentHandle {
                    player: tp,
                    index: i as u8,
                };
                // `CannotUnsuspendInUnsuspendPhase` ("can't unsuspend in their
                // next unsuspend phase" — EX13-041) gates ONLY this bulk step.
                // Player-scoped locks also cover Digimon entering after the
                // effect resolved, including tokens, but never Tamers.
                let player_phase_lock = self.modifiers.player_has(
                    tp,
                    ModifierType::CannotUnsuspendInUnsuspendPhase,
                ) && self.player(tp).battle_area[i].is_digimon_for_rules(
                    &self.card_data,
                    &self.modifiers,
                    h,
                );
                if !self.cannot_unsuspend(h)
                    && !player_phase_lock
                    && !self
                        .modifiers
                        .has(h, ModifierType::CannotUnsuspendInUnsuspendPhase)
                {
                    unlocked.push(i as u8);
                }
            }
            let breeding_handle = PermanentHandle {
                player: tp,
                index: crate::action::space::BREEDING_TARGET as u8,
            };
            let breeding_unlocked = !self.cannot_unsuspend(breeding_handle);
            let player = self.player_mut(tp);
            for i in unlocked {
                if let Some(perm) = player.battle_area.get_mut(i as usize) {
                    if perm.is_suspended {
                        unsuspended_batch.push(PermanentHandle {
                            player: tp,
                            index: i,
                        });
                    }
                    perm.is_suspended = false;
                }
            }
            if breeding_unlocked {
                if let Some(ref mut perm) = player.breeding_area {
                    perm.is_suspended = false;
                }
            }
        }
        self.mark_until_condition_dirty();

        // Phase 9 Task 7: <Reboot> consumer. "At the start of your
        // opponent's unsuspend phase, unsuspend this Digimon." Scan every
        // opponent's battle area for Digimon with the Reboot keyword
        // (printed or granted) and unsuspend them. The `CannotUnsuspend`
        // modifier gates the scan — Reboot is effect-driven unsuspension
        // and respects the suspend-lock. Reboot unsuspension joins the same
        // `OnUnsuspend` batch as the turn player's (see above).
        //
        // Collect handles first to avoid a borrow conflict with
        // `has_keyword` / modifier reads, then mutate in a second pass.
        // Non-Digimon permanents (Option states) are filtered out via
        // `Permanent::is_digimon` — Options don't attack and don't
        // legally carry Reboot, but the printed-keyword parser doesn't
        // know the card kind, so we gate here.
        let n_players = self.players.len();
        let mut reboot_handles: Vec<PermanentHandle> = Vec::new();
        for pid in 0..n_players {
            let pid = pid as PlayerId;
            if pid == tp {
                continue;
            }
            for i in 0..self.player(pid).battle_area.len() {
                let h = PermanentHandle {
                    player: pid,
                    index: i as u8,
                };
                if !self.player(pid).battle_area[i].is_digimon(&self.card_data) {
                    continue;
                }
                if !self.has_keyword(h, Keyword::Reboot) {
                    continue;
                }
                // Reboot respects both the permanent-scoped and the player-scoped
                // mass `CannotUnsuspend` lock (G-PLAYER-MASS-CANNOT-UNSUSPEND).
                if self.cannot_unsuspend(h) {
                    continue;
                }
                reboot_handles.push(h);
            }
        }
        for h in reboot_handles {
            if let Some(perm) = self
                .players
                .get_mut(h.player as usize)
                .and_then(|p| p.battle_area.get_mut(h.index as usize))
            {
                if perm.is_suspended {
                    unsuspended_batch.push(h);
                }
                perm.is_suspended = false;
            }
        }

        if !unsuspended_batch.is_empty() {
            // Suspension-keyed auras re-materialize before observers read the
            // board (same contract as `unsuspend_with_cause`).
            self.tick_declarative_effects();
            self.mark_until_condition_dirty();
            self.enqueue_unsuspend_batch(&unsuspended_batch, false);
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                    turn: self.turn_count,
                    kind: DelayedOptionLifecycleResumeKind::UnsuspendPhase,
                    pending_delete_key: None,
                    skip_key: None,
                });
                return;
            }
            if self.game_over {
                return;
            }
        }
        self.reevaluate_until_condition_modifiers_if_dirty();

        self.continue_begin_turn_after_unsuspend_phase();
    }

    /// Draw → Breeding half of the turn start, split out so a prompting
    /// `OnUnsuspend` observer fired by the unsuspend-phase batch can park the
    /// turn machine and resume here (G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND).
    pub(crate) fn continue_begin_turn_after_unsuspend_phase(&mut self) {
        let tp = self.turn_player();

        // Draw phase
        self.set_turn_phase(GamePhase::Draw);
        let should_skip_draw = match self.rules.skip_first_draw {
            // "The first player of the game skips their first draw." Turn 1
            // uniquely identifies that moment — whoever the coin flip sat at
            // `turn_order[0]` is the turn player here. Don't hardcode tp == 0.
            SkipDraw::FirstPlayerOnly => self.turn_count == 1,
            SkipDraw::AllRound1 => self.turn_count <= self.rules.player_count as u16,
            SkipDraw::None => false,
        };
        if !should_skip_draw {
            // Opaque-aware draw: branches on `player.opaque_deck_state`
            // internally. Standard mode pops from the ordered deck;
            // opaque mode consumes a `Draw` reveal from `reveal_source`.
            let drew = match self.draw_one_for_player(tp) {
                Ok(b) => b,
                Err(e) => {
                    // Opaque reveal-source error mid-game. Surfaces in
                    // the replay harness as a winner-mismatch / unexpected
                    // game-state divergence. Treat as deck-out locally so
                    // the engine doesn't crash.
                    eprintln!("[opaque-deck] per-turn draw error for player {}: {}", tp, e);
                    false
                }
            };
            if !drew {
                // Deck-out: player is eliminated (multiplayer) or loses (standard)
                self.handle_deckout(tp);
                return;
            }
            self.mark_until_condition_dirty();
            self.reevaluate_until_condition_modifiers_if_dirty();
        }

        // Breeding phase
        self.set_turn_phase(GamePhase::Breeding);
        if !self.has_actionable_breeding_option(tp) {
            self.enter_main_phase();
        }
    }

    fn has_actionable_breeding_option(&self, player_id: PlayerId) -> bool {
        let player = self.player(player_id);
        if player.breeding_area.is_none() && !player.digitama_deck.is_empty() {
            return true;
        }
        let Some(perm) = player.breeding_area.as_ref() else {
            return false;
        };
        if perm.top_card().digimon_dp(&self.card_data).is_some()
            && player.battle_area.len() < self.rules.field_slots as usize
        {
            return true;
        }
        false
    }

    /// Advance from breeding to main phase.
    ///
    /// Park-aware (user bug 2026-07-09): if a breeding-action trigger drain
    /// (e.g. EX11-008 Elizamon's `[When Moving]` pick fired by
    /// `move_from_breeding`, or an `OnHatch` observer) left a
    /// `PendingSelection` installed, advancing the phase NOW would race the
    /// selection's `previous_phase` restore — `resolve_selection` would later
    /// clobber the phase back to `Breeding`, producing a phantom second
    /// Breeding phase and a doubled `StartOfYourMainPhase` fan-out. Instead,
    /// park the advance and let `resume_pending_enter_main` (called from the
    /// `resolve_selection` tail) perform it once the selection settles. This
    /// also gives the correct rules ordering: the breeding action's triggered
    /// effect finishes resolving before the main phase begins.
    pub fn enter_main_phase(&mut self) {
        if self.pending_selection.is_some() {
            self.pending_enter_main_after_selection = true;
            self.logger.log(
                "[Phase] enter_main_phase parked: selection pending; \
                 will enter Main after it resolves",
            );
            return;
        }
        let tp = self.turn_player();
        self.set_turn_phase(GamePhase::Main);
        // StartOfYourMainPhase fires after Draw/Breeding, before the turn player
        // takes their main-phase actions. Matches Python's OnStartMainPhase.
        self.enqueue_triggered(
            EffectTiming::StartOfYourMainPhase,
            crate::selection::TriggerSource::PlayerBattleArea(tp),
        );
        self.enqueue_triggered(
            EffectTiming::StartOfYourMainPhase,
            crate::selection::TriggerSource::PlayerBreedingArea(tp),
        );
        // Both sides observe the same phase entry. Enqueue before the single
        // drain so turn-player priority and simultaneous trigger ordering apply.
        // Ordinary effects cannot activate in breeding; opponent-main has no
        // breeding-specific dispatch.
        for player in 0..self.players.len() {
            let player = player as PlayerId;
            if player != tp {
                self.enqueue_triggered(
                    EffectTiming::StartOfOpponentsMainPhase,
                    crate::selection::TriggerSource::PlayerBattleArea(player),
                );
            }
        }
        self.drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// End the current turn and advance to the next player.
    ///
    /// Fires OnEndTurn effects, checks memory swing-back (§1.5): if an OnEndTurn
    /// effect restored memory from negative to non-negative, the turn continues
    /// and returns to Main phase instead of switching.
    ///
    /// The `[End of Your Turn]` pool includes the end-of-turn attack
    /// keywords (<Engage>, <Execute>, <Vortex>, <Overclock>; §16-44-2,
    /// §16-38-2, §16-33-2, §16-34-2): each offers its attack as its own
    /// decision when its entry activates, and the end of turn waits for that
    /// attack (§6-6-2; `resume_pending_end_turn`).
    ///
    /// If the ending player then has a pending end-of-turn action (§4.6b: a
    /// `MayAttack` grant, an end-of-turn DNA route), the phase parks in
    /// `GamePhase::EndOfTurnAction` and the caller resumes by picking an attack
    /// bit or calling `pass_end_of_turn_action`. Matches Python
    /// `_complete_end_phase`.
    pub fn end_turn(&mut self) {
        if self.game_over {
            return;
        }
        // §18-3: the turn change hands the next decision to the other
        // player; the loop watch starts over (see `drain_effect_queue`).
        self.reset_loop_watch();
        self.turn_end_check_deferred = false;

        self.set_turn_phase(GamePhase::EndTurn);

        let ending_player = self.turn_player();

        // Phase 8 Task 3: resolve Delayed Options scheduled for this turn's
        // end BEFORE firing `EndOfYourTurn` observers — delayed effects are
        // part of the ending turn's resolution (see design spec §8 and the
        // plan doc, §Task 3). `DelayEffect` fires, then the permanent is
        // trashed via the Phase 7 replacement framework (cause = Cost).
        //
        // The scan is indexed on `turn_count`, not on `ending_player`: a Delay
        // played by P0 during P1's turn (via a Counter window) parks on P0's
        // battle_area with `trash_on_turn == P1's current turn`, and must fire
        // regardless of who's ending their turn.
        self.resolve_delayed_options(self.turn_count, ending_player);
        if self.pending_selection.is_some() {
            return;
        }

        self.continue_end_turn_after_delays(ending_player);
    }

    fn continue_end_turn_after_delays(&mut self, ending_player: PlayerId) {
        // Memory swing-back: capture memory before firing OnEndTurn effects,
        // fire them, then see if an effect restored memory from negative.
        let memory_before = self.memory;
        self.fire_end_of_your_turn(ending_player);
        // A decision in the pool, or an attack declared in it (an end-of-turn
        // attack keyword's processing, §16-44-2 / §16-38-2 / §16-33-2 /
        // §16-34-2), holds the end of turn until it has all resolved (§6-6-2);
        // `resume_pending_end_turn` continues it.
        if self.pending_selection.is_some()
            || self.pending_attack.is_some()
            || !self.parked_overclock_attacks.is_empty()
            || !self.deferred_end_of_turn_attack_offers.is_empty()
        {
            self.pending_end_turn_resume = Some(EndTurnResume {
                ending_player,
                memory_before_end_effects: memory_before,
            });
            return;
        }

        self.continue_end_turn_after_effects(ending_player, memory_before);
    }

    fn continue_end_turn_after_effects(&mut self, ending_player: PlayerId, memory_before: i16) {
        if memory_before < 0 && self.memory >= 0 && !self.game_over {
            self.set_turn_phase(GamePhase::Main);
            return;
        }

        // §4.6b: park in EndOfTurnAction if the player has a pending
        // end-of-turn action (a `MayAttack` grant, a registered end-of-turn
        // DNA route). Turn rotation is deferred until the player resumes via
        // `pass_end_of_turn_action`. ForceAttack is not checked here (Python
        // doesn't either): it's enforced at the Main-phase mask (§4.7d)
        // before the turn reaches `end_turn`. The end-of-turn attack keywords
        // are not here: they are trigger-type effects whose attack is
        // declared inside the `[End of Your Turn]` pool (§16-44-2, §16-38-2,
        // §16-33-2, §16-34-2).
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
        self.declined_overclock_this_eot.clear();
        // An end-of-turn attack's settle request belongs to this turn's end:
        // never carry it into the next turn's `EndOfTurnAction` window.
        self.end_of_turn_attack_settle_pending = false;

        // Expire end-of-turn modifiers/keywords for the ending player's turn.
        // Track H Phase 4k: collect body_ids from granted-triggered entries
        // that the expire pass will drop, then prune the corresponding
        // closure bodies from `Game::granted_effect_bodies` to keep the
        // registry from leaking after turn-bound grants expire.
        let dropped_body_ids = self
            .modifiers
            .collect_expiring_granted_body_ids(ending_player);
        self.modifiers.expire_end_of_turn(ending_player);
        for id in dropped_body_ids {
            self.granted_effect_bodies.remove(id);
        }
        // Phase 6: expire player-scoped flood-gate modifiers.
        self.modifiers.expire_player_end_of_turn(ending_player);
        // Prune source-independent floating mass modifiers whose turn-relative
        // window closes at this turn-end (G-CONTINUOUS-MASS-DP-DEBUFF).
        self.expire_floating_mass_modifiers(ending_player);
        self.expire_digixros_wildcards_at_end_of_turn(ending_player);
        // G-COST-REDUCE-ALLY-DIGIVOLVE: expire "For the turn" player-scoped
        // digivolve cost reducers installed by the ending player.
        self.player_digivolve_cost_reducers.retain(|r| {
            !(r.player == ending_player
                && r.expiry == crate::player_cost_reducer::PlayerCostReducerExpiry::EndOfTurn)
        });
        self.mark_until_condition_dirty();

        // EndOfOpponentsTurn: every non-ending-player observes the turn ending.
        // Fires after EndOfYourTurn has drained but before memory flip and rotation.
        for opp in self.opponents(ending_player) {
            self.enqueue_triggered(
                EffectTiming::EndOfOpponentsTurn,
                crate::selection::TriggerSource::PlayerBattleArea(opp),
            );
            let security_cards: Vec<_> = self
                .player(opp)
                .security
                .iter()
                .map(|card| card.handle())
                .collect();
            for card in security_cards {
                self.enqueue_triggered(
                    EffectTiming::EndOfOpponentsTurn,
                    crate::selection::TriggerSource::SecurityStackCard { player: opp, card },
                );
            }
        }
        // §18-1-2: pending processing due at the end of the opponent's turn
        // pools with the printed `EndOfOpponentsTurn` effects above: the
        // scheduled bodies and the provenance-keyed deletions ("At the end of
        // your opponent's turn, delete that token" — P-165 ShoeShoemon; only
        // entries whose controller is an opponent of `ending_player`).
        self.enqueue_scheduled_for_timing(EffectTiming::EndOfOpponentsTurn);
        self.enqueue_scheduled_for_timing(EffectTiming::EndOfOpponentsNextTurn);
        self.enqueue_provenance_deletions_at_opponents_turn_end(ending_player);
        self.drain_effect_queue();
        // The turn does not end until that pool has resolved: a decision in
        // it (a `TriggerOrder`, a body's own pick) holds the rotation here.
        if self.pending_selection.is_some() {
            self.pending_turn_boundary_resume =
                Some(TurnBoundaryResume::FinishTurnRotation { ending_player });
            return;
        }

        self.finish_turn_rotation(ending_player);
    }

    /// The rest of `rotate_turn_player` once the `EndOfOpponentsTurn` pool
    /// has resolved: memory floor, turn advance, seesaw flip, `begin_turn`.
    fn finish_turn_rotation(&mut self, ending_player: PlayerId) {
        if let Some(floor) = self.modifiers.end_turn_min_memory_floor(ending_player) {
            let floor = floor.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            if self.memory < floor {
                self.memory = floor;
            }
        }

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
            self.declare_step_limit_winner();
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

    /// §16-44-1 (Ver. 4.3): the <Engage> Digimon "may attack at the end of
    /// your turn"; once that attack is over the end of turn continues, with
    /// no further decision. Same for <Execute> (§16-38-1), <Vortex>
    /// (§16-33-1) and <Overclock> (§16-34-1). After an end-of-turn attack has
    /// ended (`finish_attack_after_end_of_attack_timing` sets
    /// `end_of_turn_attack_settle_pending`) and nothing is resolving any
    /// more, re-check the end-of-turn keywords: if another end-of-turn
    /// action is still available the `EndOfTurnAction` window stays open
    /// (PASS declines it); otherwise the turn rotates here instead of
    /// parking on a PASS-only step (G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP).
    ///
    /// Called at the attack's end and again from the `resolve_selection`
    /// tail, once its resolution depth has unwound, for an attack that ended
    /// inside a selection's resolution (e.g. its End of Attack pool parked,
    /// or an `<Overclock>` attack declared by the sacrifice pick).
    pub(crate) fn settle_end_of_turn_attack(&mut self) {
        if !self.end_of_turn_attack_settle_pending {
            return;
        }
        if self.pending_selection.is_some()
            || self.pending_attack.is_some()
            || !self.effect_queue.is_empty()
            || self.effect_drain_depth > 0
            || self.draining_deferred > 0
            || self.selection_resolution_depth > 0
        {
            return;
        }
        self.end_of_turn_attack_settle_pending = false;
        if self.game_over {
            return;
        }
        // An attack declared inside the `[End of Your Turn]` pool: the end of
        // turn continues where the pool left it (a queued <Overclock> attack,
        // a waiting keyword offer, then the swing-back check and the
        // rotation).
        if self.pending_end_turn_resume.is_some() || !self.parked_overclock_attacks.is_empty() {
            self.resume_pending_end_turn();
            return;
        }
        if self.current_phase != GamePhase::EndOfTurnAction {
            return;
        }
        if self.has_end_of_turn_keywords(self.turn_player()) {
            return;
        }
        self.pass_end_of_turn_action();
    }

    /// Perform a parked Breeding → Main advance once the selection that
    /// blocked it has resolved. No-op unless `enter_main_phase` parked while
    /// a selection was pending. Called from the `resolve_selection` tail.
    pub(crate) fn resume_pending_enter_main(&mut self) {
        if self.pending_selection.is_some() {
            return;
        }
        if !self.pending_enter_main_after_selection {
            return;
        }
        self.pending_enter_main_after_selection = false;
        if self.game_over {
            return;
        }
        self.enter_main_phase();
    }

    /// Resume the turn machine parked at a turn boundary by a decision in a
    /// §18-1-2 pool (`TurnBoundaryResume`). Called from the
    /// `resolve_selection` tail once nothing is pending.
    pub(crate) fn resume_pending_turn_boundary(&mut self) {
        if self.pending_selection.is_some() {
            return;
        }
        let Some(resume) = self.pending_turn_boundary_resume.take() else {
            return;
        };
        // The pool may still hold entries the resolved decision did not
        // reach (the resolver's drain stops at the next decision).
        if !self.effect_queue.is_empty() {
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                self.pending_turn_boundary_resume = Some(resume);
                return;
            }
        }
        if self.game_over {
            return;
        }
        match resume {
            TurnBoundaryResume::FinishTurnRotation { ending_player } => {
                self.finish_turn_rotation(ending_player)
            }
            TurnBoundaryResume::BeginTurnAfterUntilNextUnsuspend => {
                self.continue_begin_turn_after_until_next_unsuspend()
            }
        }
    }

    /// Continue the end of turn parked by its `[End of Your Turn]` pool
    /// (`pending_end_turn_resume`) once nothing in it is resolving. Called
    /// from the `resolve_selection` tail and when an end-of-turn attack ends
    /// (`settle_end_of_turn_attack`).
    ///
    /// An end-of-turn attack keyword's attack (§16-44-2, §16-38-2, §16-33-2,
    /// §16-34-2) is declared inside the pool; the pool's other entries
    /// resolve before its counter timing (§11-1-4; Q3869), and the end of turn
    /// waits for the attack to end (§6-6-2). Only one attack is in progress at
    /// a time: a keyword whose trigger activated while an attack was in
    /// progress offers its own attack now, after that attack has ended, in
    /// activation order (`deferred_end_of_turn_attack_offers`); a paid
    /// `<Overclock>` attack queued behind its cost's parked deletion
    /// (`parked_overclock_attacks`) activated before any of them and is
    /// declared first. Once every offer is made (declined, or its attack
    /// ended) the end of turn goes on: the §6-6-4 swing-back check, then the
    /// rotation.
    pub(crate) fn resume_pending_end_turn(&mut self) {
        if self.pending_selection.is_some() || self.pending_attack.is_some() {
            return;
        }
        self.resume_pending_overclock_attack();
        if self.pending_selection.is_some() || self.pending_attack.is_some() {
            return;
        }
        if self.pending_end_turn_resume.is_none() {
            return;
        }
        while !self.deferred_end_of_turn_attack_offers.is_empty() {
            let offer = self.deferred_end_of_turn_attack_offers.remove(0);
            if self.offer_deferred_end_of_turn_attack(offer) {
                return;
            }
        }
        let Some(resume) = self.pending_end_turn_resume.take() else {
            return;
        };
        self.continue_end_turn_after_effects(
            resume.ending_player,
            resume.memory_before_end_effects,
        );
    }

    #[doc(hidden)]
    pub fn park_end_turn_resume_for_test(
        &mut self,
        ending_player: PlayerId,
        memory_before_end_effects: i16,
    ) {
        self.pending_end_turn_resume = Some(EndTurnResume {
            ending_player,
            memory_before_end_effects,
        });
    }

    /// The end-of-turn attack keyword whose own decision is the pending
    /// selection, if any: an `<Engage>` / `<Execute>` / `<Vortex>` attack
    /// declaration (`ResumeFrame::AttackDeclarationDecision` carrying its
    /// end-of-turn keyword) or `<Overclock>`'s cost pick
    /// (`ResumeFrame::OverclockSelection`), offered inside the `[End of Your
    /// Turn]` pool (§16-44-2, §16-38-2, §16-33-2, §16-34-2). Read by the
    /// DCGO exam lowering and the test harness.
    pub fn pending_end_of_turn_keyword_decision(&self) -> Option<Keyword> {
        self.pending_selection.as_ref()?;
        match self
            .pending_selection_resume
            .as_ref()
            .map(|stack| stack.frames.as_slice())
        {
            Some([crate::resume::ResumeFrame::AttackDeclarationDecision(state)]) => {
                state.end_of_turn_keyword
            }
            Some([crate::resume::ResumeFrame::OverclockSelection(_)]) => Some(Keyword::Overclock),
            _ => None,
        }
    }

    /// True iff the given player has an action for the `EndOfTurnAction`
    /// window: a registered end-of-turn DNA route, or a Digimon with a
    /// `MayAttack` grant that can attack. (The end-of-turn attack keywords
    /// declare their attack inside the `[End of Your Turn]` pool instead.)
    pub fn has_end_of_turn_keywords(&self, player: PlayerId) -> bool {
        let Some(me) = self.players.get(player as usize) else {
            return false;
        };
        if self.has_registered_end_of_turn_dna_action(player) {
            return true;
        }
        for (i, perm) in me.battle_area.iter().enumerate() {
            // `Permanent::is_digimon` (Digimon | Dual), not the card-source
            // `CardSource::is_digimon` (Digimon only): a board-resident DUAL
            // card is a Digimon (G-ENGINE-DUAL-END-OF-TURN-WINDOW).
            if !perm.is_digimon(&self.card_data) {
                continue;
            }
            let handle = PermanentHandle {
                player,
                index: i as u8,
            };
            // MayAttack — normal can_attack. The end-of-turn attack keywords
            // (<Engage>, <Execute>, <Vortex>, <Overclock>) are not offered
            // here: their attack is declared inside the `[End of Your Turn]`
            // pool by their trigger bodies (§16-44-2, §16-38-2, §16-33-2,
            // §16-34-2; `keyword_effects::end_of_turn_keyword_attack_process`).
            if self.modifiers.has(handle, ModifierType::MayAttack)
                && self.can_attack(handle, /* vortex = */ false)
            {
                return true;
            }
        }
        false
    }

    /// True iff `player` has at least one legal `<Overclock>` cost candidate:
    /// one of their Tokens, or another Digimon accepted by the Overclock
    /// trait/predicate parameter.
    pub fn has_overclock_sacrifice(&self, player: PlayerId, overclock_index: usize) -> bool {
        !self
            .overclock_cost_action_ids(player, overclock_index)
            .is_empty()
    }

    fn overclock_cost_action_ids(&self, player: PlayerId, overclock_index: usize) -> Vec<u16> {
        use crate::action::space::encode_attack;

        let Some(me) = self.players.get(player as usize) else {
            return Vec::new();
        };
        me.battle_area
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                if i == overclock_index {
                    return None;
                }
                let candidate = PermanentHandle {
                    player,
                    index: i as u8,
                };
                self.is_legal_overclock_cost(player, overclock_index, candidate)
                    .then_some(encode_attack(0, i as u16))
            })
            .collect()
    }

    /// A legal `<Overclock>` cost: a specified candidate
    /// (`is_specified_overclock_cost`) whose deletion can be executed.
    /// §15-7-4 (Ver. 4.3): "If executing an optional processing condition's
    /// contents isn't possible, then the player can't choose to execute
    /// them" -- a Digimon that can't be deleted (a passive
    /// `CannotBeDestroyed*` restriction) is not offered. An optional or
    /// effect-based prevention (<Armor Purge>) stays offered: the deletion is
    /// executable and only replaced (§15-8-5-2); `run_overclock_selection_step`
    /// reads the outcome.
    fn is_legal_overclock_cost(
        &self,
        player: PlayerId,
        overclock_index: usize,
        candidate: PermanentHandle,
    ) -> bool {
        self.is_specified_overclock_cost(player, overclock_index, candidate)
            && !crate::replacement::deletion_passively_prevented(
                self,
                candidate,
                crate::replacement::ReplacementCause::Cost,
            )
    }

    fn is_specified_overclock_cost(
        &self,
        player: PlayerId,
        overclock_index: usize,
        candidate: PermanentHandle,
    ) -> bool {
        let Some(candidate_perm) = self
            .players
            .get(candidate.player as usize)
            .and_then(|p| p.battle_area.get(candidate.index as usize))
        else {
            return false;
        };
        if candidate.player != player || candidate.index as usize == overclock_index {
            return false;
        }

        let candidate_card = candidate_perm.top_card();
        let candidate_kind = candidate_card.card_kind(&self.card_data);
        let is_token = candidate_card.is_token || candidate_kind == CardKind::Token;
        if is_token {
            return true;
        }
        if !candidate_perm.is_digimon(&self.card_data) {
            return false;
        }

        let source = PermanentHandle {
            player,
            index: overclock_index as u8,
        };

        if let Some(matches_filter) = self.dsl_overclock_cost_filter_allows(source, candidate) {
            return matches_filter;
        }

        if let Some(required_trait) = self.printed_overclock_trait(source) {
            return candidate_card
                .traits(&self.card_data)
                .iter()
                .any(|trait_name| trait_name.eq_ignore_ascii_case(&required_trait));
        }

        true
    }

    fn dsl_overclock_cost_filter_allows(
        &self,
        source: PermanentHandle,
        candidate: PermanentHandle,
    ) -> Option<bool> {
        let source_card = self
            .players
            .get(source.player as usize)?
            .battle_area
            .get(source.index as usize)?
            .top_card();
        let card_id = source_card.card_id(&self.card_data).to_string();
        let effects = self.effects_for_card(&card_id, source_card.handle())?;
        let mut saw_filter = false;

        for effect in effects.iter() {
            if !effect.declarative || effect.granted_keyword != Some(Keyword::Overclock) {
                continue;
            }
            let rctx =
                EffectReadContext::new(self, effect.source_card, Some(source), source.player);
            if let Some(condition) = &effect.condition {
                if !condition(&rctx) {
                    continue;
                }
            }
            if let Some(filter) = effect.overclock_cost_filter.as_ref() {
                saw_filter = true;
                if filter(&rctx, candidate) {
                    return Some(true);
                }
            }
        }

        saw_filter.then_some(false)
    }

    fn printed_overclock_trait(&self, source: PermanentHandle) -> Option<String> {
        let source_card = self
            .players
            .get(source.player as usize)?
            .battle_area
            .get(source.index as usize)?
            .top_card();
        let card = self.card_data.get(source_card.data_index)?;

        for text in [&card.effect_text, &card.inherited_text, &card.security_text] {
            let Some(overclock_pos) = text.find("Overclock") else {
                continue;
            };
            let after = &text[overclock_pos..];
            let Some(open) = after.find('[') else {
                continue;
            };
            let after_open = &after[open + 1..];
            let Some(close) = after_open.find(']') else {
                continue;
            };
            let trait_name = after_open[..close].trim();
            if !trait_name.is_empty() {
                return Some(trait_name.to_string());
            }
        }

        None
    }

    /// `<Overclock>`'s processing (§16-34-1, §16-34-3), run by its
    /// `[End of Your Turn]` trigger body (§16-34-2) inside the end-of-turn
    /// pool: the optional processing condition "by deleting 1 of your Tokens
    /// or 1 of your other specified Digimon" as one optional pick over the
    /// legal candidates (`overclock_cost_action_ids`; PASS declines). A pick
    /// deletes it and, only when it was actually deleted (§15-7-1; a
    /// prevented deletion executes nothing, §15-7-2 / §15-8-5-2), the
    /// mandatory processing follows: `carrier` attacks the opponent without
    /// suspending (`run_overclock_selection_step`). The
    /// attack is declared inside the pool, so the pool's other entries
    /// resolve before its counter timing (§11-1-4). Resume-driven
    /// (`ResumeFrame::OverclockSelection`); the closures only mirror the
    /// frame. A declinable decision, so it marks the §18-3 loop watch.
    ///
    /// Returns whether the pick was installed (none on a finished game, with
    /// a decision already parked, or with no legal candidate).
    ///
    /// DCGO `CardEffectCommons.OverclockProcess`: `SelectPermanentEffect`
    /// with `canNoSelect: true`, then on a deletion `SelectAttackEffect` on
    /// the player only with `SetWithoutTap()` + `SetCanNotSelectNotAttack()`.
    pub(crate) fn install_overclock_selection(
        &mut self,
        carrier: PermanentHandle,
        source_kind: crate::enums::EffectSourceKind,
    ) -> bool {
        if self.game_over || self.pending_selection.is_some() {
            return false;
        }
        let player = carrier.player;
        let overclock_index = carrier.index as usize;
        let Some(overclock_perm) = self
            .players
            .get(player as usize)
            .and_then(|p| p.battle_area.get(overclock_index))
        else {
            return false;
        };
        if !overclock_perm.is_digimon(&self.card_data) {
            return false;
        }
        // Encoding uses the ATTACK target-half range, the convention of the
        // `install_field_selection` OwnField picks.
        let valid_action_ids = self.overclock_cost_action_ids(player, overclock_index);
        if valid_action_ids.is_empty() {
            return false;
        }

        let source_card = overclock_perm.top_card().handle();
        let opponent = self.next_clockwise(player);
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        let callback_state = OverclockSelectionState {
            player,
            source_card,
            opponent,
        };
        let decline_state = callback_state.clone();

        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: player,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: "<Overclock> - delete 1 of your Tokens or other specified Digimon to attack"
                .to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(carrier),
            source_kind,
            // Resume-driven (frame below); the closures mirror it.
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                game.run_overclock_selection_step(callback_state.clone(), action_id, false);
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                game.run_overclock_selection_step(decline_state.clone(), PASS, true);
            })),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::OverclockSelection(
                OverclockSelectionState {
                    player,
                    source_card,
                    opponent,
                },
            )],
        });
        self.mark_pending_selection_as_optional_decision();
        true
    }

    /// Offer an end-of-turn attack keyword's processing for `carrier`:
    /// `<Engage>` / `<Execute>` / `<Vortex>` as an optional attack
    /// declaration over the carrier's legal declarations
    /// (`action::mask::end_of_turn_keyword_attack_action_ids`), `<Overclock>`
    /// as its optional processing condition (`install_overclock_selection`).
    /// Returns whether a decision was installed (nothing is offered when no
    /// declaration / candidate is legal).
    pub(crate) fn offer_end_of_turn_keyword_attack(
        &mut self,
        carrier: PermanentHandle,
        keyword: Keyword,
        source_card: CardHandle,
        source_kind: crate::enums::EffectSourceKind,
    ) -> bool {
        let prompt = match keyword {
            Keyword::Overclock => return self.install_overclock_selection(carrier, source_kind),
            Keyword::Engage => "<Engage> - this Digimon may attack",
            Keyword::Execute => {
                "<Execute> - this Digimon may attack; at the end of the attack, it is deleted"
            }
            Keyword::Vortex => "<Vortex> - this Digimon may attack an opponent's Digimon",
            _ => return false,
        };
        let declarations =
            crate::action::mask::end_of_turn_keyword_attack_action_ids(self, carrier, keyword);
        let mut state = crate::resume::AttackDeclarationDecisionState::end_of_turn_keyword(
            carrier,
            source_card,
            declarations,
            keyword,
        );
        state.source_kind = source_kind;
        self.install_attack_declaration_decision(state, prompt)
    }

    /// Make a waiting end-of-turn keyword offer (see
    /// [`crate::game::DeferredEndOfTurnAttackOffer`]) now that the attack in
    /// progress has ended. The carrier is found by stack identity; nothing is
    /// offered once it has left the battle area or can no longer attack.
    /// Returns whether a decision was installed.
    fn offer_deferred_end_of_turn_attack(
        &mut self,
        offer: crate::game::DeferredEndOfTurnAttackOffer,
    ) -> bool {
        if self.game_over || offer.player != self.turn_player() {
            return false;
        }
        let Some(carrier) =
            self.resolve_digivolve_host_identity(offer.player, offer.carrier_identity)
        else {
            return false;
        };
        self.offer_end_of_turn_keyword_attack(
            carrier,
            offer.keyword,
            offer.source_card,
            offer.source_kind,
        )
    }

    pub(crate) fn run_overclock_selection_step(
        &mut self,
        state: OverclockSelectionState,
        action_id: u16,
        is_pass: bool,
    ) {
        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};

        if is_pass {
            self.declined_overclock_this_eot.insert(state.source_card);
            return;
        }

        let offset = action_id.saturating_sub(ATTACK_START);
        let sacrifice_index = (offset % TARGETS_PER_ATTACKER) as u8;
        let sacrifice_handle = PermanentHandle {
            player: state.player,
            index: sacrifice_index,
        };

        if self
            .players
            .get(state.player as usize)
            .and_then(|p| p.battle_area.get(sacrifice_index as usize))
            .is_none()
        {
            return;
        }

        // §16-34-3 / §15-7-1: the attack follows only if the condition "by
        // deleting 1 of your Tokens or 1 of your other specified Digimon" is
        // EXECUTED -- the chosen Digimon is actually deleted. A replacement
        // that prevents the deletion (§15-8-5-2: "it isn't deleted"; <Armor
        // Purge>, a "can't be deleted" grant) leaves the condition
        // unexecuted, and §15-7-2: nothing after it happens. The deletion's
        // outcome is read from a deletion receipt, which the batched flow
        // fills at commit and carries through a parked replacement decision
        // (`with_deletion_receipt`), so both the immediate and the parked
        // path read the same outcome. DCGO `OverclockProcess`:
        // `DeletePeremanentAndProcessAccordingToResult` → attack only
        // `if (isDeleted)`.
        let cost_identity = self.capture_digivolve_host_identity(sacrifice_handle);
        let receipt = self.open_deletion_receipt();

        let previous_cause = self.current_deletion_event_cause_override;
        self.current_deletion_event_cause_override =
            Some(crate::trigger_context::EventCause::Overclock);
        let _ = self.delete_permanents_batch_with_receipt(
            vec![sacrifice_handle],
            crate::replacement::ReplacementCause::Cost,
            Some(receipt),
        );
        self.current_deletion_event_cause_override = previous_cause;

        let parked = crate::game::ParkedOverclockAttack {
            player: state.player,
            source_card: state.source_card,
            opponent: state.opponent,
            cost_identity,
            cost_receipt: receipt,
        };
        if self.pending_selection.is_some() || self.pending_attack.is_some() {
            // A decision parked inside the deletion (an optional replacement
            // such as <Armor Purge>, or a deletion trigger), or an attack is
            // in progress (§11-2-4): the attack is queued behind them and
            // re-checks the receipt when it is declared
            // (`resume_pending_overclock_attack`). Queued, never overwritten:
            // each paid cost keeps its mandatory attack (§16-34-3). The pool's
            // other entries still resolve before it (the open §11-1-4
            // ordering half of G-ENGINE-OVERCLOCK-COST-REPLACEMENT-PARKS-AFTER-POOL).
            self.parked_overclock_attacks.push(parked);
            return;
        }

        if self.take_deletion_receipt_matching(receipt, state.player, cost_identity, None) {
            self.resume_overclock_attack(state.player, state.source_card, state.opponent);
        }
    }

    /// Declare the queued `<Overclock>` attacks (`parked_overclock_attacks`)
    /// in order, one at a time: nothing happens while a decision is pending
    /// or an attack is in progress (§11-2-4), and each entry attacks only if
    /// its cost was deleted (§16-34-3, §15-7-1, §15-8-5-2) and its carrier is
    /// still on the field. Stops at the first entry whose attack is now in
    /// progress (or parked a decision); the rest wait for that attack to end
    /// (`resume_pending_end_turn`). Called from the `resolve_selection` tail
    /// and `resume_pending_end_turn`.
    pub(crate) fn resume_pending_overclock_attack(&mut self) {
        while !self.game_over
            && self.pending_selection.is_none()
            && self.pending_attack.is_none()
            && !self.parked_overclock_attacks.is_empty()
        {
            let parked = self.parked_overclock_attacks.remove(0);
            if self.take_deletion_receipt_matching(
                parked.cost_receipt,
                parked.player,
                parked.cost_identity,
                None,
            ) {
                self.resume_overclock_attack(parked.player, parked.source_card, parked.opponent);
            }
        }
    }

    fn resume_overclock_attack(
        &mut self,
        player: PlayerId,
        source_card: CardHandle,
        opponent: PlayerId,
    ) {
        let Some(current_overclock_handle) = self.players.get(player as usize).and_then(|p| {
            p.battle_area.iter().enumerate().find_map(|(i, perm)| {
                (perm.top_card().card_index == source_card.0).then_some(PermanentHandle {
                    player,
                    index: i as u8,
                })
            })
        }) else {
            return;
        };
        if self.game_over {
            return;
        }

        self.begin_attack_overclock(current_overclock_handle, AttackTarget::Player(opponent));
    }

    /// Pass action: give the next player 3 memory, then end turn.
    ///
    /// Only forces memory to -3 if the passing player still had memory to give
    /// (i.e., memory >= 0). If memory is already negative — because an
    /// over-cost play pushed it there — that overflow is preserved and carried
    /// through the turn switch. Matches Python `game.pass_turn`.
    pub fn pass_turn(&mut self) {
        if self.game_over {
            return;
        }
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
        // G-ENGINE-SECURITY-STACK-END-OF-YOUR-TURN (BT25-039 Sirenmon):
        // turn-boundary `{Security}[End of Your Turn]` effects on cards living
        // in the ENDING player's persistent security stack. Mirrors
        // `rotate_turn_player`'s existing `EndOfOpponentsTurn` security scan;
        // `enqueue_from_security_stack_card` filters to `effect.security`
        // (scope: security) so battle-area-only observers are unaffected.
        let security_cards: Vec<_> = self
            .player(player)
            .security
            .iter()
            .map(|card| card.handle())
            .collect();
        for card in security_cards {
            self.enqueue_triggered(
                EffectTiming::EndOfYourTurn,
                crate::selection::TriggerSource::SecurityStackCard { player, card },
            );
        }
        // G-ENGINE-TRASH-ZONE-END-OF-YOUR-TURN (P-250 Ogremon (X Antibody)):
        // `{Trash}[End of Your Turn]` effects on cards in the ENDING player's
        // trash join the same simultaneous end-of-turn pool. Only clauses
        // printed with the `{Trash}` zone scope (`scope: trash`) are scanned,
        // so a dead card's ordinary field effects stay dormant (DCGO
        // `IsExistOnTrashTrigger`; Q&A: a {Trash} effect triggers only while
        // its card is in the trash).
        self.enqueue_from_player_trash(EffectTiming::EndOfYourTurn, player, None);
        // §18-1-2: pending processing due at the end of this turn
        // (`ScheduledEffect`s for `EndOfYourTurn` / `EndOfYourNextTurn`)
        // joins the printed `[End of Your Turn]` effects in ONE simultaneous
        // pool, ordered by the turn player (§15-4-3-5-1) -- not run after
        // them. G-ENGINE-PENDING-PROCESSING-OUTSIDE-TRIGGER-POOL.
        //
        // Note: `EffectTiming` does not have a unified `EndOfTurn`
        // variant — the enum collapses to `EndOfYourTurn` (active
        // player) and `EndOfOpponentsTurn` (inactive players, fired
        // from `rotate_turn_player`). Both peer pools are built
        // separately at their respective observer-fire sites.
        // PUPPETS-G003: the provenance-keyed turn-end deletions ("At turn
        // end, delete the Digimon this effect played") are pending processing
        // of this timing too, so they join the same pool.
        self.enqueue_scheduled_for_timing(EffectTiming::EndOfYourTurn);
        self.enqueue_scheduled_for_timing(EffectTiming::EndOfYourNextTurn);
        self.enqueue_provenance_deletions_at_turn_end();
        self.drain_effect_queue();
    }

    /// Phase 8 Task 3: fire and trash every Delayed Option (across all
    /// players' battle_areas) whose scheduled `trash_on_turn` matches
    /// `ending_turn`. Called at the top of `end_turn` before the regular
    /// `EndOfYourTurn` fan-out.
    ///
    /// The scan is NOT filtered by owner: a Delay played by P0 during P1's
    /// turn (via a Counter window, once Task 9 lands) parks on P0's
    /// battle_area but is scheduled to fire at P1's turn end. The only
    /// property that matters is `trash_on_turn == ending_turn`.
    ///
    /// For each delayed permanent we enqueue `DelayEffect`, drain, then
    /// route the trash through `delete_permanent_with_cause` with
    /// `ReplacementCause::Cost` so the Phase 7 `WhenWouldLeaveBattleArea` /
    /// `WhenWouldBeDeleted` replacement windows get a chance to intercept.
    ///
    /// Handles shift: after each delete, we re-scan from scratch. A
    /// `cancelled_keys` skip-set keyed on the stable `(owner, turn_played)`
    /// pair ensures a cancel-always replacement doesn't infinite-loop on
    /// the same permanent, and — crucially — doesn't prevent sibling
    /// delayed permanents from firing their `DelayEffect` bodies.
    ///
    /// v1 constraint: if a `DelayEffect` body installs a `PendingSelection`,
    /// the scan returns early with the selection parked on the Game. This
    /// matches the existing convention for triggered-effect dispatch during
    /// `end_turn` (see `fire_end_of_your_turn`). Cards that want to query
    /// the opponent from their `DelayEffect` body are currently
    /// unsupported; file a gap in `docs/RUST_ENGINE_GAPS.md` if a printed
    /// card needs this shape.
    pub(crate) fn resolve_delayed_options(&mut self, ending_turn: u16, ending_player: PlayerId) {
        self.resolve_delayed_options_matching(
            ending_turn,
            &[DelayTrigger::EndOfThisTurn, DelayTrigger::EndOfYourNextTurn],
            DelayedOptionLifecycleResumeKind::EndTurn { ending_player },
            None,
        );
    }

    pub(crate) fn resolve_start_delayed_options(&mut self, starting_turn: u16) {
        self.resolve_delayed_options_matching(
            starting_turn,
            &[DelayTrigger::StartOfYourNextTurn],
            DelayedOptionLifecycleResumeKind::StartTurn,
            None,
        );
    }

    fn resolve_delayed_options_matching(
        &mut self,
        turn: u16,
        triggers: &[DelayTrigger],
        resume_kind: DelayedOptionLifecycleResumeKind,
        initial_skip_key: Option<(PlayerId, u16)>,
    ) {
        use crate::permanent::OptionState;
        use std::collections::HashSet;

        // Stable key: `(owner, bottom_card_index)`. The bottom card of a
        // Delayed permanent is the Option itself (no digivolution stack on
        // an Option), and `CardSource::card_index` is a per-game unique id
        // that survives permanent-index shifts and sibling deletes. We
        // can't key on `(owner, turn_played)` alone — two Delay Options
        // played on the same turn share `turn_played`.
        let mut cancelled_keys: HashSet<(PlayerId, u16)> = HashSet::new();
        if let Some(key) = initial_skip_key {
            cancelled_keys.insert(key);
        }

        loop {
            // Find the next matching Delayed permanent across ALL players'
            // battle_areas that isn't in the skip-set.
            let next: Option<PermanentHandle> = (0..self.players.len()).find_map(|pid| {
                let pid = pid as PlayerId;
                self.player(pid)
                    .battle_area
                    .iter()
                    .enumerate()
                    .find_map(|(i, perm)| {
                        if let OptionState::Delayed {
                            owner: _,
                            trash_on_turn: t,
                            trigger: delayed_trigger,
                            placed_on_turn: _,
                        } = perm.option_state
                        {
                            if t == turn && triggers.contains(&delayed_trigger) {
                                let bottom = perm.card_sources.first()?;
                                let key = (pid, bottom.card_index);
                                if !cancelled_keys.contains(&key) {
                                    return Some(PermanentHandle {
                                        player: pid,
                                        index: i as u8,
                                    });
                                }
                            }
                        }
                        None
                    })
            });

            let Some(handle) = next else { break };

            // Capture the stable key before the permanent potentially moves.
            let key = {
                let perm = &self.player(handle.player).battle_area[handle.index as usize];
                (handle.player, perm.card_sources[0].card_index)
            };

            // DCGO gates a scheduled `<Delay>` on `CanUseCondition` BEFORE the
            // window opens at all — LM_032.cs:137-143 requires
            // `card.Owner.Enemy.GetBattleAreaDigimons().Count >= 1` for the
            // printed "If your opponent has a Digimon, <Delay>". When that
            // printed gate is FALSE the Delay is simply not activatable this
            // turn, and §16-17-1 makes the trash of the carrier the
            // ACTIVATION ("by trashing that card, the effect specified in
            // <Delay> will activate") — so an un-activated Delay must not pay
            // it. The carrier STAYS in the battle area, where §16-17-1 keeps
            // the Delay available, and its window moves to the next one.
            //
            // Before this guard the scan enqueued a body that the queue's
            // clause-condition filter then dropped, and deleted the carrier
            // anyway: a free loss of the card and of every later window.
            if !self.scheduled_delay_is_activatable(handle, triggers) {
                self.reschedule_delayed_option(key);
                cancelled_keys.insert(key);
                continue;
            }

            // Scope the decline flag to THIS resolution: a key left over from a
            // previous Option would otherwise be read as a decline here.
            self.declined_delay_option = None;
            self.enqueue_triggered(
                EffectTiming::DelayEffect,
                crate::selection::TriggerSource::Permanent(handle),
            );
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                    turn,
                    kind: resume_kind,
                    pending_delete_key: Some(key),
                    skip_key: None,
                });
                return;
            }

            // Re-find the permanent by stable key — the DelayEffect body
            // may have deleted it, shifted indices via other mutations, or
            // (rarely) moved it. If it's gone, nothing to trash.
            if self.take_declined_delay_reschedule(key) {
                // Declined: cost unpaid, carrier stays, window moved on. Skip
                // it for the rest of this scan so we advance to siblings.
                cancelled_keys.insert(key);
                continue;
            }
            if let Some(current_handle) = self.find_delayed_permanent_by_key(key, turn, triggers) {
                self.delete_permanent_with_cause(
                    current_handle,
                    crate::replacement::ReplacementCause::Cost,
                );
            }
            if self.pending_selection.is_some() {
                self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                    turn,
                    kind: resume_kind,
                    pending_delete_key: None,
                    skip_key: Some(key),
                });
                return;
            }

            // If a replacement window cancelled the delete, the permanent
            // is still on the field with the same key — add it to the
            // skip-set so we move on to siblings instead of looping on it
            // forever. A cancel-always replacement naturally re-fires at
            // subsequent turn ends when this scan runs again.
            if self
                .find_delayed_permanent_by_key(key, turn, triggers)
                .is_some()
            {
                cancelled_keys.insert(key);
            }
        }
    }

    /// If the just-resolved turn-scheduled `<Delay>` at `key` had its §16-17-2
    /// cost DECLINED, leave the carrier alone and move its window forward.
    ///
    /// Returns `true` when the caller must NOT delete the carrier.
    ///
    /// Rescheduling is not optional politeness: the scan matches on
    /// `trash_on_turn == turn`, so an Option left at its spent turn number
    /// would sit on the field forever, never offered again. §16-17-1 keeps the
    /// Delay available "while a card with this effect is in the battle area",
    /// and these are recurring printed windows ([Start of Your Turn],
    /// [End of Your Turn]), so the next window is the controller's next turn —
    /// exactly what `compute_delay_trash_turn` yields from here.
    fn take_declined_delay_reschedule(&mut self, key: (PlayerId, u16)) -> bool {
        if self.declined_delay_option != Some(key) {
            return false;
        }
        self.declined_delay_option = None;
        self.reschedule_delayed_option(key);
        true
    }

    /// Move a surviving turn-scheduled `<Delay>` carrier's window forward to
    /// its next printed occurrence.
    ///
    /// Shared by the two ways a scheduled window can close without the Delay
    /// activating: the controller DECLINED its §16-17-2 cost, or the clause's
    /// printed condition was FALSE so the window never opened
    /// (DCGO `CanUseCondition`). Both leave the carrier in the battle area,
    /// and the scan matches on `trash_on_turn == turn`, so a carrier left at
    /// its spent turn number would sit on the field forever, never offered
    /// again. A no-op when the key names no live permanent.
    fn reschedule_delayed_option(&mut self, key: (PlayerId, u16)) {
        let owner = key.0;
        let slot = self
            .player(owner)
            .battle_area
            .iter()
            .position(|perm| perm.card_sources.first().map(|c| c.card_index) == Some(key.1));
        if let Some(slot) = slot {
            let trigger = match self.player(owner).battle_area[slot].option_state {
                crate::permanent::OptionState::Delayed { trigger, .. } => Some(trigger),
                _ => None,
            };
            if let Some(trigger) = trigger {
                let next_turn = self.compute_delay_trash_turn(owner, trigger);
                if let crate::permanent::OptionState::Delayed { trash_on_turn, .. } =
                    &mut self.player_mut(owner).battle_area[slot].option_state
                {
                    *trash_on_turn = next_turn;
                }
            }
        }
    }

    /// DCGO `CanUseCondition` for a turn-scheduled `<Delay>`: does the carrier
    /// at `handle` carry a `DelayEffect` clause whose clause-level condition
    /// currently PASSES?
    ///
    /// Returns `false` only when every `DelayEffect` clause matching this
    /// scan's `triggers` is gated by a condition that is currently false —
    /// i.e. the window does not open, so the §16-17-1 activation (and with it
    /// the trash of the carrier) must not happen.
    ///
    /// Conservative in every ambiguous case (carrier gone, effects missing, no
    /// clause condition, an unconditional sibling clause): returns `true` and
    /// lets the existing enqueue/drain path decide, so this guard can only
    /// SUPPRESS a trash the rules do not license, never introduce one. A
    /// marker-only `<Delay>` (empty process, ST23-15 / ST24-15) has no
    /// condition and is therefore unaffected.
    fn scheduled_delay_is_activatable(
        &mut self,
        handle: PermanentHandle,
        triggers: &[DelayTrigger],
    ) -> bool {
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return true;
        };
        if perm.card_sources.is_empty() {
            return true;
        }
        let top = perm.top_card();
        let card_id = top.card_id(&self.card_data).to_string();
        let source_card = top.handle();
        let source_kind =
            crate::enums::EffectSourceKind::for_card_kind(top.card_kind(&self.card_data));
        let controller = handle.player;

        // Owned clone, so the condition closures can be evaluated against
        // `&mut self` — same idiom as `non_firing_queued_effect_indices_for`.
        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return true;
        };

        let mut saw_matching_delay = false;
        for effect in effects.iter() {
            if effect.timing != EffectTiming::DelayEffect {
                continue;
            }
            if let Some(trigger) = effect.delay_trigger {
                if !triggers.contains(&trigger) {
                    continue;
                }
            }
            saw_matching_delay = true;
            let Some(condition) = &effect.condition else {
                return true; // unconditional clause — always activatable
            };
            let ctx = crate::effect_context::EffectContext::new_with_source_kind(
                self,
                source_card,
                Some(handle),
                source_kind,
                controller,
            );
            if condition(&ctx.as_read()) {
                return true;
            }
        }
        // No `DelayEffect` clause matched at all: leave the legacy path alone.
        !saw_matching_delay
    }

    pub(crate) fn park_delayed_option_lifecycle(&mut self, resume: DelayedOptionLifecycleResume) {
        if self.pending_delayed_option_lifecycle.is_some() {
            self.pending_delayed_option_lifecycle_stack.push(resume);
        } else {
            self.pending_delayed_option_lifecycle = Some(resume);
        }
    }

    pub(crate) fn resume_pending_delayed_option_lifecycle(&mut self) {
        if self.pending_selection.is_some() {
            return;
        }

        loop {
            let Some(mut resume) = self
                .pending_delayed_option_lifecycle
                .take()
                .or_else(|| self.pending_delayed_option_lifecycle_stack.pop())
            else {
                return;
            };

            if let Some(key) = resume.pending_delete_key.take() {
                let current_handle = match resume.kind {
                    DelayedOptionLifecycleResumeKind::Event { timing } => {
                        self.find_event_delayed_permanent_by_key(key, timing)
                    }
                    _ => {
                        let triggers = Self::delay_lifecycle_triggers(resume.kind);
                        self.find_delayed_permanent_by_key(key, resume.turn, triggers)
                    }
                };
                let declined = self.take_declined_delay_reschedule(key);
                if let (false, Some(current_handle)) = (declined, current_handle) {
                    self.delete_permanent_with_cause(
                        current_handle,
                        crate::replacement::ReplacementCause::Cost,
                    );
                }
                if declined {
                    resume.skip_key = Some(key);
                }

                if self.pending_selection.is_some() {
                    resume.skip_key = Some(key);
                    self.pending_delayed_option_lifecycle = Some(resume);
                    return;
                }

                match resume.kind {
                    DelayedOptionLifecycleResumeKind::Event { .. }
                    | DelayedOptionLifecycleResumeKind::MainPhaseActivation
                    | DelayedOptionLifecycleResumeKind::UnsuspendPhase => continue,
                    _ => {
                        let triggers = Self::delay_lifecycle_triggers(resume.kind);
                        if self
                            .find_delayed_permanent_by_key(key, resume.turn, triggers)
                            .is_some()
                        {
                            resume.skip_key = Some(key);
                        }
                    }
                }
            }

            match resume.kind {
                DelayedOptionLifecycleResumeKind::StartTurn => {
                    self.resolve_delayed_options_matching(
                        resume.turn,
                        &[DelayTrigger::StartOfYourNextTurn],
                        resume.kind,
                        resume.skip_key,
                    );
                    if self.pending_selection.is_some() {
                        return;
                    }
                    self.continue_begin_turn_after_start_delays();
                    return;
                }
                DelayedOptionLifecycleResumeKind::EndTurn { ending_player } => {
                    self.resolve_delayed_options_matching(
                        resume.turn,
                        &[DelayTrigger::EndOfThisTurn, DelayTrigger::EndOfYourNextTurn],
                        resume.kind,
                        resume.skip_key,
                    );
                    if self.pending_selection.is_some() {
                        return;
                    }
                    self.continue_end_turn_after_delays(ending_player);
                    return;
                }
                DelayedOptionLifecycleResumeKind::UnsuspendPhase => {
                    self.reevaluate_until_condition_modifiers_if_dirty();
                    if self.game_over {
                        return;
                    }
                    self.continue_begin_turn_after_unsuspend_phase();
                    return;
                }
                DelayedOptionLifecycleResumeKind::Event { .. }
                | DelayedOptionLifecycleResumeKind::MainPhaseActivation => continue,
            }
        }
    }

    fn delay_lifecycle_triggers(kind: DelayedOptionLifecycleResumeKind) -> &'static [DelayTrigger] {
        match kind {
            DelayedOptionLifecycleResumeKind::StartTurn => &[DelayTrigger::StartOfYourNextTurn],
            DelayedOptionLifecycleResumeKind::EndTurn { .. } => {
                &[DelayTrigger::EndOfThisTurn, DelayTrigger::EndOfYourNextTurn]
            }
            DelayedOptionLifecycleResumeKind::Event { .. }
            | DelayedOptionLifecycleResumeKind::UnsuspendPhase => &[],
            DelayedOptionLifecycleResumeKind::MainPhaseActivation => {
                &[DelayTrigger::MainPhaseActivated]
            }
        }
    }

    fn find_event_delayed_permanent_by_key(
        &self,
        key: (PlayerId, u16),
        timing: EffectTiming,
    ) -> Option<PermanentHandle> {
        use crate::permanent::OptionState;

        let (owner, card_index) = key;
        let me = self.players.get(owner as usize)?;
        for (i, perm) in me.battle_area.iter().enumerate() {
            let Some(bottom) = perm.card_sources.first() else {
                continue;
            };
            if bottom.card_index != card_index {
                continue;
            }
            if let OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(event_timing),
                ..
            } = perm.option_state
            {
                if event_timing == timing {
                    return Some(PermanentHandle {
                        player: owner,
                        index: i as u8,
                    });
                }
            }
        }
        None
    }

    /// Scan all players' battle_areas for a Delayed permanent matching the
    /// given `(owner, bottom_card_index)` key and `trash_on_turn`. Returns
    /// the current `PermanentHandle` (indices may have shifted since the
    /// key was captured). Phase 8 Task 3 helper for
    /// `resolve_delayed_options`; also used by the standard `<Delay>`
    /// `[Main]`-phase activation path in `option_lifecycle.rs`.
    pub(crate) fn find_delayed_permanent_by_key(
        &self,
        key: (PlayerId, u16),
        turn: u16,
        triggers: &[DelayTrigger],
    ) -> Option<PermanentHandle> {
        use crate::permanent::OptionState;

        let (owner, card_index) = key;
        let me = self.players.get(owner as usize)?;
        for (i, perm) in me.battle_area.iter().enumerate() {
            let Some(bottom) = perm.card_sources.first() else {
                continue;
            };
            if bottom.card_index != card_index {
                continue;
            }
            if let OptionState::Delayed {
                trash_on_turn: t,
                trigger: delayed_trigger,
                placed_on_turn: _,
                ..
            } = perm.option_state
            {
                if t == turn && triggers.contains(&delayed_trigger) {
                    return Some(PermanentHandle {
                        player: owner,
                        index: i as u8,
                    });
                }
            }
        }
        None
    }
}
