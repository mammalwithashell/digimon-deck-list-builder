from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_009(CardScript):
    """BT23-009"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-009 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] [Once Per Turn] When this Digimon gets linked, 1 of your Digimon gets +4000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-009 1 digimon gains 4K DP")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon gets linked, 1 of your Digimon gets +4000 DP for the turn.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT23-009_WL")
        effect1.dp_modifier = 4000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(4000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] This Digimon may attack a player.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-009 This digimon may attack a player")
        effect2.set_effect_description("[End of Your Turn] [Once Per Turn] This Digimon may attack a player.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23-009_OET")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
