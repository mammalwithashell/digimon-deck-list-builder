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

        def _find_opponent_trash_digimon(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player is None or game is None:
                return None, None

            opponent = None
            if hasattr(game, 'get_opponent_player'):
                opponent = game.get_opponent_player(player)
            elif hasattr(player, 'get_opponent_player'):
                opponent = player.get_opponent_player()
            elif hasattr(player, 'opponent'):
                opponent = player.opponent

            if opponent is None:
                return None, None

            trash = getattr(opponent, 'trash', None)
            if trash is None:
                return opponent, None

            for c in list(trash):
                is_digimon = False
                if hasattr(c, 'is_digimon_card'):
                    try:
                        is_digimon = bool(c.is_digimon_card())
                    except Exception:
                        is_digimon = False
                if not is_digimon and hasattr(c, 'card_kind'):
                    is_digimon = getattr(c, 'card_kind', None) == 0
                if is_digimon:
                    return opponent, c
            return opponent, None

        def _return_to_top_of_deck(opponent, card_in_trash) -> bool:
            if opponent is None or card_in_trash is None:
                return False
            trash = getattr(opponent, 'trash', None)
            if trash is None:
                return False

            try:
                trash.remove(card_in_trash)
            except Exception:
                return False

            deck = getattr(opponent, 'deck', None)
            if deck is None:
                # revert if no deck container exists
                trash.append(card_in_trash)
                return False

            if hasattr(deck, 'insert'):
                deck.insert(0, card_in_trash)
                return True
            if hasattr(deck, 'appendleft'):
                deck.appendleft(card_in_trash)
                return True
            if hasattr(deck, 'append'):
                # best effort fallback if top insertion API is unavailable
                deck.append(card_in_trash)
                return True

            # revert if deck is not mutable in known ways
            trash.append(card_in_trash)
            return False

        # [On Play] ... by returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-061 On Play return 1 opponent trash Digimon to deck top to gain Memory +1")
        effect0.set_effect_description("[On Play] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            _, target = _find_opponent_trash_digimon(context)
            return target is not None

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            opponent, target = _find_opponent_trash_digimon(ctx)
            if _return_to_top_of_deck(opponent, target) and player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Digivolving] ... by returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-061 When Digivolving return 1 opponent trash Digimon to deck top to gain Memory +1")
        effect1.set_effect_description("[When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            _, target = _find_opponent_trash_digimon(context)
            return target is not None

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            opponent, target = _find_opponent_trash_digimon(ctx)
            if _return_to_top_of_deck(opponent, target) and player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
