from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_035(CardScript):
    """BT23-035"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-035 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Witchelny] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "Witchelny"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Witchelny' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-035 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-035 By trashing your top security, -6000 all opponent digimon")
        effect2.set_effect_description("[On Play] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.")
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
        # [When Digivolving] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-035 By trashing your top security, -6000 all opponent digimon")
        effect3.set_effect_description("[When Digivolving] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # When your security stack is removed from, this Digimon gains <Security A. +1> until your turn ends. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-035 Gain Sec +1. then if you are 3- security, Recovery")
        effect4.set_effect_description("When your security stack is removed from, this Digimon gains <Security A. +1> until your turn ends. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT23_035_AT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
