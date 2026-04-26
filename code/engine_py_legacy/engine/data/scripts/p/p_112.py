from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_112(CardScript):
    """P-112 Morphomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 [Eosmon] and 1 [Menoa Bellucci] among them to your hand. Place the rest at the bottom of your deck. Then, by placing this card under 1 of your [Eosmon], you may play 1 [Menoa Bellucci] from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-112 Reveal the top 3 cards of deck and play 1 [Menoa Bellucci]")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 [Eosmon] and 1 [Menoa Bellucci] among them to your hand. Place the rest at the bottom of your deck. Then, by placing this card under 1 of your [Eosmon], you may play 1 [Menoa Bellucci] from your hand without paying the cost.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
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
            def reveal_filter_2(c):
                return True
            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand'), (reveal_filter_2, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When another [Eosmon] is played, this Digimon may digivolve into an  [Eosmon] from your hand, reducing the digivolution cost by 3.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-112 This Digimon digivolves into [Eosmon]")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When another [Eosmon] is played, this Digimon may digivolve into an  [Eosmon] from your hand, reducing the digivolution cost by 3.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DigivolveIntoEosmon_P_112")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
