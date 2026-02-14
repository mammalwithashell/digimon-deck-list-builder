from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_015(CardScript):
    """BT23-015"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-015 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have a Tamer with the [Zaxon] trait, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-015 Reduce play cost by 5")
        effect1.set_effect_description("When this card would be played, if you have a Tamer with the [Zaxon] trait, reduce the play cost by 5.")
        effect1.set_hash_string("BT23_015_ReducePlayCost")
        effect1.cost_reduction = 5

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -5"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction handled via cost_reduction property

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Cost -5
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-015 Play Cost -5")
        effect2.set_effect_description("Cost -5")
        effect2.cost_reduction = 5

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -5"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction handled via cost_reduction property

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] [Once Per Turn] Delete 1 of your opponent's Digimon with 9000 DP or less. Then, you may return up to 3 non-Digi-Egg cards from their trash to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-015 Delete 1 digimon with 9K DP or less. then you may return up to 3 non egg cards from trash to bottom of deck")
        effect3.set_effect_description("[On Play] [Once Per Turn] Delete 1 of your opponent's Digimon with 9000 DP or less. Then, you may return up to 3 non-Digi-Egg cards from their trash to the bottom of the deck.")
        effect3.set_hash_string("BT23_015_OP/WD/WA")
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
        # [When Digivolving] [Once Per Turn] Delete 1 of your opponent's Digimon with 9000 DP or less. Then, you may return up to 3 non-Digi-Egg cards from their trash to the bottom of the deck.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-015 Delete 1 digimon with 9K DP or less. then you may return up to 3 non egg cards from trash to bottom of deck")
        effect4.set_effect_description("[When Digivolving] [Once Per Turn] Delete 1 of your opponent's Digimon with 9000 DP or less. Then, you may return up to 3 non-Digi-Egg cards from their trash to the bottom of the deck.")
        effect4.set_hash_string("BT23_015_OP/WD/WA")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with 9000 DP or less. Then, you may return up to 3 non-Digi-Egg cards from their trash to the bottom of the deck.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-015 Delete 1 digimon with 9K DP or less. then you may return up to 3 non egg cards from trash to bottom of deck")
        effect5.set_effect_description("[When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with 9000 DP or less. Then, you may return up to 3 non-Digi-Egg cards from their trash to the bottom of the deck.")
        effect5.set_hash_string("BT23_015_OP/WD/WA")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place this card face up as the bottom security card.
        effect6 = ICardEffect()
        effect6.set_effect_name("BT23-015 Place face up as bottom security card")
        effect6.set_effect_description("[On Deletion] Place this card face up as the bottom security card.")
        effect6.is_on_deletion = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
