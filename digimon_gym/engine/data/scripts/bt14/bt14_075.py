from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_075(CardScript):
    """BT14-075 Devimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 3 cards of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-075 Trash 3 cards from deck top")
        effect0.set_effect_description("[On Play] Trash the top 3 cards of your deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Trash the top 3 cards of your deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-075 Trash 3 cards from deck top")
        effect1.set_effect_description("[When Attacking] Trash the top 3 cards of your deck.")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Your Turn] This Digimon gets +1000 DP for every 3 cards in your trash.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-075 DP +1000 per 3 cards in trash")
        effect2.set_effect_description("[Your Turn] This Digimon gets +1000 DP for every 3 cards in your trash.")

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def dp_modifier2(ctx: Dict[str, Any]) -> int:
            player = ctx.get('player')
            if player is None and card is not None:
                player = card.owner
            if not player:
                return 0
            trash_count = len(player.trash_cards) if player.trash_cards else 0
            return (trash_count // 3) * 1000

        effect2.dp_modifier = dp_modifier2
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Trash 1 card in your opponent's hand without looking.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-075 Trash 1 card from opponent's hand")
        effect3.set_effect_description("[On Deletion] Trash 1 card in your opponent's hand without looking.")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Opponent Hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return
            enemy = player.enemy

            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in enemy.hand_cards:
                    enemy.hand_cards.remove(selected)
                    enemy.trash_cards.append(selected)

            game.effect_select_hand_card(
                enemy, hand_filter, on_trashed, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
