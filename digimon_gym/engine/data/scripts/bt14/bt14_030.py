from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_030(CardScript):
    """Auto-transpiled from DCGO BT14_030.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-030 Return Digimons to hand")
        effect0.set_effect_description("[On Play] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Bounce"""
            # target_permanent.return_to_hand()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-030 Return Digimons to hand")
        effect1.set_effect_description("[When Digivolving] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Bounce"""
            # target_permanent.return_to_hand()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnPermamemtReturnedToHand
        # [Your Turn][Once Per Turn] When another Digimon returns to the hand, <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-030 Recovery +1 (Deck)")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When another Digimon returns to the hand, <Recovery +1 (Deck)>.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Recovery_BT14_030")

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Recovery +1"""
            # card.owner.recover(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
