from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_044(CardScript):
    """BT23-044"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-044 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played from the hand, if you have [Yuuko Kamishiro] or a [CS] trait Digimon, reduce the play cost by 3.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-044 Play cost reduction -3")
        effect1.set_effect_description("When this card would be played from the hand, if you have [Yuuko Kamishiro] or a [CS] trait Digimon, reduce the play cost by 3.")
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
        effect2.set_effect_name("BT23-044 Play Cost -3")
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
        # [On Play] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-044 By suspending 1 digimon, 1 digimon cant be bounced to hand & deck")
        effect3.set_effect_description("[On Play] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.")
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
        # [When Digivolving] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-044 By suspending 1 digimon, 1 digimon cant be bounced to hand & deck")
        effect4.set_effect_description("[When Digivolving] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-044 Trash opponent top security")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_044_AT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
