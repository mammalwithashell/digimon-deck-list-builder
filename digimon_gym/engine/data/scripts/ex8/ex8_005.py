from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_005(CardScript):
    """EX8-005 Tumblemon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # When effects trash this card from digivolution cards of a [Mineral] or [Rock] trait Digimon, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-005 Memory +1")
        effect0.set_effect_description("When effects trash this card from digivolution cards of a [Mineral] or [Rock] trait Digimon, gain 1 memory.")
        effect0.is_inherited_effect = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
