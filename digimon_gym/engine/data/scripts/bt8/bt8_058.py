from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_058(CardScript):
    """BT8-058 Agumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 4 cards of your deck. Add 1 card with [Greymon] in its name and 1 card with [Dragonkin] in its traits among them to your hand. Place the remaining cards at the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-058 Reveal the top 4 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 4 cards of your deck. Add 1 card with [Greymon] in its name and 1 card with [Dragonkin] in its traits among them to your hand. Place the remaining cards at the bottom of your deck in any order.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter_0(c):
                return True
            def reveal_filter_1(c):
                return True
            game.effect_reveal_and_select_multi(
                player, 4, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
