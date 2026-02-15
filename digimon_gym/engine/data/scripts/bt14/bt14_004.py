from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_004(CardScript):
    """BT14-004 Tanemon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your effects suspends a Tamer, this Digimon gets +2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-004 DP +2000")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When one of your effects suspends a Tamer, this Digimon gets +2000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP+2000_BT14_004")
        effect0.dp_modifier = 2000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
