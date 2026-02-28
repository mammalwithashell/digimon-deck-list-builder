from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_037(CardScript):
    """EX5-037 Vajramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Draw 1> (Draw 1 card from your deck). Then, you may play 1 [Deva] trait Digimon card without the same name as the cards in your battle area or trash from your hand to an empty space in your breeding area without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-037 Draw 1 and play 1 Digimon from hand on breeding area")
        effect0.set_effect_description("[On Play] <Draw 1> (Draw 1 card from your deck). Then, you may play 1 [Deva] trait Digimon card without the same name as the cards in your battle area or trash from your hand to an empty space in your breeding area without paying the cost.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseOption
        # [Your Turn] When you use an use Option card with a cost of 1 or more, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-037 Memory +1")
        effect1.set_effect_description("[Your Turn] When you use an use Option card with a cost of 1 or more, gain 1 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_EX5_037")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
