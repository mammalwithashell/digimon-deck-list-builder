from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_061(CardScript):
    """BT3-061 Chuumon | Lv.3 (Black, Cost 3)

    [All Turns] Your opponent can't gain memory other than by Tamer effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [All Turns] Opponent can't gain memory except by Tamer effects.
        # This is a continuous restriction effect.
        # descriptive-tagged: memory_gain_restriction
        # The engine does not have granular memory-gain-source tracking,
        # so this is registered as a declarative modifier.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT3-061 Opponent can't gain memory except by Tamers")
        effect0.set_effect_description(
            "[All Turns] Your opponent can't gain memory other than by Tamer effects."
        )
        effect0.is_declarative = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
