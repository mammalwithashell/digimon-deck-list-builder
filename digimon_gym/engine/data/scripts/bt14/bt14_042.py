from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_042(CardScript):
    """BT14-042 Kunemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] By suspending this Digimon, reveal the top 3 cards of your deck.
        # Add 1 green card among them to your hand.
        # Return the rest to the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-042 On Play suspend self to reveal top 3")
        effect0.set_effect_description("[On Play] By suspending this Digimon, reveal the top 3 cards of your deck. Add 1 green card among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            permanent = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and permanent):
                return

            # Cost: suspend this Digimon
            permanent.suspend()

            def reveal_filter(c):
                return 'Green' in [col.name for col in getattr(c, 'card_colors', [])]

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player,
                3,
                reveal_filter,
                on_revealed,
                is_optional=True,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
