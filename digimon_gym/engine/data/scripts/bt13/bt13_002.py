from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_002(CardScript):
    """BT13-002 Chapmon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: dp_modifier
        # DP modifier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-002 DP modifier")
        effect0.set_effect_description("DP modifier")
        effect0.is_inherited_effect = True
        effect0.dp_modifier = 1000

        def condition0(context: Dict[str, Any]) -> bool:
            if not card:
                return False

            permanent = card.permanent_of_this_card()
            if permanent is None:
                return False

            game = context.get("game")
            if game is None:
                return False

            # [Opponent's Turn]
            if getattr(game, "turn_player", None) == permanent.owner:
                return False

            # "while you have another Digimon in play"
            owner = permanent.owner
            digimon_count = 0
            for p in getattr(owner, "battle_area", []) or []:
                if getattr(p, "is_digimon", lambda: False)() and p is not permanent:
                    digimon_count += 1
                    if digimon_count >= 1:
                        return True
            return False

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
