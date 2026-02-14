from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_043(CardScript):
    """BT23-043"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-043 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Base' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by flipping your top face-up security card face down, it doesn't leave.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-043 By flipping your top face up security card face down, prevent remove from field")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by flipping your top face-up security card face down, it doesn't leave.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23_AT")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your [Royal Base] trait Digimon would leave the battle area other than by your effects, by flipping your top face-up security card face down, 1 of those Digimon doesn't leave.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-043 By flipping your top face up security card face down, prevent remove from field")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When any of your [Royal Base] trait Digimon would leave the battle area other than by your effects, by flipping your top face-up security card face down, 1 of those Digimon doesn't leave.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT23_ESS_AT")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
