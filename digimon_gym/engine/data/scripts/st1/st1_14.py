from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_14(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect_main = ICardEffect()
        effect_main.set_effect_name("ST1-14 Main Effect")
        effect_main.set_effect_description("[Main] All of your Security Digimon get +7000 DP until the end of your opponent's next turn.")

        def activate_main():
            print("Activated Starlight Explosion Main Effect")

        effect_main.set_on_process_callback(activate_main)

        effect_sec = ICardEffect()
        effect_sec.set_effect_name("ST1-14 Security Effect")
        effect_sec.set_effect_description("[Security] Add this card to your hand.")
        effect_sec.is_security_effect = True

        def activate_sec():
            print("Activated Starlight Explosion Security Effect")

        effect_sec.set_on_process_callback(activate_sec)

        return [effect_main, effect_sec]
