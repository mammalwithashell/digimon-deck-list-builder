from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_041(CardScript):
    """Auto-transpiled from DCGO BT14_041.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-041 Recovery +1 (Deck)")
        effect0.set_effect_description("[When Digivolving] Trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Recovery +1"""
            # card.owner.recover(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddSecurity
        # [All Turns][Once Per Turn] When a card is added to your security stack, 1 of your opponent's Digimon gets -7000 DP and this Digimon gains ��Security A. +1��or the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-041 Opponent's 1 Digimon gains DP -7000 and this Digimon gains Security Attack +1")
        effect1.set_effect_description("[All Turns][Once Per Turn] When a card is added to your security stack, 1 of your opponent's Digimon gets -7000 DP and this Digimon gains ��Security A. +1��or the turn.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-7000_BT14_041")
        effect1.dp_modifier = -7000

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: DP -7000"""
            # target.change_dp(-7000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
