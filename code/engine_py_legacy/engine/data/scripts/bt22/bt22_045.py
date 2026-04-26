from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_045(CardScript):
    """BT22-045 WezenGammamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-045 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Gammamon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Gammamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Gammamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 Digimon card with [Gammamon] in its name from your hand as this Digimon's bottom digivolution card, it gains <Blocker> and +3000 DP until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-045 Tuck gammamon to gain <Blocker> +3000 DP")
        effect1.set_effect_description("[On Play] By placing 1 Digimon card with [Gammamon] in its name from your hand as this Digimon's bottom digivolution card, it gains <Blocker> and +3000 DP until your opponent's turn ends.")
        effect1.is_optional = True
        effect1.is_on_play = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if perm:
                perm.grant_keyword('_is_blocker')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 Digimon card with [Gammamon] in its name from your hand as this Digimon's bottom digivolution card, it gains <Blocker> and +3000 DP until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-045 Tuck gammamon to gain <Blocker> +3000 DP")
        effect2.set_effect_description("[On Play] By placing 1 Digimon card with [Gammamon] in its name from your hand as this Digimon's bottom digivolution card, it gains <Blocker> and +3000 DP until your opponent's turn ends.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True
        effect2._is_blocker = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if perm:
                perm.grant_keyword('_is_blocker')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
