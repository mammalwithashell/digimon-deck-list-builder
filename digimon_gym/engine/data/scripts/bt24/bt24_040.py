from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_040(CardScript):
    """Auto-transpiled from DCGO BT24_040.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have 3 or fewer security cards, reduce the play cost by 5.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-040 Reduce play cost (5)")
        effect0.set_effect_description("When this card would be played, if you have 3 or fewer security cards, reduce the play cost by 5.")

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
        effect1.set_effect_name("BT24-040 Play Cost -5")
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
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-040 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-040 Effect")
        effect3.set_effect_description("Effect")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave the battle area other than by your effects, by placing 1 other Digimon with no digivolution cards as the bottom security card, they don't leave.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-040 By placing a sourceless Digimon to Security, your [TS] digimon won't leave the field")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave the battle area other than by your effects, by placing 1 other Digimon with no digivolution cards as the bottom security card, they don't leave.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_040_AT")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
