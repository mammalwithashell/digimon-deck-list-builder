from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_018(CardScript):
    """BT15-018 Cannondramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] If your opponent has 4 or more memory, delete 1 of their Digimon with the lowest DP.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-018 Delete 1 Digimon with the lowest DP")
        effect0.set_effect_description("[End of Your Turn] [Once Per Turn] If your opponent has 4 or more memory, delete 1 of their Digimon with the lowest DP.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DeleteLowestDP_BT15_018")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent's Turn] [Once Per Turn] If you have 4 or less memory, delete 1 of your opponent's Digimon with the highest play cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-018 Delete 1 Digimon with the highest Cost")
        effect1.set_effect_description("[End of Opponent's Turn] [Once Per Turn] If you have 4 or less memory, delete 1 of your opponent's Digimon with the highest play cost.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DeleteHighestCost_BT15_018")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
