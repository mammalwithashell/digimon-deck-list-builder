from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_048(CardScript):
    """Auto-transpiled from DCGO BT14_048.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If attacked a Digimon with higher DP than this Digimon, this Digimon may digivolve into 1 level 6 Digimon card with [Leomon] in its name in your hand for a digivolution cost of 6, ignoring its digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-048 Digivolve this Digimon")
        effect0.set_effect_description("[When Attacking] If attacked a Digimon with higher DP than this Digimon, this Digimon may digivolve into 1 level 6 Digimon card with [Leomon] in its name in your hand for a digivolution cost of 6, ignoring its digivolution requirements.")
        effect0.is_optional = True
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check name: "Leomon" in card name
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
        effect1.set_effect_name("BT14-048 DP modifier")
        effect1.set_effect_description("DP modifier")
        # Static DP modifier
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
