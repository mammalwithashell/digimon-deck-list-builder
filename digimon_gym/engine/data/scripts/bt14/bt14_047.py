from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_047(CardScript):
    """Auto-transpiled from DCGO BT14_047.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon. During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-047 Suspend 1 Digimon and opponent's Digimons can't unsuspend")
        effect0.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon. During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-047 Suspend 1 Digimon and opponent's Digimons can't unsuspend")
        effect1.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
