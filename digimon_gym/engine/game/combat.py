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

        # <Alliance>: check if attacker has Alliance and suspendable allies exist
        if attacker.has_keyword('_is_alliance'):
            has_alliance_targets = any(
                perm is not attacker and perm.is_digimon and not perm.is_suspended
                for perm in self.turn_player.battle_area
            )
            if has_alliance_targets:
                self.current_phase = GamePhase.AllianceTiming
                return  # Park for Alliance decision

        # No Alliance — enter counter timing (DCGO: counter before block)
        self._enter_counter_timing()

    def _enter_counter_timing(self):
        """Check for counter opportunities and enter CounterTiming if any exist."""
        pa = self.pending_attack
        if pa is None or pa.is_end_attack:
            self._resolve_battle()
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
        """Run security check loop for an attacker against a defending player."""
        self.execute_effects(EffectTiming.OnSecurityCheck, {"attacker": attacker})
        sa_mod = attacker.security_attack_modifier()
        num_checks = max(0, 1 + sa_mod)
        for _ in range(num_checks):
            if self.game_over:
                return False
            if attacker not in self.turn_player.battle_area:
                return False
            result = defender_player.security_attack(attacker)
            if result == AttackResolution.AttackerDeleted:
                self.turn_player.delete_permanent(attacker, is_battle=True)
                return False
            elif result == AttackResolution.GameEnd:
                self.declare_winner(self.turn_player)
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

        if isinstance(target, Player):
            self._execute_security_checks(attacker, target)
        elif isinstance(target, Permanent):
            self.execute_effects(EffectTiming.OnStartBattle, {"attacker": attacker, "defender": target})
            a_dp = attacker.dp or 0
            t_dp = target.dp or 0
            attacker_wins = a_dp > t_dp
            defender_wins = a_dp < t_dp
            tie = a_dp == t_dp

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
                was_deleted = self.opponent_player.delete_permanent(target, is_battle=True)
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
                was_deleted = self.turn_player.delete_permanent(attacker, is_battle=True)
                if has_retaliation and was_deleted:
                    self.logger.log(f"[Retaliation] {self._perm_ref(attacker)} retaliates!")
                    self._emit('keyword_trigger', source_card_id=self._card_id(attacker.top_card),
                               keyword='Retaliation', card_name=self._card_name(attacker.top_card))
                    self.opponent_player.delete_permanent(target, is_battle=True, is_opponent_effect=True)
            else:
                # Tie: both deleted
                self.opponent_player.delete_permanent(target, is_battle=True)
                self.turn_player.delete_permanent(attacker, is_battle=True)
            self.execute_effects(EffectTiming.OnEndBattle, {"attacker": attacker, "defender": target})

        # Clear attacker state before end-of-attack effects
        attacker.clear_attack_state()
        self.modifiers.clear_expiry('end_of_attack')

        self.execute_effects(EffectTiming.OnEndAttack)

        # If we returned to EndOfTurnAction (from Vortex/Overclock attack), check for more
        if self.current_phase == GamePhase.EndOfTurnAction:
            if not self._has_end_of_turn_keywords():
                self.next_phase()
            return

        self.check_turn_end()
