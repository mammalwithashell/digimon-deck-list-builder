//! Combat resolution — attack flow, battle DP comparison, security checks.
//!
//! This is the atomic MVP version (Phase 4). Interrupt-pause/resume for Counter,
//! Block, and Alliance timing is deferred to a later phase; for now the flow is:
//!
//! 1. Validate attacker can attack (unsuspended, on field, is Digimon, not summoning-sick).
//! 2. Suspend attacker.
//! 3. Fire attacker's `OnAttack` effects.
//! 4. If target is a Digimon:
//!    - Resolve battle: compare effective DP.
//!    - Loser(s) are deleted (OnDeletion fires per deletion).
//!    - Ties: both are deleted.
//! 5. If target is a player:
//!    - Security check loop: reveal top N cards of defender's security.
//!    - Digimon security → battle attacker vs it (no modifiers on sec Digimon).
//!    - Other security → trashed (effects not yet wired).
//!    - If attacker still alive AND defender has no security → attacker wins the game.
//!
//! `OnSecurityCheck` / `OnEndAttack` / `OnStartBattle` / `OnEndBattle` hooks are
//! not yet fired — those require the full effect timing system and will be added
//! as needed by specific cards.

use crate::card_source::CardSource;
use crate::effect_context::EffectContext;
use crate::enums::{CardKind, Keyword, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;

/// Result of an attack resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackResult {
    /// Attack couldn't be declared (invalid attacker or target).
    Invalid,
    /// Attacker won the battle (defender was deleted or security absorbed).
    AttackerWins,
    /// Defender won (attacker was deleted).
    DefenderWins,
    /// Both attacker and defender were deleted in battle.
    MutualDestruction,
    /// Security check(s) ended with the attacker still alive; game continues.
    SecurityCheckSurvived,
    /// Security check resulted in the attacker being deleted.
    AttackerDeletedBySecurity,
    /// Attack connected on a defenseless player — game over.
    GameWon,
}

impl Game {
    /// Compute a permanent's effective DP (base + modifier sum).
    pub fn effective_dp(&self, handle: PermanentHandle) -> Option<i32> {
        let perm = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?;
        let base = perm.base_dp(&self.card_data)?;
        let bonus = self.modifiers.sum(handle, ModifierType::ChangeDp);
        Some(base + bonus)
    }

    /// Check whether a permanent can attack right now (atomic — ignores interrupts).
    pub fn can_attack(&self, handle: PermanentHandle) -> bool {
        let perm = match self.player(handle.player).battle_area.get(handle.index as usize) {
            Some(p) => p,
            None => return false,
        };
        if !perm.is_digimon(&self.card_data) {
            return false;
        }
        if perm.is_suspended {
            return false;
        }
        // Summoning sickness: can't attack on the turn it was played unless
        // Rush has been granted (§2.1 parity fix). Native Rush from a card's
        // static keyword list is not yet checked — that requires the
        // effect-listing infrastructure to be in place. For now, only
        // modifier-granted Rush exempts a permanent.
        let is_fresh = perm.turn_played == self.turn_count && perm.turn_digivolved == 0;
        if is_fresh && !self.modifiers.has_keyword(handle, Keyword::Rush) {
            return false;
        }
        true
    }

    /// Attack another Digimon on the opponent's field.
    /// Returns the battle outcome.
    pub fn attack_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        if !self.can_attack(attacker) {
            return AttackResult::Invalid;
        }
        // Defender must be on field and be a Digimon.
        let valid_defender = {
            let d = self
                .player(defender.player)
                .battle_area
                .get(defender.index as usize);
            match d {
                Some(p) => p.is_digimon(&self.card_data),
                None => false,
            }
        };
        if !valid_defender {
            return AttackResult::Invalid;
        }

        // Suspend attacker and record attack.
        self.suspend_and_count_attack(attacker);

        // Fire OnAttack effects on the attacker.
        self.fire_on_attack(attacker);

