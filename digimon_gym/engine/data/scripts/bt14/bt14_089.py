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
        # [Main] Delete 1 of your opponent's Digimon with 6000 DP or less. If you have a Digimon with [Greymon] in its name, delete 1 of your opponent's Digimon with the lowest DP instead.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-089 Delete")
        effect0.set_effect_description("[Main] Delete 1 of your opponent's Digimon with 6000 DP or less. If you have a Digimon with [Greymon] in its name, delete 1 of your opponent's Digimon with the lowest DP instead.")

        effect = effect0  # alias for condition closure
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

            my_digimon = [p for p in getattr(player, 'battle_area', []) if getattr(p, 'is_digimon', False)]
            has_greymon = any('greymon' in (getattr(p, 'name', '') or '').lower() for p in my_digimon)

            enemy = player.enemy
            enemy_digimon = [p for p in getattr(enemy, 'battle_area', []) if getattr(p, 'is_digimon', False)]
            if not enemy_digimon:
                return

            if has_greymon:
                valid = [p for p in enemy_digimon if getattr(p, 'dp', None) is not None]
                if not valid:
                    return
                min_dp = min(p.dp for p in valid)

                def target_filter(p):
                    return getattr(p, 'is_digimon', False) and getattr(p, 'dp', None) == min_dp
            else:
                def target_filter(p):
                    dp = getattr(p, 'dp', None)
                    return getattr(p, 'is_digimon', False) and dp is not None and dp <= 6000

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
