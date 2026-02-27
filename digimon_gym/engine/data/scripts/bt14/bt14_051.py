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

        # [End of Opponent's Turn][Once Per Turn] By suspending 1 of your Digimon,
        # reveal the top 5 cards of your deck. Add 2 green Digimon cards among them
        # to your hand. Return the rest to the bottom of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-051 Suspend 1 of your Digimon to reveal top 5")
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
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: suspend 1 of your Digimon.
            def own_digimon_filter(p):
                owner = getattr(p, 'owner', None)
                if owner is not None and owner != player:
                    return False
                if getattr(p, 'is_suspended', False):
                    return False
                return True

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_permanent(
                player,
                on_suspend,
                filter_fn=own_digimon_filter,
                is_optional=True,
            )

            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return 'Green' in [col.name for col in getattr(c, 'card_colors', [])]

            def on_revealed(selected, remaining):
                selected_cards = selected if isinstance(selected, list) else ([selected] if selected is not None else [])
                for c in selected_cards[:2]:
                    player.hand_cards.append(c)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player,
                5,
                reveal_filter,
                on_revealed,
                select_count=2,
                is_optional=True,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
