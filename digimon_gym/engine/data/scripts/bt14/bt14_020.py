from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_020(CardScript):
    """Auto-transpiled from DCGO BT14_020.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Trash any 1 digivolution card of 1 of your opponent's Digimon. This Digimon can't be blocked for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-020 Trash digivolution cards and this Digimon gains unblockable")
        effect0.set_effect_description("[Start of Your Main Phase] Trash any 1 digivolution card of 1 of your opponent's Digimon. This Digimon can't be blocked for the turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash Digivolution Cards"""
            # target.trash_digivolution_cards(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [Opponent's Turn] When this Digimon would be deleted, you may play 1 [Gomamon] from its digivolution cards without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-020 Play [Gomamon] from digivolution cards")
        effect1.set_effect_description("[Opponent's Turn] When this Digimon would be deleted, you may play 1 [Gomamon] from its digivolution cards without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_hash_string("PlayDigivolutionCards_BT14_020")

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Play Card"""
            # play_card_from_hand_or_trash()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
