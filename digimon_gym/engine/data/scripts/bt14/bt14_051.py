from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_051(CardScript):
    """Auto-transpiled from DCGO BT14_051.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent's Turn][Once Per Turn] By suspending 1 of your Digimon, reveal the top 5 cards of your deck. Add 2 green Digimon cards among them to the hand. Return the rest to the bottom of deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-051 Suspend your 1 Digimon to reveal the top 5 cards of deck")
        effect0.set_effect_description("[End of Opponent's Turn][Once Per Turn] By suspending 1 of your Digimon, reveal the top 5 cards of your deck. Add 2 green Digimon cards among them to the hand. Return the rest to the bottom of deck.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Reveal_BT14_051")

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
