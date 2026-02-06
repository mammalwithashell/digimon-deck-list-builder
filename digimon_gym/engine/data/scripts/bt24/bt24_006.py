from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_006(CardScript):
    """Auto-transpiled from DCGO BT24_006.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] [Oncer Per Turn] When this Digimon gets linked, <Draw 1> and trash 1 card in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-006 <Draw 1>, trash 1")
        effect0.set_effect_description("[Your Turn] [Oncer Per Turn] When this Digimon gets linked, <Draw 1> and trash 1 card in your hand.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw_BT24_006")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(1)
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
