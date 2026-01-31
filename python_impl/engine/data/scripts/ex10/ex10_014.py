from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_014(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # On Play / When Digivolving
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-014: Security A. -1")
        effect1.timing = EffectTiming.OnEnterFieldAnyone

        def activate1():
            print("Gave 2 opponent Digimon <Security A. -1>.")

        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # Link Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-014: <Link>")
        effect2.is_linked_effect = True
        effects.append(effect2)

        # ESS: Trash Link -> -6000 DP
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-014 ESS: Trash Link -> -6000 DP")
        effect3.is_inherited_effect = True
        effect3.timing = EffectTiming.OnAllyAttack

        def activate3():
            print("Trashed 1 link card. Gave opponent Digimon -6000 DP.")

        effect3.set_on_process_callback(activate3)
        effects.append(effect3)

        return effects
