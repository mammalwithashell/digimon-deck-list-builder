from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_111(CardScript):
    """BT13-111 Gallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.WhenWouldDigivolve
        # Effect: Cost reduction based on trash count
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-111 Cost Reduction")
        effect0.set_effect_description("Reduce play cost by 2 for every 5 cards in both trashes.")
        # WhenWouldDigivolve usually doesn't have a timing enum for 'Play' cost reduction in this engine yet,
        # but if we assume it works for WhenWouldDigivolve (as per C# wrapper logic), we'll set it.
        # However, for Play Cost reduction, we might need a custom check if engine supports it.
        # Assuming engine checks 'WhenWouldDigivolve' for digivolve cost, we keep it.
        # But this is Play Cost reduction? The C# code had "Play Cost -".
        # If it's Play Cost, effect0.timing should be None (static).
        # We will set the cost_reduction property anyway in condition0.

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            if not player:
                return False
            # Count trash: my trash + enemy trash
            my_trash = len(player.trash_cards)
            enemy_trash = len(player.enemy.trash_cards) if player.enemy else 0
            total_trash = my_trash + enemy_trash

            # Reduce cost by 2 for every 5 cards
            reduction = 2 * (total_trash // 5)
            effect0.cost_reduction = reduction
            return True

        effect0.set_can_use_condition(condition0)
        # No process callback needed for cost reduction if property is used
        effects.append(effect0)

        # Helper for the delete logic (used in OnPlay, WhenDigivolving, WhenAttacking)
        def delete_logic(ctx: Dict[str, Any]):
            """Action: Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def first_target_filter(p):
                if p.dp is not None and p.dp <= 6000:
                    return True
                return False

            def second_target_filter(p):
                if p.dp is not None and p.dp >= 13000:
                    return True
                return False

            enemy = player.enemy
            if not enemy:
                return

            has_first_targets = any(first_target_filter(p) for p in enemy.battle_area)

            if has_first_targets:
                 # Request selection for first part
                 def on_first_selection_done(target_perm):
                     deleted = enemy.delete_permanent(target_perm)
                     if not deleted:
                         # If selection happened but deletion failed (e.g. prevented), proceed to second part
                         game.effect_select_opponent_permanent(
                            player,
                            lambda p: enemy.delete_permanent(p),
                            filter_fn=second_target_filter,
                            is_optional=False
                         )

                 game.effect_select_opponent_permanent(
                    player, on_first_selection_done, filter_fn=first_target_filter, is_optional=False)
            else:
                # No targets for first part -> Nothing deleted -> Proceed to second part
                game.effect_select_opponent_permanent(
                    player,
                    lambda p: enemy.delete_permanent(p),
                    filter_fn=second_target_filter,
                    is_optional=False
                )

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-111 [On Play] Delete logic")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(delete_logic)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] ...
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-111 [When Digivolving] Delete logic")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(delete_logic)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] ...
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-111 [When Attacking] Delete logic")
        effect3.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(delete_logic)
        effects.append(effect3)

        return effects
