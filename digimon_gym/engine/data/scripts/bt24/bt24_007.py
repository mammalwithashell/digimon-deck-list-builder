from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_007(CardScript):
    """Auto-transpiled from DCGO BT24_007.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn] [Once Per Turn] When level 4 or higher Digimon cards with the [Demon] or [Titan] trait are trashed from your hand, you may play 1 of them with the play cost reduced by 2.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-007 Play 1 Level 4 or higher [Demon]/[Titan] Digimon trashed from hand")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When level 4 or higher Digimon cards with the [Demon] or [Titan] trait are trashed from your hand, you may play 1 of them with the play cost reduced by 2.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT24_007_YT")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
