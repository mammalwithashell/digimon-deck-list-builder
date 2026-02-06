from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_068(CardScript):
    """Auto-transpiled from DCGO BT14_068.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete up to 7 play cost's total worth of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-068 Delete Digimon")
        effect0.set_effect_description("[When Digivolving] Delete up to 7 play cost's total worth of your opponent's Digimon.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Delete"""
            # target_permanent.delete()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn][Once Per Turn] Reveal the top 3 cards of your deck. You may play up to 7 play cost's total worth of cards with the [D-Brigade] or [DigiPolice] trait among them without paying the costs. Trash the rest.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-068 Reveal the top 3 cards of deck")
        effect1.set_effect_description("[End of Your Turn][Once Per Turn] Reveal the top 3 cards of your deck. You may play up to 7 play cost's total worth of cards with the [D-Brigade] or [DigiPolice] trait among them without paying the costs. Trash the rest.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Reveal_BT14_068")

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "D-Brigade" in target traits
            # Check trait: "DigiPolice" in target traits
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Play Card, Reveal And Select"""
            # play_card_from_hand_or_trash()
            # reveal_top_cards_and_select()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
