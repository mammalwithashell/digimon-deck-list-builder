from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_037(CardScript):
    """Auto-transpiled from DCGO BT14_037.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-037 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        # Blast Digivolve
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-037 Recovery +1 and opponent's 1 Digimon reduces DP")
        effect1.set_effect_description("[On Play] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Recovery +1"""
            # card.owner.recover(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-037 Recovery +1 and opponent's 1 Digimon reduces DP")
        effect2.set_effect_description("[When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Recovery +1"""
            # card.owner.recover(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
