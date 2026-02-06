from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_096(CardScript):
    """Auto-transpiled from DCGO BT14_096.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Suspend 1 of your opponent's Digimon. Then, if you have a Tamer with [Mimi Tachikawa] in its name, 1 of your opponent's Digimon doesn't unsuspend until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-096 Effect")
        effect0.set_effect_description("[Main] Suspend 1 of your opponent's Digimon. Then, if you have a Tamer with [Mimi Tachikawa] in its name, 1 of your opponent's Digimon doesn't unsuspend until the end of their turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
