from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_044(CardScript):
    """BT14-044 Palmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects: List[ICardEffect] = []

        # [Start of Your Main Phase] 1 of your opponent's Digimon gains
        # "[All Turns] When this Digimon becomes suspended, lose 2 memory."
        # until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-044 Grant suspend penalty")
        effect0.set_effect_description("[Start of Your Main Phase] 1 of your opponent's Digimon gains \"[All Turns] When this Digimon becomes suspended, lose 2 memory.\" until the end of their turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Engine-tagged behavior hook for selecting 1 opponent Digimon
            # and granting the standard temporary suspend-penalty effect.
            pass  # descriptive-tagged: grant_opponent_suspend_memory_loss_until_end_of_their_turn

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-044 Inherited digivolution cost -1")
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

            # Require at least 1 green Tamer in play.
            tamers = getattr(player, 'tamers', None)
            if not tamers:
                return False

            for t in tamers:
                colors = getattr(t, 'card_colors', None)
                if colors and 3 in colors:
                    return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Cost reduction handled via cost_reduction property.
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
