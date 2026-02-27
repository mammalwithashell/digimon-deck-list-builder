from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_047(CardScript):
    """BT14-047 Dokugumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Suspend 1 of your opponent's Digimon.
        # During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-047 Suspend 1 Digimon and opponent's Digimons can't unsuspend")
        effect0.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.")
        effect0.is_on_play = True
        effect0._is_cannot_unsuspend_player = True

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

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=None, is_optional=False
            )

            # Apply the player-level next-unsuspend restriction to the opponent side.
            if hasattr(game, 'get_opponent_player'):
                opponent = game.get_opponent_player(player)
                if opponent and hasattr(opponent, 'grant_keyword'):
                    opponent.grant_keyword('_is_cannot_unsuspend_player')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Digivolving] Suspend 1 of your opponent's Digimon.
        # During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-047 Suspend 1 Digimon and opponent's Digimons can't unsuspend")
        effect1.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. During your opponent's next unsuspend phase, all of your opponent's Digimon with 5000 DP or less don't unsuspend.")
        effect1.is_when_digivolving = True
        effect1._is_cannot_unsuspend_player = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=None, is_optional=False
            )

            # Apply the player-level next-unsuspend restriction to the opponent side.
            if hasattr(game, 'get_opponent_player'):
                opponent = game.get_opponent_player(player)
                if opponent and hasattr(opponent, 'grant_keyword'):
                    opponent.grant_keyword('_is_cannot_unsuspend_player')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
