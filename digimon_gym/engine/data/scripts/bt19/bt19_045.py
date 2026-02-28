from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_045(CardScript):
    """BT19-045 FunBeemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-045 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.2 with [Royal Base] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "Royal Base"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Base' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # [Your Turn] When this Digimon would digivolve into a Digimon card with the [Royal Base] trait, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-045 Digivolution Cost -1")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into a Digimon card with the [Royal Base] trait, reduce the digivolution cost by 1.")
        effect1.set_hash_string("DigivoltuionCost-1_BT19_045")
        effect1.cost_reduction = 1

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: dp_modifier
        # DP modifier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-045 DP modifier")
        effect2.set_effect_description("DP modifier")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 1000

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-045 All your Digimon DP modifier")
        effect3.set_effect_description("All your Digimon DP modifier")
        effect3.dp_modifier = 1000
        effect3._applies_to_all_own_digimon = True

        def condition3(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Base' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
