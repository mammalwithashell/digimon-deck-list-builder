from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_067(CardScript):
    """BT14-067 Ebemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-067 Reveal top 3 and delete by chosen play cost")
        effect0.set_effect_description("[On Play] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return

            enemy = player.enemy

            def reveal_filter(c):
                return getattr(c, 'is_digimon', False)

            def on_revealed(selected, remaining):
                max_total = getattr(selected, 'play_cost', 0) if selected is not None else 0

                def target_filter(p):
                    return getattr(p, 'is_digimon', False)

                def on_delete(target_perm_list):
                    for t in target_perm_list or []:
                        enemy.delete_permanent(t)

                # Delete up to chosen card's play cost total worth.
                game.effect_select_opponent_permanents_upto_total_play_cost(
                    player,
                    max_total,
                    on_delete,
                    filter_fn=target_filter,
                    is_optional=True,
                )

                # Chosen and remaining revealed cards are returned to opponent's deck.
                enemy.library_cards.append(selected)
                for c in remaining:
                    enemy.library_cards.append(c)

            game.effect_reveal_and_select(
                enemy, 3, reveal_filter, on_revealed, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-067 Reveal top 3 and delete by chosen play cost")
        effect1.set_effect_description("[When Digivolving] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return

            enemy = player.enemy

            def reveal_filter(c):
                return getattr(c, 'is_digimon', False)

            def on_revealed(selected, remaining):
                max_total = getattr(selected, 'play_cost', 0) if selected is not None else 0

                def target_filter(p):
                    return getattr(p, 'is_digimon', False)

                def on_delete(target_perm_list):
                    for t in target_perm_list or []:
                        enemy.delete_permanent(t)

                # Delete up to chosen card's play cost total worth.
                game.effect_select_opponent_permanents_upto_total_play_cost(
                    player,
                    max_total,
                    on_delete,
                    filter_fn=target_filter,
                    is_optional=True,
                )

                # Chosen and remaining revealed cards are returned to opponent's deck.
                enemy.library_cards.append(selected)
                for c in remaining:
                    enemy.library_cards.append(c)

            game.effect_reveal_and_select(
                enemy, 3, reveal_filter, on_revealed, is_optional=False
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
