from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_041(CardScript):
    """BT19-041 Dynasmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing the top card of your security stack, 1 of your Digimon gains <Blocker> and gets +6000 DP until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-041 Trash your top security, one of you Digimon gets Blocker and +6000DP")
        effect0.set_effect_description("[On Play] By trashing the top card of your security stack, 1 of your Digimon gains <Blocker> and gets +6000 DP until the end of your opponent's turn.")
        effect0.is_optional = True
        effect0.is_on_play = True
        effect0._is_blocker = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +6000, Gain Keyword Blocker, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(6000)
            if perm:
                perm.grant_keyword('_is_blocker')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing the top card of your security stack, 1 of your Digimon gains <Blocker> and gets +6000 DP until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-041 Trash your top security, one of you Digimon gets Blocker and +6000DP")
        effect1.set_effect_description("[When Digivolving] By trashing the top card of your security stack, 1 of your Digimon gains <Blocker> and gets +6000 DP until the end of your opponent's turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +6000, Gain Keyword Blocker, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(6000)
            if perm:
                perm.grant_keyword('_is_blocker')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns][Once Per Turn] When this Digimon would leave the battle area, if you have 2 or fewer security cards, <Recovery +1>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-041 Recovery +1")
        effect2.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area, if you have 2 or fewer security cards, <Recovery +1>.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Recovery_Dynasmon_BT19_041")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
