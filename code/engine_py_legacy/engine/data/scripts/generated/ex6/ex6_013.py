from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_013(CardScript):
    """EX6-013 Xiquemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: jamming
        # Jamming
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-013 Jamming")
        effect0.set_effect_description("Jamming")
        effect0.is_inherited_effect = True
        effect0._is_jamming = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Draw 1>. If played from digivolution cards, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-013 Draw 1 and gain 1 memory")
        effect1.set_effect_description("[On Play] <Draw 1>. If played from digivolution cards, gain 1 memory.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
