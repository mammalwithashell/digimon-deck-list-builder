from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_044(CardScript):
    """Auto-transpiled from DCGO BT14_044.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-044 Opponent's 1 Digimon gains effect")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [All Turns] When this Digimon becomes suspended, lose 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-044 Memory -2")
        effect1.set_effect_description("[All Turns] When this Digimon becomes suspended, lose 2 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Cost -1
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-044 Cost -1")
        effect2.set_effect_description("Cost -1")
        effect2.is_inherited_effect = True
        effect2.cost_reduction = 1

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check color: CardColor.Green
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Cost -1"""
            # reduce_cost(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
