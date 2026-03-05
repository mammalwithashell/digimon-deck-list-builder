from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_034(CardScript):
    """BT11-034 Cutemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "Xros Heart"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Place 1 Digimon card with [Xros Heart] in its traits from your trash under 1 of your Tamers. If you have a Digimon with [Dorulumon] in its name or with [Dorulumon] in its digivolution cards, place up to 2 Digimon cards with [Xros Heart] in their traits from your trash under 1 of your Tamers instead.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-034 Place cards to digivolution cards from trash")
        effect1.set_effect_description("[On Play] Place 1 Digimon card with [Xros Heart] in its traits from your trash under 1 of your Tamers. If you have a Digimon with [Dorulumon] in its name or with [Dorulumon] in its digivolution cards, place up to 2 Digimon cards with [Xros Heart] in their traits from your trash under 1 of your Tamers instead.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
