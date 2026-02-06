from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_027(CardScript):
    """Auto-transpiled from DCGO BT24_027.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-027 By tucking, 1 digimon can't be deleted by battle")
        effect0.set_effect_description("Effect")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-027 By tucking, 1 digimon can't be deleted by battle")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, <Draw 1>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-027 Draw 1 card")
        effect2.set_effect_description("[When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, <Draw 1>.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Draw_BT24_027")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
