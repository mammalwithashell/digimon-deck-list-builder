from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_036(CardScript):
    """Auto-transpiled from DCGO BT14_036.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-036 DP -3000")
        effect0.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect0.is_on_play = True
        effect0.dp_modifier = -3000

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: DP -3000"""
            # target.change_dp(-3000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-036 DP -2000")
        effect1.set_effect_description("[When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-2000_BT14_036")
        effect1.is_on_attack = True
        effect1.dp_modifier = -2000

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: DP -2000"""
            # target.change_dp(-2000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
