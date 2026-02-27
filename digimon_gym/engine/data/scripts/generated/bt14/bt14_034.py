from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_034(CardScript):
    """BT14-034 Sukamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-034 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-034 DP -3000")
        effect1.set_effect_description("[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1.dp_modifier = -3000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
