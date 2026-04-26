from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_005(CardScript):
    """BT8-005 Kyokyomon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn][Once Per Turn] When an effect places a digivolution card under this Digimon, it gets +1000 DP until the end of your opponent's next turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-005 DP +1000")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When an effect places a digivolution card under this Digimon, it gets +1000 DP until the end of your opponent's next turn.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP+1000_BT8_005")
        effect0.dp_modifier = 1000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +1000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(1000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
