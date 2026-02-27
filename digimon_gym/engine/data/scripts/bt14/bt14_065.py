from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_065(CardScript):
    """BT14-065 Vademon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] ...
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-065 Reveal top 3 and De-Digivolve")
        effect0.set_effect_description("[On Play] [When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the cards to either the top or bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process_common(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return

            enemy = player.enemy
            digimon_count = {'n': 0}

            def reveal_filter(c):
                return True

            def on_revealed(selected, remaining):
                revealed = []
                if selected is not None:
                    revealed.append(selected)
                revealed.extend(remaining)

                count = 0
                for c in revealed:
                    if getattr(c, 'card_kind', None) == 0:
                        count += 1
                digimon_count['n'] = count

                # Return all revealed cards to opponent's deck (top/bottom handled by engine)
                for c in revealed:
                    enemy.library_cards.append(c)

            game.effect_reveal_and_select(
                enemy, 3, reveal_filter, on_revealed, is_optional=False
            )

            for _ in range(digimon_count['n']):
                def on_de_digivolve(target_perm):
                    target_perm.de_digivolve(1)

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False
                )

        effect0.set_on_process_callback(process_common)
        effects.append(effect0)

        # [When Digivolving] ...
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-065 Reveal top 3 and De-Digivolve")
        effect1.set_effect_description("[On Play] [When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the cards to either the top or bottom of the deck.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(process_common)
        effects.append(effect1)

        return effects
