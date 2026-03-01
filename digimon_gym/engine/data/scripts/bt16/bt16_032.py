from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_032(CardScript):
    """BT16-032 Sheepmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: collision
        # Collision
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-032 Collision")
        effect0.set_effect_description("Collision")
        effect0._is_collision = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: armor_purge
        # Armor Purge
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-032 Armor Purge")
        effect1.set_effect_description("Armor Purge")
        effect1._is_armor_purge = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [All Turns][Once Per Turn] When an attack target is switched, you may end the attack.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAttackTargetChanged)
        effect2.set_effect_name("BT16-032 End an attack when an attack target is switched.")
        effect2.set_effect_description("[All Turns][Once Per Turn] When an attack target is switched, you may end the attack.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("StopAttack_BT16-032")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-032 Alternate digivolution requirement")
        effect3.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect3._alt_digi_cost = 2

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
