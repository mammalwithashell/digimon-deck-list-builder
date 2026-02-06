from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_067(CardScript):
    """Auto-transpiled from DCGO BT14_067.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-067 Reveal the top 3 cards of opponent's deck")
        effect0.set_effect_description("[On Play] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Delete, Reveal And Select"""
            # target_permanent.delete()
            # reveal_top_cards_and_select()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-067 Reveal the top 3 cards of opponent's deck")
        effect1.set_effect_description("[When Digivolving] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Delete, Reveal And Select"""
            # target_permanent.delete()
            # reveal_top_cards_and_select()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
