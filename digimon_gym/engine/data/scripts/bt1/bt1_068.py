from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_068(CardScript):
    """BT1-068 Kokuwamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited Effect: [Your Turn] While this Digimon is level 6 or higher,
        # it gains <Security A. +1>
        effect0 = ICardEffect()
        effect0.set_effect_name("BT1-068 Inherited: SA+1 when Lv.6+")
        effect0.set_effect_description(
            "[Your Turn] While this Digimon is level 6 or higher, "
            "it gains Security Attack +1."
        )
        effect0.is_inherited_effect = True
        effect0._security_attack_modifier = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            return (perm.level or 0) >= 6

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
