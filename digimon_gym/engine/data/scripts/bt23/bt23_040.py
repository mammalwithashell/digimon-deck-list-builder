from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_040(CardScript):
    """BT23-040"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By placing 1 of your [Erika Mishima] as this Digimon's bottom digivolution card, this Digimon may digivolve into [Hudiemon] in the hand or trash with the digivolution cost reduced by 2.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-040 By placing 1 [Erika Mishima] as bottom digivolution card, this digimon may digivolve into a [Hudie] digimon in hand/trash for 2 reduced cost")
        effect1.set_effect_description("[Start of Your Main Phase] By placing 1 of your [Erika Mishima] as this Digimon's bottom digivolution card, this Digimon may digivolve into [Hudiemon] in the hand or trash with the digivolution cost reduced by 2.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Erika Mishima'))):
                return False
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

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-040 All your Digimon DP modifier")
        effect2.set_effect_description("All your Digimon DP modifier")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 1000
        effect2._applies_to_all_own_digimon = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
