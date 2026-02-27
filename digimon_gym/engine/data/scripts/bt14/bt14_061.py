from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_061(CardScript):
    """BT14-061 Vegiemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _opponent_trash_digimon(ctx: Dict[str, Any]):
            opponent = ctx.get('opponent')
            if opponent is None:
                player = ctx.get('player')
                if player is not None and hasattr(player, 'opponent'):
                    opponent = player.opponent
            if opponent is None:
                return []

            trash = getattr(opponent, 'trash', None)
            if trash is None:
                return []

            cards = list(trash)
            return [c for c in cards if getattr(c, 'card_kind', None) == 0 and getattr(c, 'level', None) is not None]

        def _resolve_return_and_gain(ctx: Dict[str, Any]):
            player = ctx.get('player')
            opponent = ctx.get('opponent')
            if opponent is None and player is not None and hasattr(player, 'opponent'):
                opponent = player.opponent
            if opponent is None:
                return

            candidates = _opponent_trash_digimon(ctx)
            if not candidates:
                return

            game = ctx.get('game')
            chosen = None
            if game is not None and hasattr(game, 'choose_cards'):
                selected = game.choose_cards(
                    ctx=ctx,
                    player=player,
                    cards=candidates,
                    min_count=1,
                    max_count=1,
                    prompt='Choose 1 Digimon card in your opponent\'s trash to return to the top of their deck.'
                )
                if selected:
                    chosen = selected[0]
            if chosen is None:
                chosen = candidates[0]

            trash = getattr(opponent, 'trash', None)
            deck = getattr(opponent, 'deck', None)
            if trash is None or deck is None:
                return

            if hasattr(trash, 'remove'):
                try:
                    trash.remove(chosen)
                except ValueError:
                    return
            else:
                return

            if hasattr(deck, 'insert'):
                deck.insert(0, chosen)
            elif hasattr(deck, 'appendleft'):
                deck.appendleft(chosen)
            else:
                return

            if player is not None:
                player.add_memory(1)

        # [On Play] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-061 On Play: Return 1 opponent trash Digimon to deck top to gain Memory +1")
        effect0.set_effect_description("[On Play] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return len(_opponent_trash_digimon(context)) > 0

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_resolve_return_and_gain)
        effects.append(effect0)

        # [When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-061 When Digivolving: Return 1 opponent trash Digimon to deck top to gain Memory +1")
        effect1.set_effect_description("[When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return len(_opponent_trash_digimon(context)) > 0

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_resolve_return_and_gain)
        effects.append(effect1)

        return effects
