from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_034(CardScript):
    """Auto-transpiled from DCGO BT14_034.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-034 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True
        # Security effect: play this card without paying cost
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-034 DP -3000")
        effect1.set_effect_description("[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1.dp_modifier = -3000

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: DP -3000"""
            # target.change_dp(-3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
