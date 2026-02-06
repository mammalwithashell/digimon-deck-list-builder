from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_094(CardScript):
    """Auto-transpiled from DCGO BT14_094.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Activate 1 of the effects below: - 1 of your opponent's Digimon gets -6000 DP for the turn. - By deleting 1 of your [Angemon], place 1 of your opponent's Digimon at the bottom of their security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-094 DP -6000")
        effect0.set_effect_description("[Main] Activate 1 of the effects below: - 1 of your opponent's Digimon gets -6000 DP for the turn. - By deleting 1 of your [Angemon], place 1 of your opponent's Digimon at the bottom of their security stack.")
        effect0.dp_modifier = -6000

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: DP -6000"""
            # target.change_dp(-6000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
