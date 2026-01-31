from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_013(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # When Digivolving: Move
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-013: Move")
        effect1.timing = EffectTiming.OnEnterFieldAnyone

        def activate1():
            print("Moved Lucemon to Battle Area.")

        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # End of Turn: Warp Digivolve
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-013: Warp Digivolve")
        effect2.timing = EffectTiming.OnEndTurn

        def activate2():
            print("Returned 5 Lucemon cards to bottom of deck. Digivolved into Lucemon: Chaos Mode.")

        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        # ESS: Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-013 ESS: Blocker")
        effect3.is_inherited_effect = True
        effects.append(effect3)

        return effects
