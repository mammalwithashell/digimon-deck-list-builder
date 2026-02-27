from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_063(CardScript):
    """BT14-063 BlackKingNumemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-063 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Reveal the top 3 cards of your deck. From among them, add 1 card with [Monzaemon] in its name to your hand and play 1 Digimon card with [Numemon] in its name without paying the cost. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-063 Reveal the top 3 cards of deck")
        effect1.set_effect_description("[On Deletion] Reveal the top 3 cards of your deck. From among them, add 1 card with [Monzaemon] in its name to your hand and play 1 Digimon card with [Numemon] in its name without paying the cost. Return the rest to the bottom of the deck.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_0(c):
                return any('Monzaemon' in _n for _n in getattr(c, 'card_names', []))

            def reveal_filter_1(c):
                return getattr(c, 'is_digimon', False) and any(
                    'Numemon' in _n for _n in getattr(c, 'card_names', [])
                )

            game.effect_reveal_and_select_multi(
                player,
                3,
                [(reveal_filter_0, 'hand'), (reveal_filter_1, 'play_free')],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-063 Blocker")
        effect2.set_effect_description("Blocker")
        effect2.is_inherited_effect = True
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
