from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
import random
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_075(CardScript):
    """BT14-075 Devimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Trash the top 3 cards of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-075 Trash 3 cards from deck top")
        effect0.set_effect_description("[On Play] Trash the top 3 cards of your deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Attacking] Trash the top 3 cards of your deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-075 Trash 3 cards from deck top")
        effect1.set_effect_description("[When Attacking] Trash the top 3 cards of your deck.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Your Turn] This Digimon gets +1000 DP for every 3 cards in your trash.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-075 Your Turn DP boost")
        effect2.set_effect_description("[Your Turn] This Digimon gets +1000 DP for every 3 cards in your trash.")

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def dp_mod2(context: Dict[str, Any]) -> int:
            owner = card.owner if card else None
            if not owner:
                return 0
            return (len(owner.trash_cards) // 3) * 1000

        effect2.dp_modifier = dp_mod2
        effects.append(effect2)

        # [On Deletion] Trash 1 card in your opponent's hand without looking.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-075 Trash 1 card from opponent's hand")
        effect3.set_effect_description("[On Deletion] Trash 1 card in your opponent's hand without looking.")
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy
            if not (enemy and enemy.hand_cards):
                return
            idx = random.randrange(len(enemy.hand_cards))
            selected = enemy.hand_cards.pop(idx)
            enemy.trash_cards.append(selected)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
