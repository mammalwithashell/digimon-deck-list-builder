from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_025(CardScript):
    """Auto-transpiled from DCGO BT24_025.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn] When any of your other blue Digimon with the [TS] trait unsuspend, this Digimon may digivolve into [Venusmon] in the hand, ignoring level.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-025 Digivolve into [Venusmon] in the hand")
        effect0.set_effect_description("[Your Turn] When any of your other blue Digimon with the [TS] trait unsuspend, this Digimon may digivolve into [Venusmon] in the hand, ignoring level.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 1 of your other Digimon with the [TS] trait may unsuspend.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-025 Unsuspend 1 other TS Digimon")
        effect1.set_effect_description("[End of Your Turn] [Once Per Turn] 1 of your other Digimon with the [TS] trait may unsuspend.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_025_EOYT")

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: jamming
        # Jamming
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-025 Jamming")
        effect2.set_effect_description("Jamming")
        effect2._is_jamming = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
