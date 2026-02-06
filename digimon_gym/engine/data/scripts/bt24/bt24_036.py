from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_036(CardScript):
    """Auto-transpiled from DCGO BT24_036.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-036 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-036 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-036 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-036 DP -5000")
        effect3.set_effect_description("[On Deletion] 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect3.is_on_deletion = True
        effect3.dp_modifier = -5000

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -5000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                target.change_dp(-5000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
