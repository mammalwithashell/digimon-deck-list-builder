from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_044(CardScript):
    """BT10-044 Angoramon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play a green Tamer, <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-044 Draw 1")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When you play a green Tamer, <Draw 1>. (Draw 1 card from your deck.)")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_BT10_044")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When an opponent's Digimon becomes suspended, <Draw 1>. (Draw 1 card from your deck.)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-044 Draw 1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When an opponent's Digimon becomes suspended, <Draw 1>. (Draw 1 card from your deck.)")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Draw1_BT10_044")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
