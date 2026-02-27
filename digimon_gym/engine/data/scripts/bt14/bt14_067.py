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

        def _process_reveal_then_delete(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and getattr(player, 'enemy', None)):
                return

            enemy = player.enemy

            def reveal_filter(c):
                return bool(getattr(c, 'is_digimon', False))

            def on_revealed(selected, remaining):
                selected_cost = int(getattr(selected, 'play_cost', 0) or 0) if selected else 0

                if selected_cost > 0:
                    deleted_total = 0

                    def target_filter(p):
                        if deleted_total >= selected_cost:
                            return False
                        if not getattr(p, 'is_digimon', False):
                            return False
                        play_cost = int(getattr(getattr(p, 'card', None), 'play_cost', 0) or 0)
                        return play_cost <= (selected_cost - deleted_total)

                    def on_delete(target_perm):
                        nonlocal deleted_total
                        play_cost = int(getattr(getattr(target_perm, 'card', None), 'play_cost', 0) or 0)
                        enemy.delete_permanent(target_perm)
                        deleted_total += max(0, play_cost)

                    while deleted_total < selected_cost:
                        before = deleted_total
                        game.effect_select_opponent_permanent(
                            player,
                            on_delete,
                            filter_fn=target_filter,
                            is_optional=True,
                        )
                        if deleted_total == before:
                            break

            game.effect_reveal_and_select(
                enemy,
                3,
                reveal_filter,
                on_revealed,
                is_optional=True,
            )

        # [On Play]
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-067 Reveal and delete by selected play cost")
        effect0.set_effect_description("[On Play] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_process_reveal_then_delete)
        effects.append(effect0)

        # [When Digivolving]
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-067 Reveal and delete by selected play cost")
        effect1.set_effect_description("[When Digivolving] Your opponent reveals the top 3 cards of their deck. Choose 1 Digimon card among them, and delete up to its play cost's total worth of your opponent's Digimon. Return the revealed cards to the top or bottom of the deck.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_process_reveal_then_delete)
        effects.append(effect1)

        return effects
