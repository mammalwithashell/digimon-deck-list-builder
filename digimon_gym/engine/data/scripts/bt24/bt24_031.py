from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_031(CardScript):
    """Auto-transpiled from DCGO BT24_031.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Iliad] trait and 1 card with [TS] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-031 Reveal 3, Add 1 [Iliad] trait, and 1 [TS] trait")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Iliad] trait and 1 card with [TS] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
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

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] You may add your top security card to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-031 May add 1 sec card to hand, if at 0 <Recovery +1>.")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] You may add your top security card to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_031_Inherited")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.recovery(1)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
