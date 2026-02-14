from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_067(CardScript):
    """BT23-067"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-067 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played from the hand, if you have [Angewomon] or [Mirei Mikagura], reduce the play cost by 3.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-067 Play cost reduction -3")
        effect1.set_effect_description("When this card would be played from the hand, if you have [Angewomon] or [Mirei Mikagura], reduce the play cost by 3.")
        effect1.set_hash_string("BT23_067_ReducePlayCost")
        effect1.cost_reduction = 3

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -3"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction handled via cost_reduction property

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Cost -3
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-067 Play Cost -3")
        effect2.set_effect_description("Cost -3")
        effect2.cost_reduction = 3

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -3"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction handled via cost_reduction property

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-067 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's level 4 or lower Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-067 Delete 1 of your opponent's level 4 or lower Digimon")
        effect4.set_effect_description("[On Play] Delete 1 of your opponent's level 4 or lower Digimon.")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-067 Delete 1 of your opponent's level 4 or lower Digimon")
        effect5.set_effect_description("[When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon.")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
