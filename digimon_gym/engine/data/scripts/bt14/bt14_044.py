from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_044(CardScript):
    """BT14-044 Palmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] 1 of your opponent's Digimon gains
        # "[All Turns] When this Digimon becomes suspended, lose 2 memory."
        # until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-044 Opponent's 1 Digimon gains effect")
        effect0.set_effect_description("[Start of Your Main Phase] 1 of your opponent's Digimon gains \"[All Turns] When this Digimon becomes suspended, lose 2 memory.\" until the end of their turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-044 Cost -1")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon would digivolve, if you have a green Tamer, reduce the cost by 1.")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            player = card.owner if card else None
            if player is None:
                return False

            # Require having a green Tamer in play.
            has_green_tamer = False
            tamers = getattr(player, 'tamers', None)
            if tamers is not None:
                for t in tamers:
                    colors = getattr(t, 'card_colors', None)
                    if colors is None and hasattr(t, 'card'):
                        colors = getattr(t.card, 'card_colors', None)
                    if colors and 3 in colors:
                        has_green_tamer = True
                        break
            return has_green_tamer

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
