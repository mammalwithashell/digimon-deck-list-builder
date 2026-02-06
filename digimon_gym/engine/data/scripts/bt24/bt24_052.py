from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_052(CardScript):
    """Auto-transpiled from DCGO BT24_052.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnMove
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-052 Effect")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-052 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon with [Diaboromon] in its text would leave the battle area, by deleting 1 of your other [Diaboromon], it doesn't leave.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-052 Delete 1 of your other [Diaboromon] to prevent this Digimon from leaving")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When this Digimon with [Diaboromon] in its text would leave the battle area, by deleting 1 of your other [Diaboromon], it doesn't leave.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_BT24_052")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
