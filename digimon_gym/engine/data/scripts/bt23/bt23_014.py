from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_014(CardScript):
    """BT23-014"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-014 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-014 Your opponent effects cannot play digimon or tamers from trash until their turn ends")
        effect1.set_effect_description("[On Play] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-014 Your opponent effects cannot play digimon or tamers from trash until their turn ends")
        effect2.set_effect_description("[When Digivolving] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-014 Effect")
        effect3.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.")
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
        # [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-014 Effect")
        effect4.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-014 Effect")
        effect5.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
