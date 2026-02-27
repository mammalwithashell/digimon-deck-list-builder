from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_089(CardScript):
    """BT14-089 Mega Flame"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's Digimon with 6000 DP or less.
        # If you have a Digimon with [Greymon] in its name, delete 1 of your
        # opponent's Digimon with the lowest DP instead.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-089 Delete")
        effect0.set_effect_description("[Main] Delete 1 of your opponent's Digimon with 6000 DP or less. If you have a Digimon with [Greymon] in its name, delete 1 of your opponent's Digimon with the lowest DP instead.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = getattr(player, 'enemy', None)
            if enemy is None:
                return

            enemy_battle = [p for p in getattr(enemy, 'battle_area', []) if getattr(p, 'is_digimon', False)]
            has_greymon = any(
                'greymon' in getattr(p, 'name', '').lower()
                for p in getattr(player, 'battle_area', [])
                if getattr(p, 'is_digimon', False)
            )

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            if has_greymon:
                candidates = [p for p in enemy_battle if getattr(p, 'dp', None) is not None]
                if not candidates:
                    return
                min_dp = min(p.dp for p in candidates)

                def lowest_dp_filter(p):
                    return getattr(p, 'is_digimon', False) and getattr(p, 'dp', None) == min_dp

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=lowest_dp_filter, is_optional=False
                )
                return

            def target_filter(p):
                if not getattr(p, 'is_digimon', False):
                    return False
                dp = getattr(p, 'dp', None)
                return dp is not None and dp <= 6000

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
