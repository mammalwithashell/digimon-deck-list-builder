from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_020(CardScript):
    """Auto-transpiled from DCGO BT24_020.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the [Sea Beast] or [Shaman] trait or [Aqua] or [Sea Animal] in any of its traits and 1 card with the [TS] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-020 Reveal 3 from deck. Add 1 [Sea Beast], [Shaman], [Aqua] or [Sea Animal] and 1 [TS].")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the [Sea Beast] or [Shaman] trait or [Aqua] or [Sea Animal] in any of its traits and 1 card with the [TS] trait among them to the hand. Return the rest to the bottom of the deck.")
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

        # Timing: EffectTiming.OnUnTappedAnyone
        # Draw 1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-020 If you have 7 or fewer cards in hand, <Draw 1>.")
        effect1.set_effect_description("Draw 1")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_020_YT_Draw1")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
