//! Turn lifecycle and phase transitions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! Callers still invoke `game.end_turn()`, `game.pass_turn()`, `game.activate_overclock(...)`.
//! The split exists because the forthcoming observer-firing gap work (Phase A
//! of the Medusamon plan) will land in this file — `StartOfYourTurn`,
//! `StartOfYourMainPhase`, and `OnLoseSecurity` fan-out all live at phase
//! boundaries.

use crate::card_source::CardHandle;
use crate::effect_context::EffectReadContext;
use crate::enums::{
    CardKind, DelayTrigger, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId, SkipDraw,
};
use crate::game::{
    DelayedOptionLifecycleResume, DelayedOptionLifecycleResumeKind, Game, OverclockError,
};
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

        crate::scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::UntilNextUnsuspend);

        // Reset per-turn state
        self.player_mut(tp).new_turn();

        // Unsuspend phase
        self.current_phase = GamePhase::Unsuspend;
        self.player_mut(tp).unsuspend_all();

        // Phase 9 Task 7: <Reboot> consumer. "At the start of your
        // opponent's unsuspend phase, unsuspend this Digimon." Scan every
        // opponent's battle area for Digimon with the Reboot keyword
        // (printed or granted) and unsuspend them. The `CannotUnsuspend`
        // modifier gates the scan — Reboot is effect-driven unsuspension
        // and respects the suspend-lock.
        //
        // Note: like the standard turn-start bulk unsuspend
        // (`unsuspend_all`), Reboot-driven unsuspension directly mutates
        // `is_suspended` without firing `OnUnsuspend` observers.
        // Phase-start unsuspension is not a trigger-carrying event per
        // the Digimon TCG rules.
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
                if self.modifiers.has(h, ModifierType::CannotUnsuspend) {
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
                perm.is_suspended = false;
            }
        }

        // Draw phase
        self.current_phase = GamePhase::Draw;
        let should_skip_draw = match self.rules.skip_first_draw {
            // "The first player of the game skips their first draw." Turn 1
            // uniquely identifies that moment — whoever the coin flip sat at
            // `turn_order[0]` is the turn player here. Don't hardcode tp == 0.
            SkipDraw::FirstPlayerOnly => self.turn_count == 1,
            SkipDraw::AllRound1 => self.turn_count <= self.rules.player_count as u16,
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
        if perm.level(&self.card_data).unwrap_or(0) >= 3
            && player.battle_area.len() < self.rules.field_slots as usize
        {
            return true;
        }
        false
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
        self.enqueue_triggered(
            EffectTiming::StartOfYourMainPhase,
            crate::selection::TriggerSource::PlayerBreedingArea(tp),
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
        // Phase 6: expire player-scoped flood-gate modifiers.
        self.modifiers.expire_player_end_of_turn(ending_player);

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
        self.drain_effect_queue();

        // Phase 2f4 Task 2: drain ScheduledEffect entries scheduled for
        // EndOfOpponentsTurn after the printed-observer fan-out.
        crate::scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::EndOfOpponentsTurn);
        crate::scheduled_effects::fire_scheduled_for_timing(
            self,
            EffectTiming::EndOfOpponentsNextTurn,
        );

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
        if self.has_registered_end_of_turn_dna_action(player) {
            return true;
        }
        for (i, perm) in me.battle_area.iter().enumerate() {
            if !perm.top_card().is_digimon(&self.card_data) {
                continue;
            }
            let handle = PermanentHandle {
                player,
                index: i as u8,
            };
            // Vortex — matches Python `perm.can_attack(is_vortex=True)`.
            if self.has_keyword(handle, Keyword::Vortex)
                && self.can_attack(handle, /* vortex = */ true)
            {
                return true;
            }
            // Overclock — needs at least one other sacrificeable permanent.
            if self.has_keyword(handle, Keyword::Overclock)
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

    fn is_legal_overclock_cost(
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

        for effect in effects {
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
            if let Some(filter) = effect.overclock_cost_filter {
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
    pub fn activate_overclock(&mut self, overclock_index: usize) -> Result<(), OverclockError> {
        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};

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

        if !self.has_keyword(overclock_handle, Keyword::Overclock) {
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
        let valid_action_ids = self.overclock_cost_action_ids(player, overclock_index);
        debug_assert!(
            !valid_action_ids.is_empty(),
            "has_overclock_sacrifice promised ≥1"
        );

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
            source_kind: crate::enums::EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let sacrifice_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let sacrifice_handle = PermanentHandle {
                    player,
                    index: sacrifice_index,
                };

                // Delete the sacrifice (firing OnDeletion triggers). Replacement
                // windows still see this as a cost; observer payloads see the
                // keyword-specific Overclock cause.
                let previous_cause = game.current_deletion_event_cause_override;
                game.current_deletion_event_cause_override =
                    Some(crate::trigger_context::EventCause::Overclock);
                game.delete_permanent_with_cause(
                    sacrifice_handle,
                    crate::replacement::ReplacementCause::Cost,
                );
                game.current_deletion_event_cause_override = previous_cause;

                if game.pending_selection.is_some() {
                    game.pending_overclock_attack = Some((player, source_card, opponent));
                    return;
                }

                game.resume_overclock_attack(player, source_card, opponent);
            }),
            on_decline: None,
        });

        Ok(())
    }

    pub(crate) fn resume_pending_overclock_attack(&mut self) {
        if self.pending_selection.is_some() {
            return;
        }
        let Some((player, source_card, opponent)) = self.pending_overclock_attack.take() else {
            return;
        };
        self.resume_overclock_attack(player, source_card, opponent);
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

        // Phase 2f4 Task 2: drain ScheduledEffect entries scheduled for
        // EndOfYourTurn here. Printed observers fire first (above);
        // scheduled bodies fire AFTER — they're the turn-end
        // housekeeping that printed text scheduled when its own observer
        // ran earlier.
        //
        // Note: `EffectTiming` does not have a unified `EndOfTurn`
        // variant — the enum collapses to `EndOfYourTurn` (active
        // player) and `EndOfOpponentsTurn` (inactive players, fired
        // from `rotate_turn_player`). Both peer drains are wired
        // separately at their respective observer-fire sites.
        crate::scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::EndOfYourTurn);
        crate::scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::EndOfYourNextTurn);
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
                if let Some(current_handle) = current_handle {
                    self.delete_permanent_with_cause(
                        current_handle,
                        crate::replacement::ReplacementCause::Cost,
                    );
                }

                if self.pending_selection.is_some() {
                    resume.skip_key = Some(key);
                    self.pending_delayed_option_lifecycle = Some(resume);
                    return;
                }

                match resume.kind {
                    DelayedOptionLifecycleResumeKind::Event { .. } => continue,
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
                DelayedOptionLifecycleResumeKind::Event { .. } => continue,
            }
        }
    }

    fn delay_lifecycle_triggers(kind: DelayedOptionLifecycleResumeKind) -> &'static [DelayTrigger] {
        match kind {
            DelayedOptionLifecycleResumeKind::StartTurn => &[DelayTrigger::StartOfYourNextTurn],
            DelayedOptionLifecycleResumeKind::EndTurn { .. } => {
                &[DelayTrigger::EndOfThisTurn, DelayTrigger::EndOfYourNextTurn]
            }
            DelayedOptionLifecycleResumeKind::Event { .. } => &[],
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
    /// `resolve_delayed_options`.
    fn find_delayed_permanent_by_key(
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
