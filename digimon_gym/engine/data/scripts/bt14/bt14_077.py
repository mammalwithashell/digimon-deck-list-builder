from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_077(CardScript):
    """BT14-077 SkullSatamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] [When Digivolving] Trash the top 2 cards of both players' decks.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-077 Trash top 2 of both decks")
        effect0.set_effect_description("[On Play] [When Digivolving] Trash the top 2 cards of both players' decks.")
        effect0.is_on_play = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            game = ctx.get('game')
            if game is None:
                return
            for player in getattr(game, 'players', []):
                for _ in range(2):
                    if hasattr(player, 'trash_from_top_of_deck'):
                        player.trash_from_top_of_deck(1)
                    elif hasattr(player, 'trash_top_deck'):
                        player.trash_top_deck(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn][Once Per Turn] When a card in your opponent's deck is trashed, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-077 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When a card in your opponent's deck is trashed, gain 1 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+_BT14_077")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            trashed_player = context.get('player')
            if trashed_player is None or card.owner is None:
                return False

            # Must be opponent's deck card being trashed.
            if trashed_player == card.owner:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player is not None and hasattr(player, 'opponent') and player.opponent is not None:
                player.opponent.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
