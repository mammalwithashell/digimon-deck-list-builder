from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_031(CardScript):
    """BT23-031"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-031 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played from the hand, if you have [LadyDevimon] or [Mirei Mikagura], reduce the play cost by 3.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-031 Play cost reduction -3")
        effect1.set_effect_description("When this card would be played from the hand, if you have [LadyDevimon] or [Mirei Mikagura], reduce the play cost by 3.")
        effect1.set_hash_string("BT23_031_ReducePlayCost")
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
        effect2.set_effect_name("BT23-031 Play Cost -3")
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Add your top security card to the hand. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-031 Add top security to hand. then if you have 3 or less security, <Recovery +1>")
        effect3.set_effect_description("[On Play] Add your top security card to the hand. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Add your top security card to the hand. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-031 Add top security to hand. then if you have 3 or less security, <Recovery +1>")
        effect4.set_effect_description("[When Digivolving] Add your top security card to the hand. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: alliance
        # Alliance
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-031 Alliance")
        effect5.set_effect_description("Alliance")
        effect5.is_inherited_effect = True
        effect5._is_alliance = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
