from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_091(CardScript):
    """Auto-transpiled from DCGO BT14_091.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Trash any 2 digivolution cards from your opponent's Digimon. Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no Digimon with more digivolution cards than the chosen Digimon, unsuspend it.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-091 Trash Digivolution Cards")
        effect0.set_effect_description("[Main] Trash any 2 digivolution cards from your opponent's Digimon. Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no Digimon with more digivolution cards than the chosen Digimon, unsuspend it.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check name: "Joe Kido" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash Digivolution Cards"""
            # target.trash_digivolution_cards(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
