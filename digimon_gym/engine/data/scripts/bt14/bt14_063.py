from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_063(CardScript):
    """Auto-transpiled from DCGO BT14_063.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Reveal the top 3 cards of your deck. From among them, add 1 card with [Monzaemon] in its name to your hand and play 1 Digimon card with [Numemon] in its name without paying the cost. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-063 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Deletion] Reveal the top 3 cards of your deck. From among them, add 1 card with [Monzaemon] in its name to your hand and play 1 Digimon card with [Numemon] in its name without paying the cost. Return the rest to the bottom of the deck.")
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check name: "Monzaemon" in card name
            # Check name: "Numemon" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Play Card, Add To Hand, Reveal And Select"""
            # play_card_from_hand_or_trash()
            # add_card_to_hand()
            # reveal_top_cards_and_select()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-063 Blocker")
        effect1.set_effect_description("Blocker")
        # TODO: Blocker keyword - this Digimon can block
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
