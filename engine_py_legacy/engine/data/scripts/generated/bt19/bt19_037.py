from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_037(CardScript):
    """BT19-037 Taomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-037 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If it's your turn, you may use 1 single-color Option card with a cost of 5 or less from your hand without paying the cost. If it's your opponent's turn, 1 of their Digimon gains <Security Attack -1> and can't activate  effects for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-037 If it's your turn, you may use 1 single-color Option")
        effect1.set_effect_description("[On Play] If it's your turn, you may use 1 single-color Option card with a cost of 5 or less from your hand without paying the cost. If it's your opponent's turn, 1 of their Digimon gains <Security Attack -1> and can't activate  effects for the turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Change Security Attack, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If it's your turn, you may use 1 single-color Option card with a cost of 5 or less from your hand without paying the cost. If it's your opponent's turn, 1 of their Digimon gains <Security Attack -1> and can't activate  effects for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-037 If it's your turn, you may use 1 single-color Option")
        effect2.set_effect_description("[When Digivolving] If it's your turn, you may use 1 single-color Option card with a cost of 5 or less from your hand without paying the cost. If it's your opponent's turn, 1 of their Digimon gains <Security Attack -1> and can't activate  effects for the turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Change Security Attack, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] 1 of your opponent's Digimon gets -4000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-037 DP -4000")
        effect3.set_effect_description("[When Attacking] 1 of your opponent's Digimon gets -4000 DP for the turn.")
        effect3.is_inherited_effect = True
        effect3.is_on_attack = True
        effect3.dp_modifier = -4000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-4000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
