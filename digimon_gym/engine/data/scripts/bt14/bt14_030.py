from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_030(CardScript):
    """BT14-030 MarineAngemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _do_bounce_with_level_limit(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            chosen_level = {'value': None}

            def opp_lv3_filter(p):
                return getattr(p, 'level', 0) == 3

            def on_opp_lv3_selected(target_perm):
                chosen_level['value'] = getattr(target_perm, 'level', 0)
                if player.enemy:
                    player.enemy.bounce_permanent_to_hand(target_perm)

            def my_filter(p):
                return True

            def on_my_selected(target_perm):
                chosen_level['value'] = getattr(target_perm, 'level', 0)
                player.bounce_permanent_to_hand(target_perm)

            paid_cost = {'done': False}

            def choose_my_digimon(_):
                game.effect_select_permanent(
                    player,
                    on_my_selected,
                    filter_fn=my_filter,
                    is_optional=True,
                )
                if chosen_level['value'] is not None:
                    paid_cost['done'] = True

            game.effect_select_opponent_permanent(
                player,
                on_opp_lv3_selected,
                filter_fn=opp_lv3_filter,
                is_optional=True,
                on_cancel=choose_my_digimon,
            )

            if chosen_level['value'] is not None:
                paid_cost['done'] = True

            if not paid_cost['done']:
                return

            def bounce_target_filter(p):
                return getattr(p, 'level', 0) <= chosen_level['value']

            def on_bounce_target(target_perm):
                if player.enemy:
                    player.enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player,
                on_bounce_target,
                filter_fn=bounce_target_filter,
                is_optional=False,
            )

        # [On Play] effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-030 Return Digimons to hand")
        effect0.set_effect_description("[On Play] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_do_bounce_with_level_limit)
        effects.append(effect0)

        # [When Digivolving] effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-030 Return Digimons to hand")
        effect1.set_effect_description("[When Digivolving] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_do_bounce_with_level_limit)
        effects.append(effect1)

        # [Your Turn][Once Per Turn] When another Digimon returns to hand, Recovery +1 (Deck)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-030 Recovery +1 (Deck)")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When another Digimon returns to the hand, <Recovery +1 (Deck)>.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Recovery_BT14_030")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            returned = context.get('returned_permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if returned is None:
                return False
            if my_perm is not None and returned == my_perm:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
