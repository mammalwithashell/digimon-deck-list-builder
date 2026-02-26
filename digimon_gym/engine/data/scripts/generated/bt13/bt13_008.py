from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_008(CardScript):
    """BT13-008 Agumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main][Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon and can't digivolve.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-008 Your 1 [Marcus Damon] becomes Digimon")
        effect1.set_effect_description("[Main][Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon and can't digivolve.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BecomeDigimon_BT13_008")

        def condition1(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            game = context.get('game')
            if not (player and game):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                src = getattr(p, 'card_source', None)
                name = getattr(src, 'card_name', '') or getattr(src, 'card_name_eng', '')
                owner = getattr(p, 'owner', None)
                is_tamer = bool(getattr(p, 'is_tamer', False))
                return owner == player and is_tamer and name == 'Marcus Damon'

            def on_select(target_perm):
                # Explicitly mark the selected Marcus Damon as treated as a 3000 DP Digimon
                # and unable to digivolve for the turn.
                setattr(target_perm, 'bt13_008_temp_dp', 3000)
                setattr(target_perm, 'bt13_008_cannot_digivolve', True)
                setattr(target_perm, 'bt13_008_until_end_of_turn', True)

            game.effect_select_permanent(
                player,
                on_select,
                filter_fn=target_filter,
                count=1,
                is_optional=False,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-008 Delete 1 Digimon with 3000 DP or less")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT13_008")

        def condition2(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            tapped = context.get('permanent') or context.get('target_permanent')
            if not player or not getattr(player, 'is_my_turn', False):
                return False
            if tapped is None:
                return False
            if getattr(tapped, 'owner', None) != player:
                return False
            if not getattr(tapped, 'is_tamer', False):
                return False

            colors = getattr(getattr(tapped, 'card_source', None), 'card_colors', None)
            if colors is None:
                colors = getattr(tapped, 'card_colors', None)
            if not isinstance(colors, list):
                return False
            # red=0, yellow=2
            return (0 in colors) or (2 in colors)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: may delete opponent Digimon with 3000 DP or less."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not getattr(p, 'is_digimon', False):
                    return False
                dp = getattr(p, 'dp', None)
                return dp is not None and dp <= 3000

            def on_delete(target_perm):
                enemy = getattr(player, 'enemy', None)
                if enemy is None:
                    return
                if not getattr(target_perm, 'is_digimon', False):
                    return
                dp = getattr(target_perm, 'dp', None)
                if dp is None or dp > 3000:
                    return
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player,
                on_delete,
                filter_fn=target_filter,
                is_optional=True,
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
