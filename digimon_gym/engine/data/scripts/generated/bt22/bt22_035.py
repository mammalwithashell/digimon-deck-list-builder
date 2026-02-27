from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_035(CardScript):
    """BT22-035 Entermon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-035 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-035 Alliance")
        effect1.set_effect_description("Alliance")
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-035 Link a level 4 or lower digimon from hand or sources")
        effect2.set_effect_description("[On Play] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect2.is_optional = True
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
        # [When Digivolving] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-035 Link a level 4 or lower digimon from hand or sources")
        effect3.set_effect_description("[When Digivolving] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect3.is_optional = True
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
        # [Your Turn] [Once Per Turn] When this Digimon gets linked, you may play 1 play cost 4 or lower card with the [Appmon] trait from your hand without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-035 Play 1 4 cost or lower [Appmon] card")
        effect4.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon gets linked, you may play 1 play cost 4 or lower card with the [Appmon] trait from your hand without paying the cost.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT22_035_WL")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenLinked
        # [When Linking] To 1 of your opponent's Digimon, give -4000 DP for the turn for each of your Digimon with the [Appmon] trait.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT22-035 Effect")
        effect5.set_effect_description("[When Linking] To 1 of your opponent's Digimon, give -4000 DP for the turn for each of your Digimon with the [Appmon] trait.")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
