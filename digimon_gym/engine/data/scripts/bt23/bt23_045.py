from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_045(CardScript):
    """BT23-045"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-045 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-045 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect1._alt_digi_cost = 3

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Royal Base] or [Zaxon] trait Digimon card from your hand or trash face up as the bottom security card, return 1 of your opponent's Digimon with as much or less DP as this Digimon to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-045 By placing 1 [Royal Base]/[Zaxon] as bottom security card, bounce 1 digimon to hand")
        effect2.set_effect_description("[On Play] By placing 1 [Royal Base] or [Zaxon] trait Digimon card from your hand or trash face up as the bottom security card, return 1 of your opponent's Digimon with as much or less DP as this Digimon to the hand.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 [Royal Base] or [Zaxon] trait Digimon card from your hand or trash face up as the bottom security card, return 1 of your opponent's Digimon with as much or less DP as this Digimon to the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-045 By placing 1 [Royal Base]/[Zaxon] as bottom security card, bounce 1 digimon to hand")
        effect3.set_effect_description("[When Digivolving] By placing 1 [Royal Base] or [Zaxon] trait Digimon card from your hand or trash face up as the bottom security card, return 1 of your opponent's Digimon with as much or less DP as this Digimon to the hand.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When this Digimon suspends, by flipping your top face-up security card face down, 1 of your Digimon unsuspends.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-045 By flipping your top face up security card face down, unsuspend 1 digimon")
        effect4.set_effect_description("[All Turns] When this Digimon suspends, by flipping your top face-up security card face down, 1 of your Digimon unsuspends.")
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
