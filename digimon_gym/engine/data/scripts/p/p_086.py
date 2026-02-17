from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_086(CardScript):
    """P-086 Syakomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have a blue Tamer in play, 1 of your Digimon can't be attacked until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-086 Your 1 Digimon gets unable to be attacked")
        effect0.set_effect_description("[On Play] If you have a blue Tamer in play, 1 of your Digimon can't be attacked until the end of your opponent's turn.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
