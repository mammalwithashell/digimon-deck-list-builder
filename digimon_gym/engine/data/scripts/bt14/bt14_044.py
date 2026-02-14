from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_044(CardScript):
    """BT14-044 Palmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-044 Opponent's 1 Digimon gains effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [All Turns] When this Digimon becomes suspended, lose 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-044 Memory -2")
        effect1.set_effect_description("[All Turns] When this Digimon becomes suspended, lose 2 memory.")

        effect = effect1  # alias for condition closure
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

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction handled via cost_reduction property

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
