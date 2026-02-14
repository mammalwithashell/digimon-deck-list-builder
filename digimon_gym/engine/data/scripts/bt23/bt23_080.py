from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_080(CardScript):
    """BT23-080"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # Gain 1 memory (Tamer)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-080 Gain 1 memory (Tamer)")
        effect0.set_effect_description("Gain 1 memory (Tamer)")
        # [Start of Main] Gain 1 memory if opponent has Digimon

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When any of your Digimon with the [CS] trait would be deleted, by returning this Tamer to the bottom of the deck, place 1 of those Digimon as the top security card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-080 By bouncing this to bottom deck, place 1 digimon about to be deleted to top security")
        effect1.set_effect_description("[All Turns] When any of your Digimon with the [CS] trait would be deleted, by returning this Tamer to the bottom of the deck, place 1 of those Digimon as the top security card.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-080 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
