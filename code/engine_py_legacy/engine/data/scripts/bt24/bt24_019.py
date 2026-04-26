from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_019(CardScript):
    """BT24-019 Kamemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement: Lv.2 with [TS] trait for cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-019 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Your Turn] When this Digimon would digivolve into a blue Digimon card with the [TS] trait, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-019 Change digivolution cost")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into a blue Digimon card with the [TS] trait, reduce the digivolution cost by 1.")
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            # Only reduce cost when the digivolving target is blue and has [TS] trait
            target_card = context.get('card_source') or context.get('digivolving_card')
            if target_card is not None:
                colors = getattr(target_card, 'card_colors', []) or []
                traits = getattr(target_card, 'card_traits', []) or []
                is_blue = any('Blue' in str(c) for c in colors)
                has_ts = any('TS' in t for t in traits)
                if not (is_blue and has_ts):
                    return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: jamming (inherited)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-019 Jamming")
        effect2.set_effect_description("Jamming")
        effect2.is_inherited_effect = True
        effect2._is_jamming = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
