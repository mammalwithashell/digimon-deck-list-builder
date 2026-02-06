from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_084(CardScript):
    """Auto-transpiled from DCGO BT24_084.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # Gain 1 memory
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-084 Memory +1")
        effect0.set_effect_description("Gain 1 memory")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # Suspend, Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-084 Digivolve an [Aegiomon] into an [Aegiochusmon]")
        effect1.set_effect_description("Suspend, Digivolve")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()
            pass  # TODO: digivolve effect needs card selection

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-084 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
