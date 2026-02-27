from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_061(CardScript):
    """BT14-061 Vegiemon | Lv.4"""

    def _return_opponent_digimon_from_trash_to_top(self, ctx: Dict[str, Any]) -> bool:
        player = ctx.get('player')
        game = ctx.get('game')
        if not player or not game:
            return False

        opponent = game.get_opponent_player(player)
        if opponent is None:
            return False

        trash = getattr(opponent, 'trash', None)
        if trash is None:
            return False

        candidates = [c for c in list(trash) if getattr(c, 'card_kind', None) == 0]
        if not candidates:
            return False

        target = candidates[0]
        trash.remove(target)
        opponent.deck.insert(0, target)
        return True

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-061 Return 1 opponent Digimon from trash to deck top to gain Memory +1")
        effect0.set_effect_description("[On Play] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if self._return_opponent_digimon_from_trash_to_top(ctx) and player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-061 Return 1 opponent Digimon from trash to deck top to gain Memory +1")
        effect1.set_effect_description("[When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of their deck, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if self._return_opponent_digimon_from_trash_to_top(ctx) and player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
