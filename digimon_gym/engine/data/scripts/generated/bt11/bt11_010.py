from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_010(CardScript):
    """BT11-010 Grizzlymon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
        # Raid
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-010 Raid")
        effect0.set_effect_description("Raid")
        effect0._is_raid = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [Your Turn][Once Per Turn] When this Digimon's attack target is switched, this Digimon gets +3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-010 DP +3000")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When this Digimon's attack target is switched, this Digimon gets +3000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP+3000_BT11_010")
        effect1.dp_modifier = 3000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
