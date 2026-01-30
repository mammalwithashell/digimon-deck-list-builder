from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_15(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect_main = ICardEffect()
        effect_main.set_effect_name("ST1-15 Main Effect")
        effect_main.set_effect_description("[Main] Delete up to 2 of your opponent's Digimon with 4000 DP or less.")

        def activate_main():
            print("Activated Giga Destroyer Main Effect")

        effect_main.set_on_process_callback(activate_main)

        effect_sec = ICardEffect()
        effect_sec.set_effect_name("ST1-15 Security Effect")
        effect_sec.set_effect_description("[Security] Activate this card's [Main] effect.")
        effect_sec.is_security_effect = True

        def activate_sec():
            print("Activated Giga Destroyer Security Effect")

        effect_sec.set_on_process_callback(activate_sec)

        return [effect_main, effect_sec]
