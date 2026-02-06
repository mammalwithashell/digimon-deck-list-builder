from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_068(CardScript):
    """Auto-transpiled from DCGO BT24_068.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Evil] or [Fallen Angel] trait and 1 card with the [Seven Great Demon Lords] trait among them to the hand. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-068 Reveal 3 from deck. Add 2. Return the rest to bot deck. Trash 1 from hand.")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Evil] or [Fallen Angel] trait and 1 card with the [Seven Great Demon Lords] trait among them to the hand. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Trash the top card of both players' decks.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-068 Trash top card from both players deck")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] Trash the top card of both players' decks.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_068_TrashTopDeck")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
