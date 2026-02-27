from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_056(CardScript):
    """BT11-056 Jijimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. You may play 1 Tamer card among them without paying the cost. Place the rest at the top or bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-056 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. You may play 1 Tamer card among them without paying the cost. Place the rest at the top or bottom of your deck in any order.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def reveal_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
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
        # [When Attacking][Once Per Turn] For each green or black Tamer you have in play, reveal 1 card from the top of your deck. You may play any number of green or black Digimon cards whose total play costs add up to 10 or less among them without paying the costs. Place the rest at the bottom of your deck in any order.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-056 Reveal the top cards of deck and play Digimon")
        effect1.set_effect_description("[When Attacking][Once Per Turn] For each green or black Tamer you have in play, reveal 1 card from the top of your deck. You may play any number of green or black Digimon cards whose total play costs add up to 10 or less among them without paying the costs. Place the rest at the bottom of your deck in any order.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Reveal_BT11_056")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 10:
                    return False
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])] or 'Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 10:
                    return False
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])] or 'Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
