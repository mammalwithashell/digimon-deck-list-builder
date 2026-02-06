from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_030(CardScript):
    """Auto-transpiled from DCGO BT24_030.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if your opponent has 2 or more Digimon, reduce the play cost by 5.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-030 Reduce play cost (5)")
        effect0.set_effect_description("When this card would be played, if your opponent has 2 or more Digimon, reduce the play cost by 5.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-030 Play Cost -5")
        effect1.set_effect_description("Effect")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-030 Bottom deck all opponent digimon with lowest digivolution cards")
        effect2.set_effect_description("[On Play] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-030 Bottom deck all opponent digimon with lowest digivolution cards")
        effect3.set_effect_description("[When Digivolving] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-030 Unsuspend this digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_030_AT")

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-030 By suspending this digimon, your [TS]/[Aqua]/[Sea Animal] digimon wont leave the field")
        effect5.set_effect_description("[All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave.")
        effect5.is_optional = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
