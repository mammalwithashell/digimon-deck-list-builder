from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_032(CardScript):
    """Auto-transpiled from DCGO BT24_032.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Appmon] trait and 1 card with the [System] or [Transmutation] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-032 Reveal 3, add 2, bot deck rest")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Appmon] trait and 1 card with the [System] or [Transmutation] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

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
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenLinked
        # [When Linking] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-032 1 of your opponent's Digimon gets -2k dp for the turn")
        effect1.set_effect_description("[When Linking] 1 of your opponent's Digimon gets -2000 DP for the turn.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
