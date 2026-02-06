from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_066(CardScript):
    """Auto-transpiled from DCGO BT14_066.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-066 Trash 1 card from hand to gain Memory +2")
        effect0.set_effect_description("[On Play] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check name: "Numemon" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Gain 2 memory, Trash From Hand"""
            # card.owner.add_memory(2)
            # card.owner.trash_from_hand(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-066 Trash 1 card from hand to gain Memory +2")
        effect1.set_effect_description("[When Digivolving] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check name: "Numemon" in card name
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Gain 2 memory, Trash From Hand"""
            # card.owner.add_memory(2)
            # card.owner.trash_from_hand(count)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 5 or lower Digimon card with [Numemon] or [Monzaemon] in its name from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-066 Play 1 Digimon from hand")
        effect2.set_effect_description("[On Deletion] You may play 1 level 5 or lower Digimon card with [Numemon] or [Monzaemon] in its name from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check name: "Numemon" in card name
            # Check name: "Monzaemon" in card name
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Play Card, Trash From Hand"""
            # play_card_from_hand_or_trash()
            # card.owner.trash_from_hand(count)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
