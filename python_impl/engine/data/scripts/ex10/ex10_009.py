from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_009(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # When Digivolving / On Deletion
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-009: Delete Lowest/Trash Deck")
        effect1.timing = EffectTiming.OnEnterFieldAnyone # And OnDestroyedAnyone

        def activate1():
            print("Deleted opponent's lowest DP Digimon OR Trashed top 5 cards of opponent's deck.")

        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # When Attacking
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-009: Play from Trash")
        effect2.timing = EffectTiming.OnAllyAttack

        def activate2():
            print("Played 1 Red/Purple Lv5 or lower Digimon from Trash to Breeding Area.")

        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        # End of Turn: Attack
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-009: Attack")
        effect3.timing = EffectTiming.OnEndTurn

        def activate3():
            print("This Digimon attacks.")

        effect3.set_on_process_callback(activate3)
        effects.append(effect3)

        return effects
