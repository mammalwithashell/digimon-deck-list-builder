from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_042(CardScript):
    """Auto-transpiled from DCGO BT24_042.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn] [Once Per Turn] When your hand is trashed from, this [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait Digimon card in the trash with the digivolution cost reduced by 1.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-042 When your hand is trashed from, digivolve")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When your hand is trashed from, this [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait Digimon card in the trash with the digivolution cost reduced by 1.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT24_042_YT_ESS")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
