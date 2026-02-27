from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_031(CardScript):
    """BT14-031 Elecmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-031 DP -2000")
        effect0.set_effect_description("[When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP-2000_BT14_031")
        effect0.is_on_attack = True
        effect0.dp_modifier = -2000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
