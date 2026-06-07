//! Once-per-turn (OPT) activation tracking (Tier 1) — impl Game.

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
}
