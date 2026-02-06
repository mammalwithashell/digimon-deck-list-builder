from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_017(CardScript):
    """Auto-transpiled from DCGO BT24_017.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
        # Raid
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-017 Raid")
        effect0.set_effect_description("Raid")
        effect0._is_raid = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # DP +2000, Delete
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-017 Delete lowest DP Digimon, Return 2 cards from their trash to deck to play 2 Tokens and gain 2k DP per opponent's Digimon.")
        effect1.set_effect_description("DP +2000, Delete")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +2000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if perm:
                perm.change_dp(2000)
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
