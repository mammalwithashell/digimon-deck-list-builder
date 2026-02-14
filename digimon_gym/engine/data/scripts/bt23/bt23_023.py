from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_023(CardScript):
    """BT23-023"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-023 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower blue or [CS] trait Digimon card from its digivolution cards without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-023 Play 1 level 4 or lower Blue or [CS] digimon from sources")
        effect1.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower blue or [CS] trait Digimon card from its digivolution cards without paying the cost.")
        effect1.set_hash_string("BT23-023_AT")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower blue or [CS] trait Digimon card from its digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-023 Play 1 level 4 or lower Blue or [CS] digimon from sources")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower blue or [CS] trait Digimon card from its digivolution cards without paying the cost.")
        effect2.is_inherited_effect = True
        effect2.set_hash_string("BT23-023_ESS_AT")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
