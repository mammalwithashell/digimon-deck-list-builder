from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_090(CardScript):
    """BT1-090 Gravity Crush"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Gain 2 memory. At end of turn, lose 2 memory.
        # NOTE: The -2 memory at end of turn cannot fire because this Option
        # goes to trash after resolving. Marked PARTIAL.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT1-090 Gain 2 memory")
        effect0.set_effect_description("[Main] Gain 2 memory. At end of turn, lose 2 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(2)
            # End-of-turn -2 memory: descriptive-tagged
            # Options go to trash after OptionSkill resolves, so OnEndTurn
            # effects cannot fire from trash. This is an engine limitation.

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
