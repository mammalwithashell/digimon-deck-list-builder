from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_042(CardScript):
    """Auto-transpiled from DCGO BT14_042.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By suspending this Digimon, reveal the top 3 cards of your deck. Add 1 green card among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-042 Suspend this Digimon to reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Play] By suspending this Digimon, reveal the top 3 cards of your deck. Add 1 green card among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check color: CardColor.Green
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Suspend, Add To Hand, Reveal And Select"""
            # target_permanent.suspend()
            # add_card_to_hand()
            # reveal_top_cards_and_select()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
