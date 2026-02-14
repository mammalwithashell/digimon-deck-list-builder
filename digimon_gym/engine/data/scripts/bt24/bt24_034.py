from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_034(CardScript):
    """BT24-034 Aegiomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Elecmon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Elecmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Elecmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-034 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnMove
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-034 Effect")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-034 Effect")
        effect3.set_effect_description("Effect")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-034 Effect")
        effect4.set_effect_description("Effect")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: barrier
        # Barrier
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-034 Barrier")
        effect5.set_effect_description("Barrier")
        effect5.is_inherited_effect = True
        effect5._is_barrier = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
