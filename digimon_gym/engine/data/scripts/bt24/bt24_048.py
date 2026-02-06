from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_048(CardScript):
    """Auto-transpiled from DCGO BT24_048.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-048 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-048 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-048 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When this Digimon deletes an opponent's Digimon in battle, it may unsuspend.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-048 Unsuspend this Digimon")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes an opponent's Digimon in battle, it may unsuspend.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_048_Inherited")
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
