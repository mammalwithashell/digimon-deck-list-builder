from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_026(CardScript):
    """BT10-026 DeckerGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: armor_purge
        # Armor Purge
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-026 Armor Purge")
        effect0.set_effect_description("Armor Purge")
        effect0._is_armor_purge = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Digimon card with [Blue Flare] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Deckerdramon] is in this Digimon's digivolution cards, 1 of your opponent's Digimon can't attack or block until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT10-026 Place 1 card to digivolution cards and opponent's 1 Digimon can't attack and block")
        effect1.set_effect_description("[On Play] You may place 1 Digimon card with [Blue Flare] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Deckerdramon] is in this Digimon's digivolution cards, 1 of your opponent's Digimon can't attack or block until the end of your opponent's turn.")
        effect1.is_on_play = True
        effect1._is_cannot_attack = True
        effect1._is_cannot_block = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
                perm.grant_keyword('_is_cannot_block')
            # Prevent target from blocking
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_BLOCK, target_perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Digimon card with [Blue Flare] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Deckerdramon] is in this Digimon's digivolution cards, 1 of your opponent's Digimon can't attack or block until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT10-026 Place 1 card to digivolution cards and opponent's 1 Digimon can't attack and block")
        effect2.set_effect_description("[When Digivolving] You may place 1 Digimon card with [Blue Flare] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Deckerdramon] is in this Digimon's digivolution cards, 1 of your opponent's Digimon can't attack or block until the end of your opponent's turn.")
        effect2.is_when_digivolving = True
        effect2._is_cannot_attack = True
        effect2._is_cannot_block = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
                perm.grant_keyword('_is_cannot_block')
            # Prevent target from blocking
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_BLOCK, target_perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT10-026 Effect")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
