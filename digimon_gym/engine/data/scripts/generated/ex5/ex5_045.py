from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_045(CardScript):
    """EX5-045 Chuumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If it's your opponent's turn, reveal the top 3 cards of your deck. You may play 1 Digimon card with [Sukamon] in its name among them without paying the cost. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-045 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Play] If it's your opponent's turn, reveal the top 3 cards of your deck. You may play 1 Digimon card with [Sukamon] in its name among them without paying the cost. Trash the rest.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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
                if not (any('Sukamon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Sukamon' in _n for _n in getattr(c, 'card_names', []))):
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

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If this Digimon had [Sukamon]/[Etemon] in its name, you may play 1 [Chuumon] from your trash suspended without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-045 Play 1 [Chuumon] from trash")
        effect1.set_effect_description("[On Deletion] If this Digimon had [Sukamon]/[Etemon] in its name, you may play 1 [Chuumon] from your trash suspended without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
