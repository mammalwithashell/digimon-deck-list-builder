from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_008(CardScript):
    """Auto-transpiled from DCGO BT14_008.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with 3000 DP or less.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-008 Delete 1 Digimon with 3000 DP or less")
        effect0.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Delete_BT14_008")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Delete"""
            # target_permanent.delete()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
