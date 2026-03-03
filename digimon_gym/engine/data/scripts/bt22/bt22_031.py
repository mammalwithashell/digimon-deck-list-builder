from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_031(CardScript):
    """BT22-031 GoldNumemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-031 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 4
        effect0._alt_digi_color = CardColor.Yellow

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Give 1 of your opponent's Digimon <Security A. -2> (This Digimon checks 2 additional security cards.) until their turn ends. Then, if this Digimon's stack has 2 or more same-level cards, this Digimon may digivolve into [PlatinumNumemon] in the hand for a digivolution cost of 4, ignoring digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-031 Give 1 digimon -2 Sec Atk. then if sources has 2 same level digimon, digivolve into [PlatinumNumemon]")
        effect1.set_effect_description("[On Play] Give 1 of your opponent's Digimon <Security A. -2> (This Digimon checks 2 additional security cards.) until their turn ends. Then, if this Digimon's stack has 2 or more same-level cards, this Digimon may digivolve into [PlatinumNumemon] in the hand for a digivolution cost of 4, ignoring digivolution requirements.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Give 1 of your opponent's Digimon <Security A. -2> (This Digimon checks 2 additional security cards.) until their turn ends. Then, if this Digimon's stack has 2 or more same-level cards, this Digimon may digivolve into [PlatinumNumemon] in the hand for a digivolution cost of 4, ignoring digivolution requirements.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-031 Give 1 digimon -2 Sec Atk. then if sources has 2 same level digimon, digivolve into [PlatinumNumemon]")
        effect2.set_effect_description("[When Digivolving] Give 1 of your opponent's Digimon <Security A. -2> (This Digimon checks 2 additional security cards.) until their turn ends. Then, if this Digimon's stack has 2 or more same-level cards, this Digimon may digivolve into [PlatinumNumemon] in the hand for a digivolution cost of 4, ignoring digivolution requirements.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Cost -1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-031 Cost -1")
        effect3.set_effect_description("Cost -1")
        effect3.is_inherited_effect = True
        effect3.cost_reduction = 1

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
