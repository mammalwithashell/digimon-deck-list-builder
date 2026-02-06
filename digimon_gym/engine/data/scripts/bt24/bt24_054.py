from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_054(CardScript):
    """Auto-transpiled from DCGO BT24_054.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your [Shuu Yulin]s are played, this Digimon may digivolve into [Hisyaryumon] in the hand for a digivolution cost of 3, ignoring digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-054 Digivolve into a [Hisyaryumon]] in the hand")
        effect0.set_effect_description("[Your Turn] When any of your [Shuu Yulin]s are played, this Digimon may digivolve into [Hisyaryumon] in the hand for a digivolution cost of 3, ignoring digivolution requirements.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspendeds, suspend 1 of your opponent's Digimon or Tamers with as high or lower a play cost as this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-054 Suspend opponent's Digimon or Tamers with play cost less than this Digimon.")
        effect1.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspendeds, suspend 1 of your opponent's Digimon or Tamers with as high or lower a play cost as this Digimon.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_054_Inherited")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
