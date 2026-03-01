from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_068(CardScript):
    """EX1-068 Ice Wall!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] All of your opponent's Digimon gain "[When Attacking] lose 2
        # memory" until the end of their next turn.
        # NOTE: Granting WhenAttacking effects to opponent Digimon is not
        # supported by the engine. Marked PARTIAL.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX1-068 Grant opponent WhenAttacking memory loss")
        effect0.set_effect_description("[Main] All of your opponent's Digimon gain \"[When Attacking] lose 2 memory\" until the end of their next turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Grant WhenAttacking memory loss to opponent Digimon"""
            # descriptive-tagged: granting WhenAttacking effects to opponent
            # Digimon is not supported by the engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect: Gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX1-068 Gain 2 memory")
        effect1.set_effect_description("[Security] Gain 2 memory.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
