from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_001(CardScript):
    """BT20-001 DemiVeemon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: dp_modifier
        # DP modifier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-001 DP modifier")
        effect0.set_effect_description("DP modifier")
        effect0.is_inherited_effect = True
        effect0.dp_modifier = 2000

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and len(permanent.digivolution_cards) >= 4):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
