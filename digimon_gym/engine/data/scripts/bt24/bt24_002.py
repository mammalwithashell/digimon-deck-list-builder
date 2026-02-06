from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_002(CardScript):
    """Auto-transpiled from DCGO BT24_002.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] By paying 1 cost, this blue Digimon with the [TS] trait unsuspends.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-002 By paying 1, this digimon may unsuspend")
        effect0.set_effect_description("[End of Your Turn] [Once Per Turn] By paying 1 cost, this blue Digimon with the [TS] trait unsuspends.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("EOYT_BT24_002")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
