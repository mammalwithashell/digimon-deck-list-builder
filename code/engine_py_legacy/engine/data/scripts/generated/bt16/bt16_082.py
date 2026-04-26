from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_082(CardScript):
    """BT16-082 Ukkomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnMove
        # [Your Turn] [Once Per Turn] When one of your Digimon moves from the breeding area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon or Tamer card among them to your hand. Return the rest to the bottom of the deck. and you may hatch in your breeding area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-082 Hatch and reveal top 3 and add a Tamer or Digimon to hand.")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When one of your Digimon moves from the breeding area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon or Tamer card among them to your hand. Return the rest to the bottom of the deck. and you may hatch in your breeding area.")
        effect0.set_max_count_per_turn(1)

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
