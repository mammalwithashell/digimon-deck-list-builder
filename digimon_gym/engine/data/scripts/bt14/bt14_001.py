from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_001(CardScript):
    """Auto-transpiled from DCGO BT14_001.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn][Once Per Turn] When a card is removed from your opponent's security stack, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-001 Draw 1")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When a card is removed from your opponent's security stack, <Draw 1>.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_BT14_001")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Draw 1"""
            # card.owner.draw(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
