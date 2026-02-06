from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_099(CardScript):
    """Auto-transpiled from DCGO BT14_099.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Trash the top 3 cards of your deck. Then, 1 of your Digimon with [Devimon] in its name gains ��Security A. +1��or the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-099 Effect")
        effect0.set_effect_description("[Main] Trash the top 3 cards of your deck. Then, 1 of your Digimon with [Devimon] in its name gains ��Security A. +1��or the turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
