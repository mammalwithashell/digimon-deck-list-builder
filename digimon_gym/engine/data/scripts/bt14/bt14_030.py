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

        def _get_level(perm) -> int:
            if perm is None:
                return -1
            for attr in ("level", "digimon_level", "card_level"):
                v = getattr(perm, attr, None)
                if isinstance(v, int):
                    return v
            src = getattr(perm, "card", None)
            if src is not None:
                lv = getattr(src, "level", None)
                if isinstance(lv, int):
                    return lv
            return -1

        def _has_level3_opponent_or_own(player) -> bool:
            enemy = getattr(player, "enemy", None)
            if enemy is not None:
                for p in getattr(enemy, "permanents", []):
                    if _get_level(p) == 3:
                        return True
            for p in getattr(player, "permanents", []):
                if p is not None:
                    return True
            return False

        def _process_bounce_sequence(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            selected_level = {'value': None}

            def own_filter(p):
                return p is not None

            def enemy_lv3_filter(p):
                return _get_level(p) == 3

            def after_cost_paid(returned_perm):
                selected_level['value'] = _get_level(returned_perm)

                def target_filter(p):
                    lv = selected_level['value']
                    return isinstance(lv, int) and lv >= 0 and _get_level(p) <= lv

                def on_bounce(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.bounce_permanent_to_hand(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce, filter_fn=target_filter, is_optional=False
                )

            def choose_enemy_lv3():
                enemy = player.enemy if player else None
                if enemy is None:
                    return

                def on_return_enemy_lv3(target_perm):
                    enemy.bounce_permanent_to_hand(target_perm)
                    after_cost_paid(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_return_enemy_lv3, filter_fn=enemy_lv3_filter, is_optional=False
                )

            def choose_own():
                def on_return_own(target_perm):
                    player.bounce_permanent_to_hand(target_perm)
                    after_cost_paid(target_perm)

                game.effect_select_own_permanent(
                    player, on_return_own, filter_fn=own_filter, is_optional=False
                )

            can_enemy = False
            enemy = getattr(player, 'enemy', None)
            if enemy is not None:
                can_enemy = any(_get_level(p) == 3 for p in getattr(enemy, 'permanents', []))
            can_own = any(p is not None for p in getattr(player, 'permanents', []))

            if can_enemy and not can_own:
                choose_enemy_lv3()
                return
            if can_own and not can_enemy:
                choose_own()
                return
            if not (can_enemy or can_own):
                return

            chooser = getattr(game, 'effect_choose_one', None)
            if callable(chooser):
                chooser(
                    player,
                    [
                        ("Return 1 opponent level 3 Digimon", choose_enemy_lv3),
                        ("Return 1 of your Digimon", choose_own),
                    ],
                    is_optional=True,
                )
            else:
                choose_enemy_lv3()

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-030 Return Digimons to hand")
        effect0.set_effect_description("[On Play] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            if not player:
                return False
            return _has_level3_opponent_or_own(player)

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_process_bounce_sequence)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-030 Return Digimons to hand")
        effect1.set_effect_description("[When Digivolving] By returning 1 of your opponent's level 3 Digimon or 1 of your Digimon to the hand, return 1 of your opponent's Digimon whose level is less than or equal to the returned Digimon's level to the hand.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            if not player:
                return False
            return _has_level3_opponent_or_own(player)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_process_bounce_sequence)
        effects.append(effect1)

        # Timing: EffectTiming.OnPermamemtReturnedToHand
        # [Your Turn][Once Per Turn] When another Digimon returns to the hand, <Recovery +1 (Deck)>.
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
            returned = context.get('returned_permanent') or context.get('target_permanent') or context.get('permanent')
            if returned is None:
                return False
            if returned == card.permanent_of_this_card():
                return False
            return _get_level(returned) >= 0

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
