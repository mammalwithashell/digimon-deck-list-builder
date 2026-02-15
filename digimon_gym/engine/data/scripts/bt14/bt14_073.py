from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_073(CardScript):
    """BT14-073 Ogremon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn][Once Per Turn] When one of your effects trashes a card in your hand, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-073 Memory +1")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When one of your effects trashes a card in your hand, gain 1 memory.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Memory+1_BT14_073")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn][Once Per Turn] When one of your effects trashes a card in your hand, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-073 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When one of your effects trashes a card in your hand, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT14_073_inherited")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
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
