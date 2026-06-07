//! Suspend / unsuspend / hatch (Tier 1).

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
    /// Suspend a single permanent. Fires `OnSuspend` observers in every
    /// player's battle area if the permanent was not already suspended.
    ///
    /// This is the canonical chokepoint for single-target suspension.
    /// `Player::unsuspend_all` (bulk turn-begin unsuspend) intentionally
    /// bypasses this path — `StartOfYourTurn` is the canonical timing for
    /// turn-start effects.
    pub fn suspend(&mut self, handle: PermanentHandle) {
        let is_breeding = handle.index == crate::action::space::BREEDING_TARGET as u8;
        let event_card = if is_breeding {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .map(|perm| perm.top_card().handle())
        } else {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize))
                .map(|perm| perm.top_card().handle())
        };
        let already = if is_breeding {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .map(|perm| perm.is_suspended)
                .unwrap_or(true)
        } else {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize))
                .map(|perm| perm.is_suspended)
                .unwrap_or(true)
        }; // treat out-of-range as "already suspended" to no-op
        if already {
            return;
        }
        let perm = if is_breeding {
            self.players
                .get_mut(handle.player as usize)
                .and_then(|p| p.breeding_area.as_mut())
        } else {
            self.players
                .get_mut(handle.player as usize)
                .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        };
        if let Some(perm) = perm {
            perm.is_suspended = true;
        }
        self.mark_until_condition_dirty();
        if let Some(card) = event_card {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnSuspend,
                crate::selection::TriggerSource::EventObserved {
                    player: handle.player,
                    permanent: handle,
                    card,
                },
            );
        }
        self.drain_effect_queue();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Unsuspend a single permanent. Fires `OnUnsuspend` observers in every
    /// player's battle area if the permanent was suspended.
    ///
    /// See `suspend` for the bulk-unsuspend caveat.
    pub fn unsuspend(&mut self, handle: PermanentHandle) {
        let event_card = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.top_card().handle());
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
        self.mark_until_condition_dirty();
        if let Some(card) = event_card {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnUnsuspend,
                crate::selection::TriggerSource::EventObserved {
                    player: handle.player,
                    permanent: handle,
                    card,
                },
            );
        }
        self.drain_effect_queue();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Hatch for a player (copies turn_count to avoid borrow conflict).
    /// Fires `OnHatch` observers in every player's battle area after the egg
    /// moves into the breeding area.
    pub fn hatch(&mut self, player_id: PlayerId) -> bool {
        let turn = self.turn_count;
        let ok = self.player_mut(player_id).hatch(turn);
        if ok {
            self.mark_until_condition_dirty();
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
            self.reevaluate_until_condition_modifiers_if_dirty();
        }
        ok
    }
}
