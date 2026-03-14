from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_059(CardScript):
    """BT8-059 Kokuwamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Continuous effect: Players can't ignore digivolution requirements
        # (DCGO: ICannotIgnoreDigivolutionConditionEffect)
        # Checked by effect_digivolve_from_hand when ignore_requirements=True
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-059 Players can't ignore digivolution requirements")
        effect0.set_effect_description(
            "[All Turns] Players can't ignore digivolution requirements."
        )
        effect0._cannot_ignore_evo_requirements = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
