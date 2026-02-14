from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_050(CardScript):
    """BT23-050"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-050 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Armadillomon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Armadillomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Armadillomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-050 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-050 Give -2000 DP, then if its your turn, you may DNA")
        effect2.set_effect_description("[On Play] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-050 Give -2000 DP, then if its your turn, you may DNA")
        effect3.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
