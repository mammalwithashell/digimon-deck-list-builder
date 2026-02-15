from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_095(CardScript):
    """BT14-095 Poison Ivy"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-095 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [All Turns] When this Digimon becomes suspended, lose 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-095 Memory -2")
        effect1.set_effect_description("[All Turns] When this Digimon becomes suspended, lose 2 memory.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add Temp Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant temporary effect to target permanent
            pass  # descriptive-tagged: add_temp_effect

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
