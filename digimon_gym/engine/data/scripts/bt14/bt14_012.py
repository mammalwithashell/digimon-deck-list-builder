from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_012(CardScript):
    """Auto-transpiled from DCGO BT14_012.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] This Digimon gets +2000 DP for the turn. Then, if you have a Tamer with [Tai Kamiya] in its name, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-012 DP +2000 and gain Memory +1")
        effect0.set_effect_description("[When Attacking] This Digimon gets +2000 DP for the turn. Then, if you have a Tamer with [Tai Kamiya] in its name, gain 1 memory.")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)
            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-012 DP modifier")
        effect1.set_effect_description("DP modifier")
        effect1.dp_modifier = 2000  # Inherited: +2000 DP while name contains [Greymon] or [Omnimon]
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
