from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_033(CardScript):
    """Auto-transpiled from DCGO BT14_033.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-033 This Digimon digivolves into Digimon card in security")
        effect0.set_effect_description("[Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "Vaccine" in target traits
            # Check color: CardColor.Yellow
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Play Card, Trash From Hand, Add To Security"""
            # play_card_from_hand_or_trash()
            # card.owner.trash_from_hand(count)
            # card.owner.add_to_security()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddSecurity
        # [Your Turn][Once Per Turn] When a card is added to your security stack, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-033 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When a card is added to your security stack, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT14_033")

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Gain 1 memory"""
            # card.owner.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
