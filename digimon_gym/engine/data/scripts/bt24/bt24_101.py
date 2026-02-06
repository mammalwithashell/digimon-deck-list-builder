from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_101(CardScript):
    """Auto-transpiled from DCGO BT24_101.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-101 Effect")
        effect0.set_effect_description("Effect")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-101 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When your security stack is removed from, trash your opponent's top security card.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-101 Trash Opponent's top security")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When your security stack is removed from, trash your opponent's top security card.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_101_AT_Trash_sec")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your [TS] trait Digimon or Tamers would leave the battle area, by trashing your top security card, they don't leave.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-101 By trashing top security, card doesn't leave")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When any of your [TS] trait Digimon or Tamers would leave the battle area, by trashing your top security card, they don't leave.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_101_AT_Protect_TS")

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
