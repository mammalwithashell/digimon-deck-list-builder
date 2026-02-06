from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_13(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect_main = ICardEffect()
        effect_main.set_effect_name("ST1-13 Main Effect")
        effect_main.set_effect_description("[Main] 1 of your Digimon gets +3000 DP for the turn.")

        def activate_main():
            # Logic to select 1 Digimon and add 3000 DP
            print("Activated Shadow Wing Main Effect")

        effect_main.set_on_process_callback(activate_main)

        effect_sec = ICardEffect()
        effect_sec.set_effect_name("ST1-13 Security Effect")
        effect_sec.set_effect_description("[Security] Add this card to your hand.")
        effect_sec.is_security_effect = True

        def activate_sec():
            # Logic to add to hand
            print("Activated Shadow Wing Security Effect")

        effect_sec.set_on_process_callback(activate_sec)

        return [effect_main, effect_sec]
