from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_033(CardScript):
    """BT5-033 Cutemon | Lv.3 Yellow Digimon

    [Opponent's Turn] Your opponent can't reduce digivolution costs.

    NOTE: The engine does not currently support preventing opponent cost
    reductions. This is implemented as a descriptive-tagged effect.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Opponent's Turn] Opponent can't reduce evo costs ---
        # descriptive-tagged: engine lacks cost-lock mechanism
        effect0 = ICardEffect()
        effect0.set_effect_name("BT5-033 Opponent can't reduce evo costs")
        effect0.set_effect_description(
            "[Opponent's Turn] Your opponent can't reduce digivolution costs."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Active on opponent's turn only
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        # descriptive-tagged: preventing opponent cost reductions not supported
        effects.append(effect0)

        return effects
