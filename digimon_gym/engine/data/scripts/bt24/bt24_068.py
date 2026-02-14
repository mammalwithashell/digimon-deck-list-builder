from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_068(CardScript):
    """BT24-068 DemiDevimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Evil] or [Fallen Angel] trait and 1 card with the [Seven Great Demon Lords] trait among them to the hand. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-068 Reveal 3 from deck. Add 2. Return the rest to bot deck. Trash 1 from hand.")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Evil] or [Fallen Angel] trait and 1 card with the [Seven Great Demon Lords] trait among them to the hand. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
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
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

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

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
