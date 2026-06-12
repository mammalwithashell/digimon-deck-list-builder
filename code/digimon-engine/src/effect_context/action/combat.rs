//! Attack / battle / counter mutations on `EffectContext` — extracted by mechanic.

#![allow(unused_imports)]
use crate::action::mask::*;
use crate::action::space::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;

impl<'a> EffectContext<'a> {
    pub(crate) fn cleanup_exposed_battle_area_digi_egg(&mut self, target: PermanentHandle) -> bool {
        let exposed = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_some_and(|perm| {
                perm.top_card().card_kind(&self.game.card_data) == CardKind::DigiEgg
            });
        if !exposed {
            return false;
        }

        self.game.delete_permanent_with_effects(target);
        true
    }

    /// Redirect the active attack to a new target. Fires
    /// `OnAttackTargetChange` globally. Spec §6.1.
    ///
    /// Callable from any effect closure running during an active attack —
    /// `OnAttack`, `WhenAttacking`, a replacement process (equivalent to
    /// `rctx.substitute(...)` via the replacement committer), or a
    /// selection callback.
    ///
    /// Errors:
    /// - `AttackError::NoActiveAttack` — `pending_attack` is `None`.
    /// - `AttackError::InvalidTarget` — target is not a legal attack target
    ///   (own attacker, non-Digimon, Delayed/Training Option, or wrong
    ///   player for a direct-player attack).
    ///
    /// No-op + `Ok(())` if `new_target` equals the current effective
    /// target (suppress redundant `OnAttackTargetChange` fan-out).
    pub fn redirect_attack(
        &mut self,
        new_target: crate::selection::AttackTarget,
    ) -> Result<(), crate::combat::AttackError> {
        use crate::combat::AttackError;
        let Some(pa) = self.game.pending_attack.as_ref() else {
            return Err(AttackError::NoActiveAttack);
        };
        let attacker = pa.attacker;
        self.game
            .validate_attack_redirect_target(attacker, new_target)?;
        self.game.apply_attack_target_substitution_with_reason(
            new_target,
            crate::trigger_context::AttackTargetChangeReason::EffectRedirect(Some(
                self.source_card,
            )),
        );
        Ok(())
    }

