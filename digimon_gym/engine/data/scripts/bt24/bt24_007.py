from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_007(CardScript):
    """BT24-007 Tsunomon | Lv.2"""

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

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level < 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
