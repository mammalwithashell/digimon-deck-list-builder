from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_047(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-047 Blocker")
        effect1.set_effect_description("<Blocker>")
        effect1.is_inherited_effect = False
        effect1.is_keyword_effect = True
        effect1.keyword = "Blocker" # Setting dynamic attribute
        effects.append(effect1)

        # Effect 2: Inherited Reboot
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-047 Inherited Reboot")
        effect2.set_effect_description("<Reboot>")
        effect2.is_inherited_effect = True
        effect2.is_keyword_effect = True
        effect2.keyword = "Reboot" # Setting dynamic attribute
        effects.append(effect2)

        return effects
