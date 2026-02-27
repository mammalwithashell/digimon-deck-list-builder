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

        def _build_shared_effect(name: str, description: str, on_play: bool = False, when_digivolving: bool = False) -> ICardEffect:
            effect = ICardEffect()
            effect.set_effect_name(name)
            effect.set_effect_description(description)
            effect.is_on_play = on_play
            effect.is_when_digivolving = when_digivolving

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game and player.enemy):
                    return

                enemy = player.enemy
                digimon_count = 0

                def reveal_filter(c):
                    return True

                def on_revealed(selected, remaining):
                    nonlocal digimon_count
                    revealed = []
                    if selected is not None:
                        if isinstance(selected, list):
                            revealed.extend(selected)
                        else:
                            revealed.append(selected)
                    if remaining:
                        revealed.extend(remaining)

                    for c in revealed:
                        if getattr(c, 'card_kind', None) == 0:
                            digimon_count += 1

                    # Return all revealed cards to opponent's deck via engine flow.
                    for c in revealed:
                        enemy.library_cards.append(c)

                game.effect_reveal_and_select(
                    enemy, 3, reveal_filter, on_revealed, is_optional=False
                )

                for _ in range(digimon_count):
                    def on_de_digivolve(target_perm):
                        removed = target_perm.de_digivolve(1)
                        if removed:
                            enemy.trash_cards.extend(removed)

                    game.effect_select_opponent_permanent(
                        player,
                        on_de_digivolve,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                    )

            effect.set_on_process_callback(process)
            return effect

        effect0 = _build_shared_effect(
            "BT14-065 Reveal opponent top 3 and De-Digivolve",
            "[On Play] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.",
            on_play=True,
        )
        effects.append(effect0)

        effect1 = _build_shared_effect(
            "BT14-065 Reveal opponent top 3 and De-Digivolve",
            "[When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.",
            when_digivolving=True,
        )
        effects.append(effect1)

        return effects
