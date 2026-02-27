from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_002(CardScript):
    """BT14-002 Bukamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: [Your Turn] While your opponent has no Digimon with as many
        # or more digivolution cards than this Digimon, this Digimon gains Jamming.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-002 Inherited Jamming")
        effect0.set_effect_description("[Your Turn] While your opponent has no Digimon with as many or more digivolution cards than this Digimon, this Digimon gains <Jamming>.")
        effect0.is_inherited_effect = True
        effect0._is_jamming = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            holder = card.permanent_of_this_card() if card else None
            if holder is None:
                return False

            holder_evo_count = len(getattr(holder, 'digivolution_cards', []) or [])
            opponent = card.owner.opponent if card and card.owner else None
            opponent_board = getattr(opponent, 'battle_area', []) if opponent else []

            for opp_digimon in opponent_board:
                if opp_digimon is None:
                    continue
                opp_evo_count = len(getattr(opp_digimon, 'digivolution_cards', []) or [])
                if opp_evo_count >= holder_evo_count:
                    return False

            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
