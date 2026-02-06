from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_032(CardScript):
    """Auto-transpiled from DCGO BT14_032.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Add the top card of your security stack to the hand. Then, you may place 1 card with [Sukamon] in its name from your hand on top of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-032 Add the security top card to hand and place 1 card from hand to security ")
        effect0.set_effect_description("[On Play] Add the top card of your security stack to the hand. Then, you may place 1 card with [Sukamon] in its name from your hand on top of your security stack.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check name: "Sukamon" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash From Hand, Add To Hand, Add To Security"""
            # card.owner.trash_from_hand(count)
            # add_card_to_hand()
            # card.owner.add_to_security()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-032 DP -3000")
        effect1.set_effect_description("[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1.dp_modifier = -3000

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: DP -3000"""
            # target.change_dp(-3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
