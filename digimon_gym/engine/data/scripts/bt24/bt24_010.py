from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_010(CardScript):
    """Auto-transpiled from DCGO BT24_010.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-010 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-010 <De-Digivolve 1> 1 of your opponent's digimon")
        effect1.set_effect_description("[On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # De-digivolve opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                removed = target.de_digivolve(1)
                enemy.trash_cards.extend(removed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: raid
        # Raid
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-010 Raid")
        effect2.set_effect_description("Raid")
        effect2._is_raid = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
