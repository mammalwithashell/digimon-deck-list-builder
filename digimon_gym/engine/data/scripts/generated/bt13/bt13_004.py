from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_004(CardScript):
    """BT13-004 Budmon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: dp_modifier
        # DP modifier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-004 DP modifier")
        effect0.set_effect_description("DP modifier")
        effect0.is_inherited_effect = True
        effect0.dp_modifier = 1000

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check if opponent has any suspended Digimon in play
            opponent = card.owner.get_opponent() if card.owner else None
            if opponent is None:
                return False
            battle_area = getattr(opponent, 'battle_area', None)
            if battle_area is None:
                return False
            # Return True if any Digimon in opponent's battle area is suspended
            for digimon in battle_area.digimon_cards:
                if getattr(digimon, 'is_suspended', False):
                    return True
            return False
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
