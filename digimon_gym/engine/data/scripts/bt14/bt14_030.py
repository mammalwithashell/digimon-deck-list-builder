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

        def _get_level_from_permanent(p) -> int:
            if p is None:
                return -1
            for attr in ("level", "digimon_level", "card_level"):
                v = getattr(p, attr, None)
                if isinstance(v, int):
                    return v
            src = getattr(p, "card", None)
            if src is not None:
                v = getattr(src, "level", None)
                if isinstance(v, int):
                    return v
            return -1

        def _run_bounce_sequence(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return

            chosen_level = {'value': None}

            def enemy_lv3_filter(p):
                return _get_level_from_permanent(p) == 3

            def my_digimon_filter(p):
                owner = getattr(p, 'owner', None)
                return owner is player and _get_level_from_permanent(p) >= 0

            def after_cost_bounce(target_perm):
                if target_perm is None:
                    return
                chosen_level['value'] = _get_level_from_permanent(target_perm)
                owner = getattr(target_perm, 'owner', None)
                if owner:
                    owner.bounce_permanent_to_hand(target_perm)

            # Option A: opponent level 3
            game.effect_select_opponent_permanent(
                player,
                after_cost_bounce,
                filter_fn=enemy_lv3_filter,
                is_optional=True,
            )

            # Option B: one of your Digimon (if A not taken)
            if chosen_level['value'] is None:
                game.effect_select_permanent(
                    player,
                    after_cost_bounce,
                    filter_fn=my_digimon_filter,
                    is_optional=True,
                )

            if chosen_level['value'] is None:
                return

            max_level = chosen_level['value']

            def enemy_target_filter(p):
                return _get_level_from_permanent(p) <= max_level

            def on_bounce_enemy(target_perm):
                enemy = player.enemy
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player,
                on_bounce_enemy,
                filter_fn=enemy_target_filter,
                is_optional=False,
            )

        # [On Play]
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
        effect0.set_on_process_callback(_run_bounce_sequence)
        effects.append(effect0)

        # [When Digivolving]
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
        effect1.set_on_process_callback(_run_bounce_sequence)
        effects.append(effect1)

        # [Your Turn][Once Per Turn] when another Digimon is returned to hand, Recovery +1
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-030 Recovery +1 (Deck)")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When another Digimon returns to the hand, <Recovery +1 (Deck)>." )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Recovery_BT14_030")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            moved = context.get('permanent') or context.get('target_permanent')
            if moved is None:
                return False
            self_perm = card.permanent_of_this_card() if card else None
            if self_perm is not None and moved is self_perm:
                return False
            return _get_level_from_permanent(moved) >= 0

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
