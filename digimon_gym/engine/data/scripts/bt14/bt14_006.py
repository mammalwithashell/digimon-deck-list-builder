from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_006(CardScript):
    """Auto-transpiled from DCGO BT14_006.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn] When a Digimon card with the [Dark Animal] or [SoC] trait is trashed from your hand, this Digimon may digivolve into that card.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-006 This Digimon digivolves into discarded card")
        effect0.set_effect_description("[Your Turn] When a Digimon card with the [Dark Animal] or [SoC] trait is trashed from your hand, this Digimon may digivolve into that card.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_hash_string("Digivolve_BT14_006")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "Dark Animal" in target traits
            # Check trait: "DarkAnimal" in target traits
            # Check trait: "SoC" in target traits
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Digivolve"""
            # digivolve_into(target_card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
