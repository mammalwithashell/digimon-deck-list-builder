from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_031(CardScript):
    """BT14-031 Elecmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-031 DP -2000")
        effect0.set_effect_description("[When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP-2000_BT14_031")
        effect0.is_on_attack = True
        effect0.dp_modifier = -2000

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -2000 to 1 chosen opponent Digimon"""
            player = ctx.get('player')
            if not player or not player.enemy or not player.enemy.battle_area:
                return

            dp_targets = [p for p in player.enemy.battle_area if p.is_digimon and p.dp is not None]
            if not dp_targets:
                return

            game = ctx.get('game')
            target = None
            if game and hasattr(game, 'choose_target'):
                target = game.choose_target(player, dp_targets, effect0)
            if target is None:
                target = dp_targets[0]

            target.change_dp(-2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
