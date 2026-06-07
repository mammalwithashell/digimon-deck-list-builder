//! Memory mutations on `EffectContext` — extracted from `mod.rs` by mechanic.
//!
//! Effect-driven memory gains/losses. `gain_memory` enforces the by-effect
//! memory-gain flood-gates (`CannotGainMemoryByEffect`,
//! `CannotGainMemoryExceptFromTamers`, permanent-scoped `CannotAddMemory`)
//! before delegating the raw mutation to `Game`.

use crate::effect_context::EffectContext;
use crate::enums::ModifierType;
use crate::permanent::PermanentHandle;

impl<'a> EffectContext<'a> {
    pub fn gain_memory(&mut self, amount: i16) {
        let target = self.player;
        // Phase 6: CannotGainMemoryByEffect — suppress all memory gains by effect.
        if self
            .game
            .modifiers
            .player_has(target, ModifierType::CannotGainMemoryByEffect)
        {
            return;
        }
        // Phase 6: CannotGainMemoryExceptFromTamers — only Tamer-sourced gains are
        // allowed; block Digimon/Option-sourced gains.
        if self
            .game
            .modifiers
            .player_has(target, ModifierType::CannotGainMemoryExceptFromTamers)
            && !self.source_is_tamer()
        {
            return;
        }
        // Track C / D consult site (2026-05-08): permanent-scoped
        // `CannotAddMemory` — while any permanent in the acting player's
        // battle area carries this modifier, the controller's effects
        // can't add memory. Sibling of player-scoped
        // `CannotGainMemoryByEffect` for printed text anchored to a
        // specific Digimon.
        let battle_area_len = self.game.player(target).battle_area.len();
        for i in 0..battle_area_len {
            let h = PermanentHandle {
                player: target,
                index: i as u8,
            };
            if self.game.modifiers.has(h, ModifierType::CannotAddMemory) {
                return;
            }
        }
        self.game.gain_memory_for_player(target, amount);
    }

    pub fn lose_memory(&mut self, amount: i16) {
        let new_memory = self.game.memory - amount;
        self.game.set_memory(new_memory);
    }

    pub fn set_memory(&mut self, value: i16) {
        self.game.set_memory(value);
    }
}
