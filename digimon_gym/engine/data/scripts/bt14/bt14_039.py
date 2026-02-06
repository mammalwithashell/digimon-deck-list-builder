from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_039(CardScript):
    """Auto-transpiled from DCGO BT14_039.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: armor_purge
        # Armor Purge
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-039 Armor Purge")
        effect0.set_effect_description("Armor Purge")
        effect0._is_armor_purge = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 Digimon card with [Numemon] in its name from your trash as this Digimon's bottom digivolution card, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-039 Place 1 card from trash to digivolution cards to gain Memory +2")
        effect1.set_effect_description("[On Play] By placing 1 Digimon card with [Numemon] in its name from your trash as this Digimon's bottom digivolution card, gain 2 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-039 Security Attack +1")
        effect2.set_effect_description("Security Attack +1")
        effect2._security_attack_modifier = 1
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
