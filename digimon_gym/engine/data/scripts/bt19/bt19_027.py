from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_027(CardScript):
    """BT19-027 Ryugumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: decode
        # Decode
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-027 Decode")
        effect0.set_effect_description("Decode")
        effect0._is_decode = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 4 or lower blue Digimon card from this Digimon's digivolution cards without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-027 Play 1 Digimon from this Digimon's digivolution cards")
        effect1.set_effect_description("[When Digivolving] You may play 1 level 4 or lower blue Digimon card from this Digimon's digivolution cards without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] (Once Per Turn) By returning 1 of your Digimon to the bottom of the deck, return 1 of your opponent's Digimon with level equal to or lower to your Digimon to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-027 Bottom deck 1 of your Digimon to bottom deck and opponent's Digimon with the same level")
        effect2.set_effect_description("[End of Your Turn] (Once Per Turn) By returning 1 of your Digimon to the bottom of the deck, return 1 of your opponent's Digimon with level equal to or lower to your Digimon to the bottom of the deck.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BottomDeck_BT19_027")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
