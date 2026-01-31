from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_012(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Hand Effect: Reduced Cost Play
        # This requires engine support for Hand Effects which is limited.
        # Stubbing the intent.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-012: Reduced Cost Play")
        effect1.timing = EffectTiming.OnDeclaration
        effects.append(effect1)

        # Security Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-012 Sec: Play Dark Master")
        effect2.timing = EffectTiming.SecuritySkill

        def activate2():
            print("Played a Level 5 or lower Dark Master from Hand/Trash.")

        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        return effects
