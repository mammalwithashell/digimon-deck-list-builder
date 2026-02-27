from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_100(CardScript):
    """BT8-100 Disaster Blaster"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your opponent's Digimon gets -3000 DP for the turn. If you have a Digimon in play with 2 or more colors, or with 2 or more colors in one of its digivolution cards, 1 of your opponent's Digimon gets -6000 DP instead.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-100 Effect")
        effect0.set_effect_description("[Main] 1 of your opponent's Digimon gets -3000 DP for the turn. If you have a Digimon in play with 2 or more colors, or with 2 or more colors in one of its digivolution cards, 1 of your opponent's Digimon gets -6000 DP instead.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
