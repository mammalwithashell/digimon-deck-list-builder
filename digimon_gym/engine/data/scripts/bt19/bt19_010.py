from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_010(CardScript):
    """BT19-010 Shoutmon X4 | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, you may place up to 3 Digimon cards with the [Xros Heart] trait from this Digimon's digivolution cards under 1 of your Tamers.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.WhenRemoveField)
        effect0.set_effect_name("BT19-010 Save up to 3 Digimon cards with the [Xros Heart] trait.")
        effect0.set_effect_description("[All Turns] When this Digimon would leave the battle area, you may place up to 3 Digimon cards with the [Xros Heart] trait from this Digimon's digivolution cards under 1 of your Tamers.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-010 Effect")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
