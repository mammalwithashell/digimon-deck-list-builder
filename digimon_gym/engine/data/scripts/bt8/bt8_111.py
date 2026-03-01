from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_111(CardScript):
    """BT8-111 Creepymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 2 cards of your deck for each of your opponent's Digimon in play. If you trash 4 or more cards with this effect, you may play 1 level 5 or lower purple Digimon card from your trash without paying its memory cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT8-111 Trash cards from deck top and gain play a Digimon from trash")
        effect0.set_effect_description("[When Digivolving] Trash the top 2 cards of your deck for each of your opponent's Digimon in play. If you trash 4 or more cards with this effect, you may play 1 level 5 or lower purple Digimon card from your trash without paying its memory cost.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck, and this Digimon gets +3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name("BT8-111 Trash opponent's deck and get DP +3000")
        effect1.set_effect_description("[When Attacking][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck, and this Digimon gets +3000 DP for the turn.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashDeck_BT8_111")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 3 cards from opponent's deck
            enemy = player.enemy if player else None
            if enemy and enemy.library_cards:
                mill_count = min(3, len(enemy.library_cards))
                trashed = enemy.library_cards[:mill_count]
                enemy.library_cards = enemy.library_cards[mill_count:]
                enemy.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
