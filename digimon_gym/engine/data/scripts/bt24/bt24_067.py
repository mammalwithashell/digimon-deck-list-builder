from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_067(CardScript):
    """Auto-transpiled from DCGO BT24_067.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.WhenLinked
        # Play Card, Trash From Hand
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-067 Play 1 [Rei Katsura]")
        effect0.set_effect_description("Play Card, Trash From Hand")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("WhenLinked_BT24_067")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
