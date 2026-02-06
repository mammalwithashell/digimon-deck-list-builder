from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_091(CardScript):
    """Auto-transpiled from DCGO BT24_091.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-091 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-091 Bounce all opponent's lowest level. Unsuspend a Digimon. Then, you may link this card.")
        effect1.set_effect_description("Effect")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level Digimon to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-091 Bounce 1 opponent's lowest level Digimon.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level Digimon to the hand.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WA_BT24-091")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Bounce: return opponent's digimon to hand
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                player.bounce_permanent_to_hand(target)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
