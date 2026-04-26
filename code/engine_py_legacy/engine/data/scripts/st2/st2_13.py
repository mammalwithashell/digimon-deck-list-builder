from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST2_13(CardScript):
    """ST2-13 Hammer Spark | Option | Blue

    [Main] Gain 1 memory.
    [Security] Gain 2 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Gain 1 memory ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST2-13 Main: Gain 1 memory")
        effect0.set_effect_description("[Main] Gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Gain 1 memory."""
            player = ctx.get('player')
            if player:
                player.add_memory(1)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Gain 2 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("ST2-13 Security: Gain 2 memory")
        effect1.set_effect_description("[Security] Gain 2 memory.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Gain 2 memory."""
            player = ctx.get('player')
            if player:
                player.add_memory(2)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
