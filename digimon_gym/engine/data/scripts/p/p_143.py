from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_143(CardScript):
    """P-143 Drimogemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] You may move this to the empty space in your breeding area.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-143 Move this Digimon to empty space in breeding area")
        effect0.set_effect_description("[End of Your Turn] [Once Per Turn] You may move this to the empty space in your breeding area.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("EndTurn_P_143")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
