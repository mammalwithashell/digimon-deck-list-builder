from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_061(CardScript):
    """Auto-transpiled from DCGO BT14_061.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning 1 Digimon card from your opponent's trash to the top of the deck, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-061 Return 1 card from opponent's trash to deck top to gain Memory +1")
        effect0.set_effect_description("[On Play] By returning 1 Digimon card from your opponent's trash to the top of the deck, gain 1 memory.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of the deck, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-061 Return 1 card from opponent's trash to deck top to gain Memory +1")
        effect1.set_effect_description("[When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of the deck, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
