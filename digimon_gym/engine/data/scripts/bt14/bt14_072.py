from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_072(CardScript):
    """Auto-transpiled from DCGO BT14_072.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-072 Return 1 card from trash to hand and trash 1 card from hand")
        effect0.set_effect_description("[On Play] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check trait: "Dark Animal" in target traits
            # Check trait: "DarkAnimal" in target traits
            # Check color: CardColor.Purple
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash From Hand, Add To Hand"""
            # card.owner.trash_from_hand(count)
            # add_card_to_hand()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-072 Return 1 card from trash to hand and trash 1 card from hand")
        effect1.set_effect_description("[When Attacking] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check trait: "Dark Animal" in target traits
            # Check trait: "DarkAnimal" in target traits
            # Check color: CardColor.Purple
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Trash From Hand, Add To Hand"""
            # card.owner.trash_from_hand(count)
            # add_card_to_hand()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
