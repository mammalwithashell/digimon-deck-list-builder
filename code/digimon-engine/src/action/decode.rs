//! Unified action-decoder for the RL action space.
//!
//! Maps a flat `action_id` in `[0, ACTION_SPACE_SIZE)` onto the right
//! internal `Game` call based on the current `GamePhase`. Mirrors Python's
//! `digimon_gym/engine/game/action_decoder.py::decode_action` so a pytest
//! run can drive either backend with identical action sequences.
//!
//! Illegal action_ids (out of range, wrong phase, missing target) are
//! silently ignored — the caller is expected to consult the action mask.
//! This matches Python's decoder, which also no-ops on invalid inputs.

use crate::action::space::{
    decode_digivolve, ACTION_SPACE_SIZE, ATTACK_START, BREEDING_TARGET, DIGIVOLVE_END,
    DIGIVOLVE_START, DNA_DIGIVOLVE_END, DNA_DIGIVOLVE_START, EFFECTS_PER_PERMANENT,
    FIELD_EFFECT_END, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_SLOT_FOR_MAIN,
    FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START, HAND_EFFECT_END, HAND_EFFECT_START, HATCH,
    MOVE_FROM_BREEDING, PASS, PLAY_HAND_END, PLAY_HAND_START, SECURITY_TARGET,
    TARGETS_PER_ATTACKER, TRASH_EFFECT_END, TRASH_EFFECT_START,
};
use crate::enums::{CardKind, GamePhase, Keyword, ModifierType, PlaySource, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;

impl Game {
    /// Execute the action identified by `action_id` on behalf of `player_id`.
    ///
    /// Phase-dispatches the action. Illegal / out-of-range IDs are ignored
    /// to match Python's `decode_action`. Selection / interrupt phases
    /// (Block, Counter, Alliance, Select*) all funnel through
    /// `resolve_selection`, which the engine drives via `PendingSelection`
    /// callbacks.
    pub fn decode_action(&mut self, action_id: u16, player_id: PlayerId) {
        if action_id as usize >= ACTION_SPACE_SIZE {
            return;
        }
        // CONCEDE_GAME (93) is always-legal at any agent decision point.
        // Intercept BEFORE the pending_selection routing so a concede during
        // a selection clears the selection and ends the game cleanly. The
        // action mask publishes this bit whenever the player has any other
        // legal action; the engine accepts it regardless of mask state.
        if action_id == crate::action::space::CONCEDE_GAME {
            self.concede(player_id);
            return;
        }
        self.tick_declarative_effects();
        if self.pending_selection.is_some() {
            let _ = self.resolve_selection(player_id, action_id);
            self.tick_declarative_effects();
            return;
        }
        match self.current_phase {
            GamePhase::Mulligan => self.decode_mulligan(action_id, player_id),
            GamePhase::Main => self.decode_main(action_id),
            GamePhase::Breeding => self.decode_breeding(action_id),
            GamePhase::SelectTarget
            | GamePhase::SelectMaterial
            | GamePhase::SelectTrash
            | GamePhase::SelectSource
            | GamePhase::SelectHand
            | GamePhase::SelectReveal
            | GamePhase::SelectSecurity
            | GamePhase::EffectChoice
            | GamePhase::BlockTiming
            | GamePhase::CounterTiming
            | GamePhase::AllianceTiming
            // Phase 4 selection kinds — full dispatch lands in Tasks 2-5;
            // route through resolve_selection now so the state machine can
            // already accept callbacks installed by later tasks.
            | GamePhase::SelectUnion
            | GamePhase::SelectPermutation
            | GamePhase::SelectBudgeted
            | GamePhase::SelectBreedingPermanent
            // BO3 play-order pick: routes through resolve_selection like
            // every other selection phase. The callback installed by
            // `request_play_order_selection` maps action 94/95 to PlayOrder.
            | GamePhase::SelectPlayOrder => {
                let _ = self.resolve_selection(player_id, action_id);
            }
            GamePhase::EndOfTurnAction => self.decode_end_of_turn_action(action_id),
            GamePhase::Unsuspend
            | GamePhase::Draw
            | GamePhase::EndTurn
            | GamePhase::GameOver => { /* no-op */ }
        }
        self.tick_declarative_effects();
    }

    fn decode_mulligan(&mut self, action_id: u16, player_id: PlayerId) {
        // Only the player whose mulligan is pending may resolve it. A stray
        // action from the other seat (e.g. an out-of-turn submit while the
        // opponent is still mulliganing) is ignored rather than mis-applied to
        // the current decider's decision.
        if self.mulligan_current_player() != Some(player_id) {
            return;
        }
        match action_id {
            0 => {
                let _ = self.accept_mulligan(player_id, /* keep */ true);
            }
            1 => {
                let _ = self.accept_mulligan(player_id, /* keep */ false);
            }
            _ => {}
        }
    }

    fn decode_main(&mut self, action_id: u16) {
        let tp = self.turn_player();

        // [0..30) — Play from hand. Fork on CardKind: Digimon / Tamer go
        // through the field-play path; Options go through the Phase 8
        // Option pipeline (pay cost → OnUseOption + OptionMain → dispose).
        if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
            let hand_idx = action_id as usize;
            let card_kind = self
                .player(tp)
                .hand
                .get(hand_idx)
                .map(|c| c.card_kind(&self.card_data));
            match card_kind {
                Some(CardKind::Option) | Some(CardKind::Dual) => {
                    let _ = self.play_option_from_hand(tp, hand_idx);
                }
                Some(CardKind::Digimon) | Some(CardKind::Tamer) => {
                    if self.play_from_hand(tp, hand_idx).is_some() {
                        self.check_turn_end();
                    }
                }
                // DigiEgg / Token / missing — not playable via Main-phase
                // hand-play. Silent no-op matches Python's decoder.
                _ => {}
            }
            return;
        }

        // [30..60) — Hand [Main] effects
        if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&action_id) {
            let hand_idx = (action_id - HAND_EFFECT_START) as usize;
            let _ = self.activate_hand_main(tp, hand_idx);
            return;
        }

        // PASS = 62
        if action_id == PASS {
            self.pass_turn();
            return;
        }

        // [63..93) — DNA digivolve. `initiate_dna_digivolve` installs a
        // `SelectMaterial` pending selection for the first material; the
        // follow-up picks land on the selection phase branch.
        if (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&action_id) {
            let hand_idx = (action_id - DNA_DIGIVOLVE_START) as usize;
            self.initiate_dna_digivolve(tp, hand_idx);
            return;
        }

        // [100..400) — Attack
        if (ATTACK_START..ATTACK_START + 300).contains(&action_id) {
            let offset = action_id - ATTACK_START;
            let attacker_idx = offset / TARGETS_PER_ATTACKER;
            let target_idx = offset % TARGETS_PER_ATTACKER;
            self.execute_attack(tp, attacker_idx as u8, target_idx as u8, false);
            return;
        }

        // [400..1000) — Digivolve. `BREEDING_TARGET` in the field slot
        // routes to the breeding-area variant (no `WhenDigivolving`
        // triggers); everything else is a standard hand→field digivolve.
        if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
            let (hand, field) = decode_digivolve(action_id);
            if field == BREEDING_TARGET {
                self.digivolve_from_hand_onto_breeding(tp, hand as usize, PlaySource::ByDigivolve);
            } else {
                self.digivolve_from_hand(
                    tp,
                    hand as usize,
                    field as usize,
                    PlaySource::ByDigivolve,
                );
            }
            return;
        }

        // [1000..1150) — Field [Main] effects (slot 2) + DigiLink (slot 3)
        if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
            let offset = action_id - FIELD_EFFECT_START;
            let perm_idx = (offset / EFFECTS_PER_PERMANENT) as usize;
            let effect_slot = offset % EFFECTS_PER_PERMANENT;
            if effect_slot == FIELD_EFFECT_SLOT_FOR_MAIN {
                let _ = self.activate_field_main(tp, perm_idx);
            } else if effect_slot == FIELD_EFFECT_SLOT_FOR_LINK {
                self.activate_field_link(tp, perm_idx);
            }
            return;
        }

        // [1150..1195) — Trash [Main] effects
        if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action_id) {
            let trash_idx = (action_id - TRASH_EFFECT_START) as usize;
            let _ = self.activate_trash_main(tp, trash_idx);
        }
    }

    fn decode_breeding(&mut self, action_id: u16) {
        let tp = self.turn_player();
        match action_id {
            HATCH => {
                if self.hatch(tp) {
                    self.enter_main_phase();
                }
            }
            MOVE_FROM_BREEDING => {
                if self.move_from_breeding(tp) {
                    self.enter_main_phase();
                }
            }
            PASS => {
                // Breeding pass: advance to Main phase. Python's action_breeding_pass
                // calls next_phase(); Rust's equivalent is enter_main_phase.
                self.enter_main_phase();
            }
            _ => {}
        }
    }

    fn decode_end_of_turn_action(&mut self, action_id: u16) {
        if action_id == PASS {
            self.pass_end_of_turn_action();
            return;
        }

        let tp = self.turn_player();

        if (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&action_id) {
            let hand_idx = (action_id - DNA_DIGIVOLVE_START) as usize;
            self.initiate_dna_digivolve(tp, hand_idx);
            return;
        }

        // [100..400) — End-of-turn attacks (Vortex, MayAttack)
        if (ATTACK_START..ATTACK_START + 300).contains(&action_id) {
            let offset = action_id - ATTACK_START;
            let attacker_idx = (offset / TARGETS_PER_ATTACKER) as u8;
            let target_idx = (offset % TARGETS_PER_ATTACKER) as u8;
            if let Some(vortex) = self.end_of_turn_attack_vortex_flag(tp, attacker_idx, target_idx)
            {
                self.execute_attack(tp, attacker_idx, target_idx, vortex);
            }
            return;
        }

        // [1000..1150) — Overclock sacrifice-and-attack (effect slot 0).
        if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
            let offset = action_id - FIELD_EFFECT_START;
            let perm_idx = (offset / EFFECTS_PER_PERMANENT) as usize;
            let effect_slot = offset % EFFECTS_PER_PERMANENT;
            if effect_slot == FIELD_EFFECT_SLOT_FOR_OVERCLOCK {
                let _ = self.activate_overclock(perm_idx);
            }
        }
    }

    fn execute_attack(&mut self, player: PlayerId, attacker_idx: u8, target_idx: u8, vortex: bool) {
        // Validate attacker.
        let attacker_handle = PermanentHandle {
            player,
            index: attacker_idx,
        };
        if (attacker_idx as usize) >= self.player(player).battle_area.len() {
            return;
        }

        if target_idx == SECURITY_TARGET as u8 {
            // Attack the opposing player (there's exactly one opponent in
            // standard 2-player games; `next_clockwise` resolves it).
            let opponent = self.next_clockwise(player);
            let _ = self.attack_player(attacker_handle, opponent, vortex);
            return;
        }

        let opponent = self.next_clockwise(player);
        if (target_idx as usize) >= self.player(opponent).battle_area.len() {
            return;
        }
        let defender = PermanentHandle {
            player: opponent,
            index: target_idx,
        };
        let _ = self.attack_digimon(attacker_handle, defender, vortex);
    }

    fn end_of_turn_attack_vortex_flag(
        &self,
        player: PlayerId,
        attacker_idx: u8,
        target_idx: u8,
    ) -> Option<bool> {
        if (attacker_idx as usize) >= self.player(player).battle_area.len() {
            return None;
        }
        if self
            .modifiers
            .player_has(player, ModifierType::CannotAttack)
        {
            return None;
        }

        let attacker = PermanentHandle {
            player,
            index: attacker_idx,
        };
        let has_vortex = self.has_keyword(attacker, Keyword::Vortex);
        let has_normal_eot_attack = self.modifiers.has(attacker, ModifierType::MayAttack)
            || self.modifiers.has(attacker, ModifierType::ForceAttack)
            || self.has_keyword(attacker, Keyword::Engage);
        let vortex_legal = has_vortex && self.can_attack(attacker, /* vortex = */ true);
        let normal_legal =
            has_normal_eot_attack && self.can_attack(attacker, /* vortex = */ false);

        if target_idx == SECURITY_TARGET as u8 {
            if self
                .modifiers
                .has(attacker, ModifierType::CannotAttackPlayer)
            {
                return None;
            }
            if vortex_legal
                && self
                    .modifiers
                    .has(attacker, ModifierType::VortexCanAttackPlayer)
            {
                return Some(true);
            }
            return normal_legal.then_some(false);
        }

        let opponent = self.next_clockwise(player);
        let Some(target) = self.player(opponent).battle_area.get(target_idx as usize) else {
            return None;
        };
        if !target.is_digimon(&self.card_data) {
            return None;
        }
        let defender = PermanentHandle {
            player: opponent,
            index: target_idx,
        };
        // Track C / D consult site (2026-05-08): `CanAttackTargetDefendingPermanent`
        // overrides `CannotAttackTarget` at attack action decode.
        if self.attack_target_blocked_by_modifier(attacker, defender) {
            return None;
        }
        if vortex_legal {
            return Some(true);
        }
        if !normal_legal {
            return None;
        }

        let can_attack_unsuspended = self
            .modifiers
            .has(attacker, ModifierType::CanAttackUnsuspended);
        if target.is_suspended || can_attack_unsuspended {
            return Some(false);
        }
        if self.has_keyword(attacker, Keyword::Raid) {
            let max_opp = self
                .player(opponent)
                .battle_area
                .len()
                .min(crate::action::space::MAX_FIELD_SLOTS as usize);
            let mut best: Option<i32> = None;
            for j in 0..max_opp {
                let candidate = &self.player(opponent).battle_area[j];
                let candidate_handle = PermanentHandle {
                    player: opponent,
                    index: j as u8,
                };
                if candidate.is_suspended || !self.permanent_is_digimon_for_rules(candidate_handle)
                {
                    continue;
                }
                if let Some(dp) = self.effective_dp(candidate_handle) {
                    best = Some(best.map_or(dp, |b| b.max(dp)));
                }
            }
            if best.is_some_and(|max_dp| {
                self.effective_dp(defender)
                    .is_some_and(|defender_dp| defender_dp == max_dp)
            }) {
                return Some(false);
            }
        }
        None
    }
}
