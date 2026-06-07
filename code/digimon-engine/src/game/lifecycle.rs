//! Game-over / elimination / play-order (Tier 1).

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
    /// End the turn if memory has crossed to the opponent's side.
    /// Call this after a batch of effects resolves (not synchronously inside
    /// `pay_memory`, which would starve effects of their turn).
    pub fn check_turn_end(&mut self) {
        if self.pending_selection.is_some() {
            return;
        }
        if self.memory < 0 && !self.game_over {
            self.end_turn();
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
            self.terminal_outcome_reason = Some(TerminalOutcomeReason::EngineDeclared);
            self.current_phase = GamePhase::GameOver;
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::GameOver {
                seq,
                winner: self.winner,
                reason: TerminalOutcomeReason::EngineDeclared,
            });
        }

        // Adjust turn_player_idx if needed
        if self.turn_player_idx >= self.turn_order.len() {
            self.turn_player_idx = 0;
        }
    }

    /// Declare a winner (e.g., after a direct attack on a player with 0 security).
    pub fn declare_winner(&mut self, winner_id: PlayerId) {
        self.declare_winner_with_reason(winner_id, TerminalOutcomeReason::EngineDeclared);
    }

    /// Choose a deterministic winner for non-natural termination.
    ///
    /// Training games should always produce a winner. Timeout ranking uses
    /// visible, stable game-state advantages, then falls back to player order
    /// so exact ties are still deterministic.
    pub fn step_limit_tiebreaker_winner(&self) -> PlayerId {
        fn score(
            game: &Game,
            player: PlayerId,
        ) -> (usize, i32, usize, usize, std::cmp::Reverse<PlayerId>) {
            let p = game.player(player);
            (
                p.security.len(),
                p.total_field_dp(&game.card_data),
                p.deck.len(),
                p.hand.len(),
                std::cmp::Reverse(player),
            )
        }

        self.turn_order
            .iter()
            .copied()
            .max_by_key(|&player| score(self, player))
            .unwrap_or(0)
    }

    /// End the game by step/turn limit while still assigning a winner.
    pub fn declare_step_limit_winner(&mut self) {
        let winner = self.step_limit_tiebreaker_winner();
        self.declare_winner_with_reason(winner, TerminalOutcomeReason::StepLimit);
    }

    /// Concede the game on behalf of `player_id`. The opponent is declared the
    /// winner with `TerminalOutcomeReason::Concede`. Always-legal at every
    /// agent decision point — the action mask reports action `93` as legal in
    /// every phase. Safe to call mid-selection: any pending selection is
    /// cleared, the effect queue is dropped, and any in-progress combat /
    /// attack-timing state is short-circuited by the terminal phase change.
    ///
    /// Event ordering: a `GameEvent::Concede { player }` event is pushed
    /// **before** the `GameOver` event so listeners can observe the concede
    /// before the terminal-outcome notification. This mirrors the
    /// surrender-event-before-declare_winner pattern used by the legacy
    /// Python engine (see `CLAUDE.md` working rule #16).
    ///
    /// No-op if the game is already over.
    pub fn concede(&mut self, player_id: PlayerId) {
        if self.game_over {
            return;
        }
        // Clear pending selection and effect queue so the game terminates
        // cleanly even when concede fires mid-selection or with queued
        // triggered effects waiting to resolve.
        self.pending_selection = None;
        self.effect_queue.clear();
        // Emit the Concede event before declare_winner so its seq is
        // strictly less than the GameOver event's seq.
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Concede {
            seq,
            player: player_id,
        });
        // Resolve the winner: opponent of the conceder. In 2-player play
        // there is exactly one opponent; in multiplayer we fall back to the
        // first non-eliminated opponent in turn order.
        let opponents = self.opponents(player_id);
        let winner_id = opponents.first().copied().unwrap_or(player_id);
        self.declare_winner_with_reason(winner_id, TerminalOutcomeReason::Concede);
    }

    /// Install a `SelectionKind::PlayOrder` prompt with `loser_id` as the
    /// chooser. The engine enters `GamePhase::SelectPlayOrder`; the action
    /// mask reports actions `PLAY_FIRST` (94) and `PLAY_SECOND` (95) as legal
    /// for `loser_id` only. On resolution, the callback writes the chosen
    /// `PlayOrder` to `self.last_play_order_choice` and the standard
    /// selection unwind restores `previous_phase` (typically `GameOver`).
    ///
    /// The wrapper (Python `MatchEnv` for BO3 match training) is expected to
    /// call this method between games of a match, then read
    /// `self.last_play_order_choice` once the selection resolves to decide
    /// which side plays first in the next game.
    ///
    /// Any existing `pending_selection` is dropped — when a game terminates
    /// via a win condition mid-effect (security depleted by a triggered
    /// effect, deck-out during a chain), the engine can leave a
    /// `pending_selection` installed; the BO3 wrapper calls this method
    /// after termination, and we just discard the stale prompt since the
    /// game it belonged to is over.
    pub fn request_play_order_selection(&mut self, loser_id: PlayerId) {
        self.pending_selection = None;
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectPlayOrder;
        self.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::PlayOrder,
            selecting_player: loser_id,
            previous_phase,
            valid_action_ids: vec![
                crate::action::space::PLAY_FIRST,
                crate::action::space::PLAY_SECOND,
            ],
            is_optional: false,
            prompt: "Choose play order for the next game".to_string(),
            effect_choices: None,
            // No real card source — this prompt is driven by the BO3 match
            // rules, not by a card effect. We use a sentinel `CardHandle(0)`
            // (CardHandle is a `pub u16` newtype) and `EffectSourceKind::Rule`.
            source_card: crate::card_source::CardHandle(0),
            source_permanent: None,
            source_kind: crate::enums::EffectSourceKind::Rule,
            callback: Box::new(|game: &mut Game, action_id: u16| {
                let picked = if action_id == crate::action::space::PLAY_FIRST {
                    crate::selection::PlayOrder::First
                } else {
                    // PLAY_SECOND (95) is the only other valid id; the
                    // selection installation gates this through valid_action_ids.
                    crate::selection::PlayOrder::Second
                };
                game.last_play_order_choice = Some(picked);
            }),
            on_decline: None,
        });
    }

    /// Convenience entry point that resolves a pending play-order selection
    /// directly (without routing through the action-id interface). Useful for
    /// programmatic testing and for wrapper code that already knows the pick.
    ///
    /// Returns `Err` if no `PlayOrder` selection is currently installed.
    pub fn resolve_play_order_selection(
        &mut self,
        picked: crate::selection::PlayOrder,
    ) -> Result<(), SelectionError> {
        let action_id = match picked {
            crate::selection::PlayOrder::First => crate::action::space::PLAY_FIRST,
            crate::selection::PlayOrder::Second => crate::action::space::PLAY_SECOND,
        };
        let pending = self
            .pending_selection
            .as_ref()
            .ok_or(SelectionError::NoPendingSelection)?;
        if !matches!(pending.kind, crate::selection::SelectionKind::PlayOrder) {
            return Err(SelectionError::NoPendingSelection);
        }
        let player = pending.selecting_player;
        self.resolve_selection(player, action_id)
    }

    /// Declare a winner with an explicit terminal reason.
    pub fn declare_winner_with_reason(
        &mut self,
        winner_id: PlayerId,
        reason: TerminalOutcomeReason,
    ) {
        self.game_over = true;
        self.winner = Some(winner_id);
        self.terminal_outcome_reason = Some(reason);
        self.current_phase = GamePhase::GameOver;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::GameOver {
            seq,
            winner: self.winner,
            reason,
        });
    }
}
