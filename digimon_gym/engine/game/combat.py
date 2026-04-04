"""Combat resolution mixin for the Game class.

Methods that heavily mutate game state during attack/block/counter/battle flows.
Uses `from __future__ import annotations` + TYPE_CHECKING to avoid circular imports.
"""
from __future__ import annotations
from typing import Optional, Union

from .constants import PendingAttack
from ..data.enums import GamePhase, EffectTiming, AttackResolution
from ..interfaces.modifiers import ModifierType
from ..validation.digivolve_validator import can_digivolve
from ..core.permanent import Permanent
from ..core.player import Player


class CombatMixin:
    """Attack, block, counter, and battle resolution methods."""

    def resolve_attack(self, attacker: Permanent, target: Union[Permanent, Player],
                       without_suspend: bool = False, is_vortex: bool = False,
                       return_phase: Optional[GamePhase] = None):
        """Begin an attack sequence. May pause for BlockTiming/CounterTiming/AllianceTiming."""
        if not attacker.can_attack(without_tap=without_suspend, is_vortex=is_vortex):
            return

        target_name = target.player_name if isinstance(target, Player) else self._perm_ref(target)
        attacker_ref = self._perm_ref(attacker)
        self.logger.log(f"[Attack] {attacker_ref} attacks {target_name}")
        self._emit(
            'attack_declare',
            source_card_id=self._card_id(attacker.top_card),
            source_slot=self._perm_slot(attacker),
            target_card_id=self._card_id(target.top_card) if isinstance(target, Permanent) else None,
            target_slot=self._perm_slot(target) if isinstance(target, Permanent) else None,
            attacker_name=self._card_name(attacker.top_card),
            target_type='player' if isinstance(target, Player) else 'digimon',
            target_name=target_name,
        )

        # <Progress>: mark attacker as attacking (for effect immunity)
        attacker.is_attacking = True
        attacker.attack_count_this_turn += 1

        # Clear FORCE_ATTACK modifier once the forced Digimon actually attacks
        if self.modifiers.has_modifier(attacker, ModifierType.FORCE_ATTACK):
            entries = self.modifiers._modifiers.get(ModifierType.FORCE_ATTACK, [])
            self.modifiers._modifiers[ModifierType.FORCE_ATTACK] = [
                e for e in entries
                if not e.is_active(attacker)
            ]

        if not without_suspend:
            attacker.suspend()

        # Trigger When Attacking (OnUseAttack = self, OnAllyAttack = other allies)
        self.execute_effects(EffectTiming.OnUseAttack, {"attacker": attacker})
        self.execute_effects(EffectTiming.OnAllyAttack, {"attacker": attacker})

        # Store pending attack context
        self.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=target,
            effective_target=target,
            without_suspend=without_suspend,
            is_vortex=is_vortex,
            return_phase=return_phase,
        )

        # Park if a When Attacking effect created a pending selection
        # (e.g. effect_play_from_zone). The selection must resolve before
        # combat continues to counter/block/security.  Resume via
        # _continue_attack_after_wa_selection, called from _decode_selection.
        if self.pending_selection is not None:
            self._post_wa_selection_continuation = True
            return

        self._continue_attack_post_wa()

    def _continue_attack_post_wa(self):
        """Continue the attack flow after When Attacking effects have resolved."""
        pa = self.pending_attack
        if pa is None:
            return

        # Fire opponent-side "when attacked" observers (DCGO: SwitchDefender window)
        self._fire_opponent_attack_observers(pa.attacker)

        # <Alliance>: check if attacker has Alliance and suspendable allies exist
        if pa.attacker.has_keyword('_is_alliance'):
            has_alliance_targets = any(
                perm is not pa.attacker and perm.is_digimon and not perm.is_suspended
                for perm in self.turn_player.battle_area
            )
            if has_alliance_targets:
                self.current_phase = GamePhase.AllianceTiming
                return  # Park for Alliance decision

        # No Alliance — enter counter timing (DCGO: counter before block)
        self._enter_counter_timing()

    def redirect_attack(self, new_target: 'Permanent'):
        """Redirect the current attack to a different target.

        DCGO ref: attackProcess.SwitchDefender(activateClass, false, card.PermanentOfThisCard())
        Called by card effects that redirect attacks (e.g. EX11-042, P-094, BT22-061).
        """
        pa = self.pending_attack
        if pa is None:
            return
        old_target = pa.effective_target
        pa.effective_target = new_target
        self.logger.log(f"[Attack] Attack redirected to {self._perm_ref(new_target)}")
        self._emit(
            'attack_redirect',
            source_card_id=self._card_id(pa.attacker.top_card),
            target_card_id=self._card_id(new_target.top_card),
            target_slot=self._perm_slot(new_target),
        )
        # Fire OnAttackTargetChanged timing
        self.execute_effects(EffectTiming.OnAttackTargetChanged, {
            'attacker': pa.attacker,
            'old_target': old_target,
            'new_target': new_target,
        })

    def _enter_counter_timing(self):
        """Check for counter opportunities and enter CounterTiming if any exist."""
        pa = self.pending_attack
        if pa is None or pa.is_end_attack:
            self._resolve_battle()
            return

        # DCGO: attacker validity check at every state transition
        if pa.attacker not in self.turn_player.battle_area or not pa.attacker.is_digimon:
            self._end_attack_early()
            return

        has_counter = self._opponent_has_counter_options()

        if has_counter:
            self.current_phase = GamePhase.CounterTiming
            self.active_player = self.opponent_player
            return  # Park here; _decode_counter() will resume

        # No counter options — check for blockers
        self._check_blockers_or_continue()

    def _check_blockers_or_continue(self):
        """Check for blockers after counter decisions, then continue attack flow."""
        pa = self.pending_attack
        if pa is None or pa.is_end_attack:
            self._resolve_battle()
            return

        # DCGO: attacker validity check at every state transition
        if pa.attacker not in self.turn_player.battle_area or not pa.attacker.is_digimon:
            self._end_attack_early()
            return

        has_blockers = any(
            perm.can_block(pa.attacker) for perm in self.opponent_player.battle_area
        )

        if has_blockers:
            self.current_phase = GamePhase.BlockTiming
            self.active_player = self.opponent_player
            return  # Park here; _decode_block() will resume

        # No blockers — resolve battle immediately
        self._resolve_battle()

    def _opponent_has_counter_options(self) -> bool:
        """Check if the defending player has any valid blast digivolve options."""
        defender = self.opponent_player
        for card in defender.hand_cards:
            if not card.is_digimon:
                continue
            effects = card.effect_list(EffectTiming.NoTiming)
            has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
            if not has_blast:
                continue
            for perm in defender.battle_area:
                if can_digivolve(card, perm):
                    return True
        return False

    def _execute_security_checks(self, attacker: Permanent, defender_player: Player) -> bool:
        """Run security check loop for an attacker against a defending player.

        DCGO order per check:
        1. Break top security card (reveal + pop)
        2. Activate the card's [Security] effects
        3. Fire OnLoseSecurity + OnSecurityCheck (per-check, after security effects)
        4. Battle security Digimon if applicable
        5. Trash card, clear per-check effects
        """
        sa_mod = attacker.security_attack_modifier()
        num_checks = max(0, 1 + sa_mod)
        for _ in range(num_checks):
            if self.game_over:
                return False
            if attacker not in self.turn_player.battle_area:
                return False
            result = defender_player.security_attack(attacker)
            # Fire OnSecurityCheck per check, AFTER security effects (DCGO deferred timing)
            self.execute_effects(EffectTiming.OnSecurityCheck, {"attacker": attacker})
            if result == AttackResolution.AttackerDeleted:
                self.turn_player.delete_permanent(attacker, is_battle=True)
                return False
            elif result == AttackResolution.GameEnd:
                self.declare_winner(self.turn_player)
                return False
        return True

    def _end_attack_early(self):
        """End an attack early when attacker/target becomes invalid mid-attack."""
        pa = self.pending_attack
        if pa is None:
            return
        attacker = pa.attacker
        return_phase = pa.return_phase or GamePhase.Main
        self.active_player = None
        self.pending_attack = None
        self.current_phase = return_phase

        # Still fire OnEndAttack if attacker exists (DCGO skips if attacker TopCard is null)
        if attacker.top_card is not None:
            self.execute_effects(EffectTiming.OnEndAttack)

        attacker.clear_attack_state()
        self.modifiers.clear_expiry('end_of_attack')

    @staticmethod
    def _can_be_destroyed_by_battle(perm: Permanent) -> bool:
        """Check if a permanent can be destroyed by battle resolution.

        DCGO: CanBeDestroyedByBattle() checks CanBeDestroyed() first, then scans
        for ICanNotBeDestroyedByBattleEffect. Jamming is one such effect but only
        applies vs security Digimon (handled in security_attack), not field battles.
        Other effects (e.g. "can't be destroyed by battle") are checked via modifiers.
        """
        if perm._owner_game and hasattr(perm._owner_game, 'modifiers'):
            if not perm._owner_game.modifiers.can_be_destroyed(perm, is_battle=True):
                return False
        return True

    def _resolve_battle(self):
        """Execute the actual battle resolution after block/counter decisions."""
        pa = self.pending_attack
        if pa is None:
            return

        attacker = pa.attacker
        target = pa.effective_target

        # Clear interrupt state — back to turn_player control
        return_phase = pa.return_phase or GamePhase.Main
        self.active_player = None
        self.pending_attack = None
        self.current_phase = return_phase

        # DCGO: final attacker validity check before battle
        if attacker not in self.turn_player.battle_area or not attacker.is_digimon:
            # Attacker gone — skip battle, still clean up attack state
            if attacker.top_card is not None:
                self.execute_effects(EffectTiming.OnEndAttack)
            attacker.clear_attack_state()
            self.modifiers.clear_expiry('end_of_attack')
            return

        if isinstance(target, Player):
            self._execute_security_checks(attacker, target)
        elif isinstance(target, Permanent):
            # DCGO: if defender was removed mid-attack, skip battle (no security check either)
            if target not in self.opponent_player.battle_area:
                self.logger.log("[Battle] Defender removed mid-attack — skipping battle")
            elif not target.is_digimon:
                self.logger.log("[Battle] Defender is no longer a Digimon — skipping battle")
            else:
                self.execute_effects(EffectTiming.OnStartBattle, {"attacker": attacker, "defender": target})

                # <Iceclad>: compare digivolution card count instead of DP
                if attacker.has_keyword('_is_iceclad') or target.has_keyword('_is_iceclad'):
                    a_val = len(attacker.card_sources) - 1  # exclude top card
                    t_val = len(target.card_sources) - 1
                else:
                    a_val = attacker.dp or 0
                    t_val = target.dp or 0

                a_dp = attacker.dp or 0  # kept for event emission
                t_dp = target.dp or 0
                attacker_wins = a_val > t_val
                defender_wins = a_val < t_val
                tie = a_val == t_val

                if attacker_wins:
                    result_str = 'attacker_wins'
                elif defender_wins:
                    result_str = 'defender_wins'
                else:
                    result_str = 'tie'
                self._emit(
                    'battle_result',
                    source_card_id=self._card_id(attacker.top_card),
                    target_card_id=self._card_id(target.top_card),
                    attacker_dp=a_dp,
                    defender_dp=t_dp,
                    result=result_str,
                )

                if attacker_wins:
                    # <Retaliation>: when this Digimon is deleted in battle, delete the winner
                    has_retaliation = target.has_keyword('_is_retaliation')
                    # DCGO: check CanBeDestroyedByBattle before adding to losers
                    if self._can_be_destroyed_by_battle(target):
                        was_deleted = self.opponent_player.delete_permanent(target, is_battle=True)
                    else:
                        was_deleted = False
                        self.logger.log(f"[Battle] {self._perm_ref(target)} survives — can't be destroyed by battle")
                    if has_retaliation and was_deleted:
                        self.logger.log(f"[Retaliation] {self._perm_ref(target)} retaliates!")
                        self._emit('keyword_trigger', source_card_id=self._card_id(target.top_card),
                                   keyword='Retaliation', card_name=self._card_name(target.top_card))
                        self.turn_player.delete_permanent(attacker, is_battle=True, is_opponent_effect=True)
                    # <Piercing>: after winning battle vs Digimon, check security
                    elif was_deleted and attacker.has_keyword('_is_piercing') and attacker in self.turn_player.battle_area:
                        self.logger.log(f"[Piercing] {self._perm_ref(attacker)} pierces through!")
                        self._emit('keyword_trigger', source_card_id=self._card_id(attacker.top_card),
                                   keyword='Piercing', card_name=self._card_name(attacker.top_card))
                        self._execute_security_checks(attacker, self.opponent_player)
                elif defender_wins:
                    has_retaliation = attacker.has_keyword('_is_retaliation')
                    if self._can_be_destroyed_by_battle(attacker):
                        was_deleted = self.turn_player.delete_permanent(attacker, is_battle=True)
                    else:
                        was_deleted = False
                        self.logger.log(f"[Battle] {self._perm_ref(attacker)} survives — can't be destroyed by battle")
                    if has_retaliation and was_deleted:
                        self.logger.log(f"[Retaliation] {self._perm_ref(attacker)} retaliates!")
                        self._emit('keyword_trigger', source_card_id=self._card_id(attacker.top_card),
                                   keyword='Retaliation', card_name=self._card_name(attacker.top_card))
                        self.opponent_player.delete_permanent(target, is_battle=True, is_opponent_effect=True)
                else:
                    # Tie: both are winners AND losers (DCGO). Check CanBeDestroyedByBattle independently.
                    if self._can_be_destroyed_by_battle(target):
                        self.opponent_player.delete_permanent(target, is_battle=True)
                    else:
                        self.logger.log(f"[Battle] {self._perm_ref(target)} survives tie — can't be destroyed by battle")
                    if self._can_be_destroyed_by_battle(attacker):
                        self.turn_player.delete_permanent(attacker, is_battle=True)
                    else:
                        self.logger.log(f"[Battle] {self._perm_ref(attacker)} survives tie — can't be destroyed by battle")
                self.execute_effects(EffectTiming.OnEndBattle, {"attacker": attacker, "defender": target})

        # DCGO order: OnEndAttack fires FIRST, then clear "until end of attack" state.
        # This lets OnEndAttack effects still see end-of-attack modifiers.
        self.execute_effects(EffectTiming.OnEndAttack)

        attacker.clear_attack_state()
        self.modifiers.clear_expiry('end_of_attack')

        # If we returned to EndOfTurnAction (from Vortex/Overclock attack), check for more
        if self.current_phase == GamePhase.EndOfTurnAction:
            if not self._has_end_of_turn_keywords():
                self.next_phase()
            return

        self.check_turn_end()

    def switch_attack_target(self, new_target: Permanent):
        """Redirect the current attack to a different target (DCGO: SwitchDefender)."""
        if self.pending_attack is None:
            return
        if self.modifiers.has_modifier(self.pending_attack.attacker,
                                       ModifierType.CANNOT_SWITCH_ATTACK_TARGET):
            self.logger.log("[Redirect] Attack target cannot be switched!")
            return
        old_ref = self._perm_ref(self.pending_attack.effective_target) if isinstance(
            self.pending_attack.effective_target, Permanent) else "Player"
        self.pending_attack.effective_target = new_target
        self.logger.log(
            f"[Redirect] Attack redirected from {old_ref} to {self._perm_ref(new_target)}")
        self.execute_effects(EffectTiming.OnAttackTargetChanged,
                             {"attacker": self.pending_attack.attacker, "new_target": new_target})

    def _fire_opponent_attack_observers(self, attacker: Permanent):
        """Fire effects on opponent permanents that observe attacks (redirect window).

        Scans the opponent player's battle area for effects with
        _is_when_attacked_observer=True and fires them. This is the
        DCGO SwitchDefender / redirect window. Uses Permanent.effect_list()
        which includes card source effects, linked effects, and granted effects.
        """
        from ..data.enums import EffectTiming as _ET
        opponent = self.opponent_player
        if opponent is None:
            return
        for perm in list(opponent.battle_area):
            for effect in perm.effect_list(_ET.NoTiming):
                if not getattr(effect, '_is_when_attacked_observer', False):
                    continue
                if not effect.on_process_callback:
                    continue
                if not effect.can_activate_this_turn():
                    continue
                effect.set_effect_source_permanent(perm)
                context = {
                    'game': self, 'player': opponent, 'permanent': perm,
                    'card': effect.effect_source_card,
                    'attacker': attacker,
                    'turn_player': self.turn_player,
                    'opponent_player': opponent,
                }
                if effect.can_use_condition and not effect.can_use_condition(context):
                    continue
                effect.record_activation()
                effect.on_process_callback(context)