    pub fn select_redirect_attack_target(
        &mut self,
        targets: AttackTargetRestriction,
        optional: bool,
        prompt: &str,
    ) -> Result<(), crate::combat::AttackError> {
        use crate::combat::AttackError;
        let Some(pa) = self.game.pending_attack.as_ref() else {
            return Err(AttackError::NoActiveAttack);
        };
        let attacker = pa.attacker;
        let current_target = pa.effective_target;
        let opponent = self.game.next_clockwise(attacker.player);
        let mut valid_action_ids = Vec::new();

        if matches!(
            targets,
            AttackTargetRestriction::Any | AttackTargetRestriction::PlayerOnly
        ) {
            let target = AttackTarget::Player(opponent);
            if target != current_target
                && self
                    .game
                    .validate_attack_redirect_target(attacker, target)
                    .is_ok()
            {
                valid_action_ids.push(encode_attack(attacker.index as u16, SECURITY_TARGET));
            }
        }

        if matches!(
            targets,
            AttackTargetRestriction::Any | AttackTargetRestriction::DigimonOnly
        ) {
            for index in 0..self.game.player(opponent).battle_area.len() {
                let target = AttackTarget::Digimon(PermanentHandle {
                    player: opponent,
                    index: index as u8,
                });
                if target != current_target
                    && self
                        .game
                        .validate_attack_redirect_target(attacker, target)
                        .is_ok()
                {
                    valid_action_ids.push(encode_attack(attacker.index as u16, index as u16));
                }
            }
        }

        if valid_action_ids.is_empty() {
            return Ok(());
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let previous_phase = self.game.current_phase;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;

        self.game.current_phase = GamePhase::SelectTarget;
        self.game.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::Target,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game, action_id| {
                let (decoded_attacker, decoded_target) = decode_attack(action_id);
                if decoded_attacker as u8 != attacker.index {
                    return;
                }
                let opponent = game.next_clockwise(attacker.player);
                let target = if decoded_target == SECURITY_TARGET {
                    AttackTarget::Player(opponent)
                } else {
                    AttackTarget::Digimon(PermanentHandle {
                        player: opponent,
                        index: decoded_target as u8,
                    })
                };
                if game
                    .validate_attack_redirect_target(attacker, target)
                    .is_ok()
                {
                    game.apply_attack_target_substitution_with_reason(
                        target,
                        crate::trigger_context::AttackTargetChangeReason::EffectRedirect(Some(
                            source_card,
                        )),
                    );
                }
            }),
            on_decline: if optional {
                Some(Box::new(|_game: &mut Game| {}) as DeclineCallback)
            } else {
                None
            },
        });
        Ok(())
    }

    /// Cancel the active attack. Sets `pending_attack.cancelled = true`;
    /// `advance_pending_attack` detects the flag and short-circuits to
    /// `Cleanup`. `EndOfAttack` still fires (cleanup symmetry);
    /// `EndOfBattle` does NOT (no DP comparison ran). Spec §6.2.
    ///
    /// Errors: `AttackError::NoActiveAttack` if `pending_attack` is `None`.
    ///
    /// **Late-cancel semantics.** If called after a `ctx.redirect_attack`
    /// in the same attack, the redirect's mutation to
    /// `effective_target` survives — `cancelled` short-circuits
    /// `advance_pending_attack` regardless. Cancel wins over redirect
    /// in the sense that no battle occurs; the redirected target is
    /// observable only via the `OnAttackTargetChange` observer that
    /// already fired.
    pub fn cancel_attack(&mut self) -> Result<(), crate::combat::AttackError> {
        self.game.cancel_pending_attack_from_effect_checked()
    }

    pub fn cancel_pending_attack(&mut self) {
        self.game.cancel_pending_attack_from_effect();
    }

    pub fn open_counter_window(&mut self) -> Result<bool, crate::combat::AttackError> {
        self.game.open_counter_window_from_effect_checked()
    }

    pub fn battle_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        self.game.battle_digimon(attacker, defender)
    }

    pub fn may_attack_now(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional(attacker, targets, without_suspending, true, prompt)
    }

    pub fn may_attack_now_optional(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional_with_upgrade(
            attacker,
            targets,
            without_suspending,
            optional,
            prompt,
            None,
        )
    }

    pub fn may_attack_now_optional_with_upgrade(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional_with_upgrade_and_summoning(
            attacker,
            targets,
            without_suspending,
            optional,
            prompt,
            cost_upgrade,
            false,
        )
    }

    pub fn may_attack_now_ignoring_summoning_sickness(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional_with_upgrade_and_summoning(
            attacker,
            targets,
            without_suspending,
            optional,
            prompt,
            None,
            true,
        )
    }

    pub fn may_attack_now_ignoring_summoning_sickness_with_upgrade(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
    ) -> Result<(), AttackError> {
        self.may_attack_now_optional_with_upgrade_and_summoning(
            attacker,
            targets,
            without_suspending,
            optional,
            prompt,
            cost_upgrade,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn may_attack_now_optional_with_upgrade_and_summoning(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        optional: bool,
        prompt: &str,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
        ignore_summoning_sickness: bool,
    ) -> Result<(), AttackError> {
        let valid_action_ids = if ignore_summoning_sickness {
            effect_attack_target_action_ids_with_options(
                self.game,
                attacker,
                targets,
                without_suspending,
                true,
            )
        } else {
            effect_attack_target_action_ids(self.game, attacker, targets, without_suspending)
        };
        if valid_action_ids.is_empty() {
            return Ok(());
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let previous_phase = self.game.current_phase;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;

        self.game.current_phase = GamePhase::SelectTarget;
        self.game.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::Target,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game, action_id| {
                let (decoded_attacker, decoded_target) = decode_attack(action_id);
                if decoded_attacker as u8 != attacker.index {
                    return;
                }
                let opponent = game.next_clockwise(attacker.player);
                let target = if decoded_target == SECURITY_TARGET {
                    AttackTarget::Player(opponent)
                } else {
                    AttackTarget::Digimon(PermanentHandle {
                        player: opponent,
                        index: decoded_target as u8,
                    })
                };
                game.begin_attack_open(AttackOpen {
                    attacker,
                    initiator: AttackInitiator::Effect {
                        source: Some(source_card),
                        optional,
                    },
                    suspend_attacker: !without_suspending,
                    ignore_summoning_sickness,
                    target_constraint: TargetConstraint::Forced(target),
                    allow_cancel: optional,
                    cost_upgrade,
                });
            }),
            on_decline: if optional {
                Some(Box::new(|_game: &mut Game| {}) as DeclineCallback)
            } else {
                None
            },
        });
        Ok(())
    }

    pub fn force_opponent_attack(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        prompt: &str,
    ) -> Result<(), AttackError> {
        self.force_opponent_attack_with_upgrade(attacker, targets, without_suspending, prompt, None)
    }

    pub fn force_opponent_attack_with_upgrade(
        &mut self,
        attacker: PermanentHandle,
        targets: AttackTargetRestriction,
        without_suspending: bool,
        prompt: &str,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
    ) -> Result<(), AttackError> {
        let previous_override = self.override_selecting_player;
        self.override_selecting_player = Some(attacker.player);
        let result = self.may_attack_now_optional_with_upgrade(
            attacker,
            targets,
            without_suspending,
            false,
            prompt,
            cost_upgrade,
        );
        self.override_selecting_player = previous_override;
        result
    }
}
