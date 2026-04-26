from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_06(CardScript):
    """ST12-06 BaoHuckmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: dp_modifier
        # DP modifier
        effect0 = ICardEffect()
        effect0.set_effect_name("ST12-06 DP modifier")
        effect0.set_effect_description("DP modifier")
        effect0.is_inherited_effect = True
        effect0.dp_modifier = 1000

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
