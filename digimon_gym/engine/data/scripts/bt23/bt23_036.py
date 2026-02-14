from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_036(CardScript):
    """BT23-036"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if your opponent has a Digimon with 10000 DP or more, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-036 Reduce the play cost by 5")
        effect1.set_effect_description("When this card would be played, if your opponent has a Digimon with 10000 DP or more, reduce the play cost by 5.")
        effect1.set_hash_string("PlayCost-5_BT23_036")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-036 Play Cost -5")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your other Digimon may digivolve into a level 6 or lower Digimon card with [Leomon] in its name or the [CS] trait in the hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-036 Digivolve into a level 6 with [Leomon] in name or [CS] trait")
        effect3.set_effect_description("[On Play] 1 of your other Digimon may digivolve into a level 6 or lower Digimon card with [Leomon] in its name or the [CS] trait in the hand without paying the cost.")
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
        # [When Digivolving] 1 of your other Digimon may digivolve into a level 6 or lower Digimon card with [Leomon] in its name or the [CS] trait in the hand without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-036 Digivolve into a level 6 with [Leomon] in name or [CS] trait")
        effect4.set_effect_description("[When Digivolving] 1 of your other Digimon may digivolve into a level 6 or lower Digimon card with [Leomon] in its name or the [CS] trait in the hand without paying the cost.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 1 of your Digimon gains <Raid> for the turn, Then that Digimon may attack.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-036 Give 1 digimon raid, then it may attack")
        effect5.set_effect_description("[End of Your Turn] [Once Per Turn] 1 of your Digimon gains <Raid> for the turn, Then that Digimon may attack.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_036_EOYT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
