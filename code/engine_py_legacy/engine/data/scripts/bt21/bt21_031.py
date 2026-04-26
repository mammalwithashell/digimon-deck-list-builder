from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_031(CardScript):
    """BT21-031 Sangomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: change_digi_cost
        # Change digivolution cost
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-031 Change digivolution cost")
        effect0.set_effect_description("Change digivolution cost")
        # Reduce digivolution cost by 1 for matching
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] (Once Per Turn) Gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndAttack)
        effect1.set_effect_name("BT21-031 Memory +1")
        effect1.set_effect_description("[End of Attack] (Once Per Turn) Gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT21_031")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
