from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_089(CardScript):
    """Auto-transpiled from DCGO BT14_089.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's Digimon with 6000 DP or less. If you have a Digimon with [Greymon] in its name, delete 1 of your opponent's Digimon with the lowest DP instead.
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
            perm = ctx.get('permanent')
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
