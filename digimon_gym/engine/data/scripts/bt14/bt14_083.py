from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_083(CardScript):
    """Auto-transpiled from DCGO BT14_083.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash any 1 digivolution card of 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-083 Trash digivolution cards")
        effect0.set_effect_description("[On Play] Trash any 1 digivolution card of 1 of your opponent's Digimon.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash Digivolution Cards"""
            # target.trash_digivolution_cards(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [Your Turn] When a digivolution card of an opponent's Digimon is trashed, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-083 Memory +1")
        effect1.set_effect_description("[Your Turn] When a digivolution card of an opponent's Digimon is trashed, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Gain 1 memory, Suspend"""
            # card.owner.add_memory(1)
            # target_permanent.suspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-083 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True
        # Security effect: play this card without paying cost
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
