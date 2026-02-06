from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_080(CardScript):
    """Auto-transpiled from DCGO BT14_080.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-080 Trash cards from opponent's deck top")
        effect0.set_effect_description("[When Digivolving][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("TrashDeck_BT14_080")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-080 Trash cards from opponent's deck top")
        effect1.set_effect_description("[When Attacking][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashDeck_BT14_080")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If your opponent has 10 or more cards in their trash, this Digimon gains ��Security A. +1�� for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-080 This Digimon gains Security Attack +1")
        effect2.set_effect_description("[When Attacking][Once Per Turn] If your opponent has 10 or more cards in their trash, this Digimon gains ��Security A. +1�� for the turn.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("SecurityAttack+1_BT14_080")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
