from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_043(CardScript):
    """BT14-043 KoDokugumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] By suspending 1 of your Digimon, suspend 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-043 Suspend your 1 Digimon to suspend opponent's 1 Digimon")
        effect0.set_effect_description("[On Play] By suspending 1 of your Digimon, suspend 1 of your opponent's Digimon.")
        effect0.is_optional = True
        effect0.is_on_play = True

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

            # Cost: suspend 1 of your Digimon (must be currently unsuspended)
            def own_filter(p):
                return getattr(p, 'owner', None) == player and not p.is_suspended()

            def after_pay_cost(_paid_perm):
                # Effect: suspend 1 of your opponent's Digimon
                def opp_filter(p):
                    return getattr(p, 'owner', None) != player

                def on_suspend_opponent(target_perm):
                    target_perm.suspend()

                game.effect_select_opponent_permanent(
                    player,
                    on_suspend_opponent,
                    filter_fn=opp_filter,
                    is_optional=False,
                )

            game.effect_select_my_permanent(
                player,
                after_pay_cost,
                filter_fn=own_filter,
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
