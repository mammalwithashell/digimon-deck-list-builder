from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_033(CardScript):
    """BT23-033"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-033 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-033 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-033 link 1 level 4- digimon from trash or digivoultion sources to this digimon")
        effect2.set_effect_description("[On Play] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.")
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
        # [When Digivolving] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-033 link 1 level 4- digimon from trash or digivoultion sources to this digimon")
        effect3.set_effect_description("[When Digivolving] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then to 1 of your opponent's Digimon, give -1000 DP until their turn ends for each of your security cards.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-033 If 5 or less security, <Recovery +1(Deck)>. then -1k DP for each security card")
        effect4.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then to 1 of your opponent's Digimon, give -1000 DP until their turn ends for each of your security cards.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT23_033_YT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
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

        # Timing: EffectTiming.WhenLinked
        # [When Linking] Until your opponent's turn ends, their effects can't return this Digimon to hands or decks or affect it with <De-Digivolve> effects.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-033 Gain deck and hand bounce & De-Digivolve immunity")
        effect5.set_effect_description("[When Linking] Until your opponent's turn ends, their effects can't return this Digimon to hands or decks or affect it with <De-Digivolve> effects.")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
