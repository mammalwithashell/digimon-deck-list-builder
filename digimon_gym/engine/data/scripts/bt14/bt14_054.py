from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_054(CardScript):
    """BT14-054 SaberLeomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] By unsuspending this Digimon, suspend 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-054 Unsuspend this Digimon to suspend opponent's 1 Digimon")
        effect0.set_effect_description("[When Digivolving] By unsuspending this Digimon, suspend 1 of your opponent's Digimon.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Cost/condition part: unsuspend this Digimon first.
            perm.unsuspend()

            # Then suspend 1 opponent Digimon.
            def target_filter(p):
                return True

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [End of Your Turn] This Digimon may attack an opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-054 This Digimon can attack to opponent's 1 Digimon")
        effect1.set_effect_description("[End of Your Turn] This Digimon may attack an opponent's Digimon.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Optional attack into opponent Digimon.
            if hasattr(game, 'effect_select_attack_opponent_digimon'):
                game.effect_select_attack_opponent_digimon(player, perm, is_optional=True)
            elif hasattr(game, 'effect_force_attack_opponent_digimon'):
                game.effect_force_attack_opponent_digimon(player, perm, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
