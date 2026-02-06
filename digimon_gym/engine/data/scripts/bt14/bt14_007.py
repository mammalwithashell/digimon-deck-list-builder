from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_007(CardScript):
    """Auto-transpiled from DCGO BT14_007.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, this Digimon may digivolve into [Greymon] in your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-007 This Digimon digivolves")
        effect0.set_effect_description("[Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, this Digimon may digivolve into [Greymon] in your hand without paying the cost.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check name: "Tai Kamiya" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Digivolve"""
            # digivolve_into(target_card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-007 DP modifier")
        effect1.set_effect_description("DP modifier")
        # Static DP modifier
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