        // Verify attacker still on field after OnAttack effects.
        if !self.handle_valid(attacker) {
            self.modifiers.expire_end_of_attack();
            return AttackResult::Invalid;
        }
        if !self.handle_valid(defender) {
            self.modifiers.expire_end_of_attack();
            return AttackResult::AttackerWins;
        }

        // Resolve battle.
        let outcome = self.resolve_battle(attacker, defender);

        self.modifiers.expire_end_of_attack();
        outcome
    }

    /// Attack the defending player (security check sequence).
    pub fn attack_player(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
    ) -> AttackResult {
        if !self.can_attack(attacker) {
            return AttackResult::Invalid;
        }

        self.suspend_and_count_attack(attacker);
        self.fire_on_attack(attacker);

        if !self.handle_valid(attacker) {
            self.modifiers.expire_end_of_attack();
            return AttackResult::Invalid;
        }

        // Number of security checks = 1 + SecurityAttack modifier sum.
        let sa_bonus = self
            .modifiers
            .sum(attacker, ModifierType::SecurityAttackChange);
        let checks = (1 + sa_bonus).max(0) as usize;

        let mut attacker_alive = true;
        for _ in 0..checks {
            if self.game_over {
                break;
            }
            if !self.handle_valid(attacker) {
                attacker_alive = false;
                break;
            }
            let sec_card = match self.player_mut(defender_player).security.pop() {
                Some(c) => c,
                None => {
                    // No security left — attack connects and wins the game.
                    let winner = attacker.player;
                    self.declare_winner(winner);
                    self.modifiers.expire_end_of_attack();
                    return AttackResult::GameWon;
                }
            };
            let outcome = self.resolve_security_card(attacker, sec_card, defender_player);
            match outcome {
                AttackResult::AttackerDeletedBySecurity => {
                    attacker_alive = false;
                    break;
                }
                AttackResult::GameWon => {
                    self.modifiers.expire_end_of_attack();
                    return AttackResult::GameWon;
                }
                _ => {}
            }
        }

        self.modifiers.expire_end_of_attack();

        if !attacker_alive {
            AttackResult::AttackerDeletedBySecurity
        } else {
            AttackResult::SecurityCheckSurvived
        }
    }

    // ─── Private helpers ──────────────────────────────────────────────

    fn suspend_and_count_attack(&mut self, handle: PermanentHandle) {
        let perm = &mut self.players[handle.player as usize].battle_area
            [handle.index as usize];
        perm.is_suspended = true;
        perm.attacks_this_turn = perm.attacks_this_turn.saturating_add(1);
    }

    fn handle_valid(&self, handle: PermanentHandle) -> bool {
        self.player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|p| p.is_digimon(&self.card_data))
            .unwrap_or(false)
    }

    /// Fire OnAttack effects for the attacker.
    fn fire_on_attack(&mut self, handle: PermanentHandle) {
        let (card_id, card_handle) = {
            let perm = match self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
            {
                Some(p) => p,
                None => return,
            };
            let top = perm.top_card();
            (top.card_id(&self.card_data).to_string(), top.handle())
        };

        let effect_impl = match self.effect_registry.get(&card_id) {
            Some(arc) => arc,
            None => return,
        };
        let effects = effect_impl.effects(card_handle);

        for effect in &effects {
            if !effect.on_attack {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectContext::new(self, card_handle, Some(handle), handle.player);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx =
                    EffectContext::new(self, card_handle, Some(handle), handle.player);
                process(&mut ctx);
            }
        }
    }

    /// Resolve battle between two permanents by DP comparison.
    fn resolve_battle(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        let a_dp = self.effective_dp(attacker).unwrap_or(0);
        let d_dp = self.effective_dp(defender).unwrap_or(0);

        if a_dp > d_dp {
            // Attacker wins — defender is deleted.
            self.delete_permanent_with_effects(defender);
            AttackResult::AttackerWins
        } else if a_dp < d_dp {
            // Defender wins — attacker is deleted.
            self.delete_permanent_with_effects(attacker);
            AttackResult::DefenderWins
        } else {
            // Tie — both are deleted. Delete in order: defender first to match
            // DCGO convention, but both need OnDeletion to fire.
            // Since the second deletion can shift indices, re-resolve via card_index
            // to be safe: we use the handles directly since delete_permanent_with_effects
            // reads the top card's card_index before deletion.
            self.delete_permanent_with_effects(defender);
            // After deleting defender, attacker's own handle index is unchanged
            // (different player's battle_area), so the attacker handle is still valid.
            if self.handle_valid(attacker) {
                self.delete_permanent_with_effects(attacker);
            }
            AttackResult::MutualDestruction
        }
    }

    /// Resolve a single security card being revealed.
    fn resolve_security_card(
        &mut self,
        attacker: PermanentHandle,
        sec_card: CardSource,
        defender_player: PlayerId,
    ) -> AttackResult {
        let kind = sec_card.card_kind(&self.card_data);
        match kind {
            CardKind::Digimon => {
                // Battle attacker vs the security Digimon.
                // Security Digimon has its base DP (no field modifiers).
                let attacker_dp = self.effective_dp(attacker).unwrap_or(0);
                let sec_dp = sec_card.dp(&self.card_data).unwrap_or(0);
                if attacker_dp >= sec_dp {
                    // Attacker wins or ties: security Digimon is trashed,
                    // attacker survives (ties against security favor the attacker
                    // in Digimon TCG — the security Digimon is trashed).
                    self.player_mut(defender_player).trash.push(sec_card);
                    AttackResult::SecurityCheckSurvived
                } else {
                    // Security Digimon wins: attacker is deleted, security
                    // Digimon is trashed (security is "consumed" either way).
                    self.player_mut(defender_player).trash.push(sec_card);
                    self.delete_permanent_with_effects(attacker);
                    AttackResult::AttackerDeletedBySecurity
                }
            }
            CardKind::Option | CardKind::Tamer => {
                // Security effects would fire here. For Phase 4 MVP, just trash.
                self.player_mut(defender_player).trash.push(sec_card);
                AttackResult::SecurityCheckSurvived
            }
            CardKind::DigiEgg => {
                // Eggs in security are just trashed (shouldn't happen normally).
                self.player_mut(defender_player).trash.push(sec_card);
                AttackResult::SecurityCheckSurvived
            }
        }
    }

    /// Delete a permanent, firing its OnDeletion effects first.
    /// Also clears any modifiers attached to the handle.
    pub fn delete_permanent_with_effects(&mut self, handle: PermanentHandle) {
        // Fire OnDeletion BEFORE actually deleting (so effects can observe state).
        let (card_id, card_handle) = {
            let perm = match self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
            {
                Some(p) => p,
                None => return,
            };
            let top = perm.top_card();
            (top.card_id(&self.card_data).to_string(), top.handle())
        };

        if let Some(effect_impl) = self.effect_registry.get(&card_id) {
            let effects = effect_impl.effects(card_handle);
            for effect in &effects {
                if !effect.on_deletion {
                    continue;
                }
                if let Some(cond) = &effect.condition {
                    let ctx =
                        EffectContext::new(self, card_handle, Some(handle), handle.player);
                    if !cond(&ctx) {
                        continue;
                    }
                }
                if let Some(process) = &effect.process {
                    let mut ctx =
                        EffectContext::new(self, card_handle, Some(handle), handle.player);
                    process(&mut ctx);
                }
            }
        }

        // Now remove the permanent from the field and trash its cards.
        self.player_mut(handle.player)
            .delete_permanent(handle.index as usize);
        // Clear any modifiers on the handle (by index).
        self.modifiers.clear_permanent(handle);
    }

}
