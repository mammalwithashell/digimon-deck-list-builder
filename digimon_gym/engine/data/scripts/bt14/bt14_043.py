from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_043(CardScript):
    """Auto-transpiled from DCGO BT14_043.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By suspending 1 of your Digimon, suspend 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-043 Suspend your 1 Digimon to suspend opponent's 1 Digimon")
        effect0.set_effect_description("[On Play] By suspending 1 of your Digimon, suspend 1 of your opponent's Digimon.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
