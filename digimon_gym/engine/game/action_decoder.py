"""Action decoder mixin for the Game class.

Decodes integer actions into game operations. Uses `from __future__ import annotations`
+ TYPE_CHECKING to avoid circular imports.
"""
from __future__ import annotations
from typing import TYPE_CHECKING

from .constants import (
    FIELD_SLOTS, TARGETS_PER_ATTACKER, FIELDS_PER_HAND, EFFECTS_PER_PERM,
    SOURCES_PER_FIELD, BREEDING_SLOT, SECURITY_TARGET, ACTION_SPACE_SIZE,
    SEL_MY_FIELD_START, SEL_TRASH_START, SEL_TRASH_END,
)
from ..data.enums import GamePhase, EffectTiming

if TYPE_CHECKING:
    from ..core.permanent import Permanent
    from ..core.player import Player


class ActionDecoderMixin:
    """Action decoding and related helper methods."""

    def decode_action(self, action_id: int, player_id: int):
        """Decode an integer action and execute the corresponding game action."""
        phase = self.current_phase

        if phase == GamePhase.Mulligan:
            self._decode_mulligan(action_id)
        elif phase == GamePhase.Main:
            self._decode_main(action_id)
        elif phase == GamePhase.Breeding:
            self._decode_breeding(action_id)
        elif phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                       GamePhase.SelectHand, GamePhase.SelectReveal,
                       GamePhase.SelectEffectChoice, GamePhase.SelectSecurity):
            self._decode_selection(action_id)
        elif phase == GamePhase.BlockTiming:
            self._decode_block(action_id)
        elif phase == GamePhase.CounterTiming:
            self._decode_counter(action_id)
        elif phase == GamePhase.SelectTrash:
            self._decode_trash_selection(action_id)
        elif phase == GamePhase.SelectSource:
            self._decode_source_selection(action_id)
        elif phase == GamePhase.EndOfTurnAction:
            self._decode_end_of_turn_action(action_id)
        elif phase == GamePhase.AllianceTiming:
            self._decode_alliance(action_id)

    def _decode_mulligan(self, action_id: int):
        if action_id == 0:
            self.action_keep_opening_hand()
        elif action_id == 1:
            self.action_mulligan_opening_hand()

    def _decode_main(self, action_id: int):
        if 0 <= action_id <= 29:
            self.action_play_card(action_id)
        elif action_id == 62:
            self.action_pass_turn()
        elif 100 <= action_id <= 399:
            normalized = action_id - 100
            attacker_idx = normalized // TARGETS_PER_ATTACKER
            target_idx = normalized % TARGETS_PER_ATTACKER
            if target_idx == SECURITY_TARGET:
                self.action_attack_player(attacker_idx)
            else:
                self.action_attack_digimon(attacker_idx, target_idx)
        elif 63 <= action_id <= 92:
            hand_idx = action_id - 63
            self._initiate_dna_digivolve(hand_idx)
        elif 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // FIELDS_PER_HAND
            field_idx = normalized % FIELDS_PER_HAND
            if field_idx == BREEDING_SLOT:
                self.action_digivolve_breeding(hand_idx)
            else:
                self.action_digivolve(field_idx, hand_idx)
        elif 1000 <= action_id <= 1999:
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            effect_idx = normalized % EFFECTS_PER_PERM
            if perm_idx < len(self.turn_player.battle_area):
                perm = self.turn_player.battle_area[perm_idx]
                if effect_idx == 0 and perm.has_keyword('_is_training'):
                    self._execute_training(perm, self.turn_player)
                elif effect_idx == 1 and self._has_delay_effect(perm):
                    self._execute_delay(perm, self.turn_player)

    def _decode_breeding(self, action_id: int):
        if action_id == 60:
            self.action_hatch()
        elif action_id == 61:
            self.action_move_from_breeding()
        elif action_id == 62:
            self.action_breeding_pass()
        elif 1000 <= action_id <= 1999:
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            if perm_idx == BREEDING_SLOT and self.turn_player.breeding_area:
                perm = self.turn_player.breeding_area
                if perm.has_keyword('_is_training'):
                    self._execute_training(perm, self.turn_player)

    def _recover_from_stale_selection(self):
        """Guard against stale selection state after a callback."""
        if (self._is_selection_phase(self.current_phase)
                and self.pending_selection is None):
            self.current_phase = GamePhase.Main
        if (self.pending_selection is not None
                and not self.pending_selection.valid_indices
                and not getattr(self.pending_selection, 'is_optional', False)):
            self.logger.log(
                f"[Recovery] Empty valid_indices — clearing")
            self.pending_selection = None
            if self._is_selection_phase(self.current_phase):
                self.current_phase = GamePhase.Main

    @staticmethod
    def _is_selection_phase(phase: GamePhase) -> bool:
        """Return True if the phase is a selection/interrupt phase."""
        return phase in (
            GamePhase.SelectTarget, GamePhase.SelectMaterial,
            GamePhase.SelectTrash, GamePhase.SelectSource,
            GamePhase.SelectHand, GamePhase.SelectReveal,
            GamePhase.SelectEffectChoice, GamePhase.SelectSecurity,
        )

    def _decode_selection(self, action_id: int):
        """Handle target or material selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        # Optional selection: action 62 = decline/pass
        if action_id == 62 and getattr(ps, 'is_optional', False):
            prev_phase = ps.previous_phase
            on_decline = ps.on_decline
            self.pending_selection = None
            self.revealed_cards = []
            self.current_phase = prev_phase
            self.active_player = None
            if on_decline:
                on_decline()
            self._recover_from_stale_selection()
            self._check_deferred_turn_end()
            return

        if ps.valid_indices and action_id not in ps.valid_indices:
            return  # invalid selection, ignore

        callback = ps.callback
        prev_phase = ps.previous_phase
        self.pending_selection = None
        self.current_phase = prev_phase
        self.active_player = None
        callback(action_id)
        self._recover_from_stale_selection()
        self._check_deferred_turn_end()

    def _decode_block(self, action_id: int):
        """Handle the defender's blocking decision during an attack."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            self.logger.log("[Block] Declined to block")
            self._resolve_battle()

        elif 100 <= action_id <= 100 + FIELD_SLOTS - 1:
            from ..interfaces.modifiers import ModifierType as _MT
            blocker_idx = action_id - 100
            defender = self.opponent_player

            if blocker_idx >= len(defender.battle_area):
                return

            blocker = defender.battle_area[blocker_idx]
            if not blocker.can_block(pa.attacker):
                return

            # Check CanSwitchAttackTarget
            if self.modifiers.has_modifier(pa.attacker, _MT.CANNOT_SWITCH_ATTACK_TARGET):
                self.logger.log(f"[Block] Attack target cannot be switched!")
                self._resolve_battle()
                return

            blocker.suspend()
            pa.is_blocked = True
            pa.blocker = blocker
            pa.effective_target = blocker

            self.logger.log(f"[Block] {self._perm_ref(blocker)} blocks the attack")

            self.execute_effects(EffectTiming.OnBlockAnyone, {"blocker": blocker})
            self.execute_effects(EffectTiming.OnAttackTargetChanged, {"attacker": pa.attacker, "new_target": blocker})
            self.execute_effects(EffectTiming.OnEndBlockDesignation, {"blocker": blocker})

            self._resolve_battle()

    def _decode_counter(self, action_id: int):
        """Handle the defender's counter/blast digivolve decision."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            self.logger.log("[Counter] Declined counter")
            self._check_blockers_or_continue()

        elif 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // FIELDS_PER_HAND
            field_idx = normalized % FIELDS_PER_HAND

            defender = self.opponent_player

            if hand_idx >= len(defender.hand_cards):
                self._check_blockers_or_continue()
                return
            if field_idx >= len(defender.battle_area):
                self._check_blockers_or_continue()
                return

            card = defender.hand_cards[hand_idx]
            perm = defender.battle_area[field_idx]

            effects = card.effect_list(EffectTiming.NoTiming)
            has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
            if not has_blast:
                self._check_blockers_or_continue()
                return

            self.logger.log(f"[Counter] Blast Digivolve: {self._card_ref(card)} onto {self._perm_ref(perm)}")

            defender.hand_cards.remove(card)
            perm.add_card_source(card)

            self.execute_effects(EffectTiming.OnCounterTiming, {"counter_card": card, "counter_permanent": perm})
            self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})

            self._check_blockers_or_continue()

    def _decode_trash_selection(self, action_id: int):
        """Handle trash card selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        if action_id == 62 and getattr(ps, 'is_optional', False):
            on_decline = ps.on_decline
            prev_phase = ps.previous_phase
            self.pending_selection = None
            self.current_phase = prev_phase
            self.active_player = None
            if on_decline:
                on_decline()
            self._recover_from_stale_selection()
            self._check_deferred_turn_end()
            return

        if SEL_TRASH_START <= action_id <= SEL_TRASH_END:
            idx = action_id - SEL_TRASH_START
            selecting = ps.selecting_player
            if idx < len(selecting.trash_cards):
                callback = ps.callback
                prev_phase = ps.previous_phase
                self.pending_selection = None
                self.current_phase = prev_phase
                self.active_player = None
                callback(idx)
                self._recover_from_stale_selection()
                self._check_deferred_turn_end()

    def _decode_source_selection(self, action_id: int):
        """Handle digivolution source selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        if action_id == 62 and getattr(ps, 'is_optional', False):
            on_decline = ps.on_decline
            prev_phase = ps.previous_phase
            self.pending_selection = None
            self.current_phase = prev_phase
            self.active_player = None
            if on_decline:
                on_decline()
            self._recover_from_stale_selection()
            self._check_deferred_turn_end()
            return

        if 2000 <= action_id < ACTION_SPACE_SIZE:
            normalized = action_id - 2000
            field_idx = normalized // SOURCES_PER_FIELD
            source_idx = normalized % SOURCES_PER_FIELD

            selecting = ps.selecting_player
            if field_idx < len(selecting.battle_area):
                perm = selecting.battle_area[field_idx]
                if source_idx < len(perm.card_sources):
                    callback = ps.callback
                    prev_phase = ps.previous_phase
                    self.pending_selection = None
                    self.current_phase = prev_phase
                    self.active_player = None
                    callback(action_id)
                    self._recover_from_stale_selection()
                    self._check_deferred_turn_end()

    def _execute_training(self, perm: "Permanent", owner: "Player"):
        """Execute <Training>: suspend this Digimon and place top deck card at bottom of digi stack."""
        if not owner.library_cards:
            return
        perm.suspend()
        top_card = owner.library_cards.pop(0)
        perm.add_card_source_bottom(top_card)
        self.logger.log(f"[Training] {self._perm_ref(perm)} trains: placed {self._card_ref(top_card)} at bottom of digi stack")

    def _has_delay_effect(self, perm: "Permanent") -> bool:
        """Check if a permanent has a <Delay> effect that can be activated."""
        for source in perm.card_sources:
            all_effects = source.effect_list(EffectTiming.NoTiming)
            for effect in all_effects:
                if getattr(effect, '_is_delay', False):
                    return True
        return False

    def _get_delay_callback(self, perm: "Permanent"):
        """Find the delayed effect callback."""
        for source in perm.card_sources:
            effects = source.effect_list(EffectTiming.NoTiming)
            for idx, effect in enumerate(effects):
                if getattr(effect, '_is_delay', False):
                    if idx + 1 < len(effects):
                        next_effect = effects[idx + 1]
                        if next_effect.on_process_callback:
                            return next_effect
        return None

    def _execute_delay(self, perm: "Permanent", owner: "Player"):
        """Execute <Delay>: trash this card from battle area and activate the delayed effect."""
        delay_effect = self._get_delay_callback(perm)
        perm_ref = self._perm_ref(perm)

        if perm in owner.battle_area:
            owner.battle_area.remove(perm)
        for source in perm.card_sources:
            owner.trash_cards.append(source)

        self.logger.log(f"[Delay] {perm_ref} trashed from battle area to activate delayed effect")

        if delay_effect and delay_effect.on_process_callback:
            context = {
                "game": self,
                "player": owner,
                "permanent": perm,
                "card": delay_effect.effect_source_card,
                "turn_player": self.turn_player,
                "opponent_player": self.opponent_player,
            }
            if delay_effect.can_use_condition is None or delay_effect.can_use_condition(context):
                self._log_effect_activation(delay_effect, EffectTiming.AfterEffectsActivate)
                delay_effect.record_activation()
                delay_effect.on_process_callback(context)

    def _decode_end_of_turn_action(self, action_id: int):
        """Handle end-of-turn keyword actions (Vortex, Overclock)."""
        if action_id == 62:
            self.next_phase()
            return

        if 100 <= action_id <= 399:
            normalized = action_id - 100
            attacker_idx = normalized // TARGETS_PER_ATTACKER
            target_idx = normalized % TARGETS_PER_ATTACKER
            if attacker_idx < len(self.turn_player.battle_area) and target_idx < len(self.opponent_player.battle_area):
                attacker = self.turn_player.battle_area[attacker_idx]
                target = self.opponent_player.battle_area[target_idx]
                self.logger.log(f"[Vortex] End-of-turn attack!")
                self.resolve_attack(attacker, target, is_vortex=True,
                                    return_phase=GamePhase.EndOfTurnAction)

        elif 1000 <= action_id <= 1999:
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            if perm_idx < len(self.turn_player.battle_area):
                overclock_perm = self.turn_player.battle_area[perm_idx]
                self._initiate_overclock(overclock_perm)

    def _initiate_overclock(self, overclock_perm: "Permanent"):
        """Overclock step 1: select a Token or other Digimon to sacrifice."""
        valid = []
        for i, perm in enumerate(self.turn_player.battle_area):
            if perm is not overclock_perm and (perm.is_token or perm.is_digimon):
                valid.append(SEL_MY_FIELD_START + i)
        if not valid:
            return

        def on_sacrifice_selected(action_id: int):
            idx = action_id - SEL_MY_FIELD_START
            if 0 <= idx < len(self.turn_player.battle_area):
                sacrifice = self.turn_player.battle_area[idx]
                self.logger.log(f"[Overclock] Sacrificed {self._perm_ref(sacrifice)}")
                self.turn_player.delete_permanent(sacrifice)
                self.logger.log(f"[Overclock] End-of-turn attack on player!")
                self.resolve_attack(overclock_perm, self.opponent_player, without_suspend=True,
                                    return_phase=GamePhase.EndOfTurnAction)

        self.request_selection(
            GamePhase.SelectTarget, self.turn_player, on_sacrifice_selected,
            valid, is_optional=True)

    def _decode_alliance(self, action_id: int):
        """Handle Alliance target selection during attack."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            self._check_blockers_or_continue()
            return

        if 100 <= action_id <= 111:
            ally_idx = action_id - 100
            if ally_idx < len(self.turn_player.battle_area):
                ally = self.turn_player.battle_area[ally_idx]
                if ally is not pa.attacker and ally.is_digimon and not ally.is_suspended:
                    ally_dp = ally.dp or 0
                    ally.suspend()
                    pa.attacker.change_dp(ally_dp)
                    pa.attacker._temp_sa_modifier += 1
                    self.logger.log(f"[Alliance] Suspended {self._perm_ref(ally)}, adding {ally_dp} DP and SA+1")

                    has_more = any(
                        perm is not pa.attacker and perm.is_digimon and not perm.is_suspended
                        for perm in self.turn_player.battle_area
                    )
                    if has_more:
                        return  # Stay in AllianceTiming for another choice

            # No more allies or invalid — proceed to blocker check
            self._check_blockers_or_continue()
