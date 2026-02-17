from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_017(CardScript):
    """P-017 DemiDevimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 2 cards of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-017 Trash 2 cards from deck top")
        effect0.set_effect_description("[On Play] Trash the top 2 cards of your deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
