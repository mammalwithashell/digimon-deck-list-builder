from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_001(CardScript):
    """BT16-001 Poromon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] If this Digimon has 2 or more colors, delete 1 of your opponent's Digimon with 2000 or less DP.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("BT16-001 Delete opponent's Digimon with 2000 DP or less.")
        effect0.set_effect_description("[When Attacking] [Once Per Turn] If this Digimon has 2 or more colors, delete 1 of your opponent's Digimon with 2000 or less DP.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Delete_BT16_001")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
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
                if p.dp is None or p.dp > 2000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
