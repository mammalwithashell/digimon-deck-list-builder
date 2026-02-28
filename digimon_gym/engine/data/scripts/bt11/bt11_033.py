from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_033(CardScript):
    """BT11-033 MirageGaogamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return 1 of your opponent's level 5 or lower Digimon to its owner's hand. If no Digimon was returned by this effect, your opponent adds the top card of their security stack to their hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-033 Return 1 level 5 or lower Digimon to hand or opponent adds the top card of Security")
        effect0.set_effect_description("[When Digivolving] Return 1 of your opponent's level 5 or lower Digimon to its owner's hand. If no Digimon was returned by this effect, your opponent adds the top card of their security stack to their hand.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Bounce, Add To Hand, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddHand
        # [All Turns][Once Per Turn] When an effect adds cards to your opponent's hand, gain 1 memory for every 4 cards in your opponent's hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-033 Gain Memory")
        effect1.set_effect_description("[All Turns][Once Per Turn] When an effect adds cards to your opponent's hand, gain 1 memory for every 4 cards in your opponent's hand.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("GainMemory_BT11_033")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
