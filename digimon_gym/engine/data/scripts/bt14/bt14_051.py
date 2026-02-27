from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_051(CardScript):
    """BT14-051 Okuwamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent's Turn][Once Per Turn] By suspending 1 of your Digimon,
        # reveal the top 5 cards of your deck. Add 2 green Digimon cards among them
        # to the hand. Return the rest to the bottom of deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-051 Suspend your 1 Digimon to reveal the top 5 cards of deck")
        effect0.set_effect_description("[End of Opponent's Turn][Once Per Turn] By suspending 1 of your Digimon, reveal the top 5 cards of your deck. Add 2 green Digimon cards among them to the hand. Return the rest to the bottom of deck.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Reveal_BT14_051")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend 1 of your Digimon, then reveal top 5 and add up to 2 green Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def own_unsuspended_filter(p):
                return getattr(p, 'owner', None) == player and not getattr(p, 'is_suspended', False)

            def after_suspend(_selected_perm):
                def reveal_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    return 'Green' in [col.name for col in getattr(c, 'card_colors', [])]

                def on_revealed(selected, remaining):
                    # Add up to 2 green Digimon among revealed cards
                    if isinstance(selected, list):
                        for c in selected[:2]:
                            player.hand_cards.append(c)
                    elif selected is not None:
                        player.hand_cards.append(selected)
                    for c in remaining:
                        player.library_cards.append(c)

                game.effect_reveal_and_select(
                    player, 5, reveal_filter, on_revealed, is_optional=True, max_select=2
                )

            game.effect_select_permanent(
                player, after_suspend, filter_fn=own_unsuspended_filter, is_optional=True, suspend_on_select=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
