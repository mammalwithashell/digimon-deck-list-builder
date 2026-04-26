from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_006(CardScript):
    """BT11-006 Tsunomon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn][Once Per Turn] When an effect trashes a card in your hand, this Digimon gets +1000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-006 DP +1000")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When an effect trashes a card in your hand, this Digimon gets +1000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP+1000_BT11_006")
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
