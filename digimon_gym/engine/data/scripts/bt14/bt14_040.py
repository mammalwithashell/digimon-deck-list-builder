from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_040(CardScript):
    """Auto-transpiled from DCGO BT14_040.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Tamer card from your hand on top of your security stack. 
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-040 Place 1 Tamer card from hand at the top of security")
        effect0.set_effect_description("[On Play] You may place 1 Tamer card from your hand on top of your security stack. ")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash From Hand, Add To Security"""
            # card.owner.trash_from_hand(count)
            # card.owner.add_to_security()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Tamer card from your hand on top of your security stack. 
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-040 Place 1 Tamer card from hand at the top of security")
        effect1.set_effect_description("[When Digivolving] You may place 1 Tamer card from your hand on top of your security stack. ")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Trash From Hand, Add To Security"""
            # card.owner.trash_from_hand(count)
            # card.owner.add_to_security()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns][Once Per Turn] When you play a Tamer, you may play 1 level 3 Digimon card from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-040 Play 1 Digimon from hand")
        effect2.set_effect_description("[All Turns][Once Per Turn] When you play a Tamer, you may play 1 level 3 Digimon card from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Play1Digimon_Bt14_040")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Play Card, Trash From Hand"""
            # play_card_from_hand_or_trash()
            # card.owner.trash_from_hand(count)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
